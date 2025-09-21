use std::net::{SocketAddr, ToSocketAddrs, Ipv4Addr};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::sync::Semaphore;
use crate::network::icmp_listener::IcmpListener;

pub struct PortScanner;

impl PortScanner {
    pub async fn scan_ports(target: &str, ports: &[u16], timeout_ms: u64) -> Vec<u16> {
        let mut open_ports = Vec::new();

        // Set up ICMP listener
        let icmp_listener = IcmpListener::new();
        let mut icmp_rx = match icmp_listener.start(Some("eth0")).await {
            Ok(rx) => rx,
            Err(e) => {
                eprintln!("Error: ICMP listener failed: {}", e);
                eprintln!("Port scanning requires ICMP detection. Please ensure you have proper permissions.");
                eprintln!("On Linux, run with sudo. On Windows, install Npcap.");
                return open_ports;
            }
        };

        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return open_ports,
        };

        let local_addr = socket.local_addr().unwrap();
        let _local_ip = match local_addr.ip() {
            std::net::IpAddr::V4(ip) => ip,
            _ => Ipv4Addr::new(127, 0, 0, 1),
        };
        let _local_port = local_addr.port();
        let mut ports_with_errors = std::collections::HashSet::new();
        let target_ip = match target.parse::<Ipv4Addr>() {
            Ok(ip) => ip,
            Err(_) => return open_ports,
        };

        for &port in ports {
            let target_addr = format!("{}:{}", target, port);
            let probe_data = b"PORT_SCAN";

            icmp_listener.register_probe(target_ip, port);

            if socket.send_to(probe_data, &target_addr).await.is_ok() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        // Listen for ICMP errors for the timeout period
        {
            let start_time = std::time::Instant::now();
            let timeout = Duration::from_millis(timeout_ms);

            while start_time.elapsed() < timeout {
                if let Ok((_target_ip, port)) = icmp_rx.try_recv() {
                    ports_with_errors.insert(port);
                }

                // Cleanup old probes
                icmp_listener.cleanup_old_probes(timeout);

                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        }

        for &port in ports {
            if !ports_with_errors.contains(&port) {
                open_ports.push(port);
            }
        }

        open_ports
    }

    pub async fn quick_scan(target: &str) -> Vec<u16> {
        let common_ports = &[
            21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 161, 194, 443, 993, 995, 1433, 1521,
            3306, 3389, 5432, 5900, 6379, 8080, 8443, 8888, 9200, 27017,
        ];

        Self::scan_ports(target, common_ports, 500).await
    }
}

pub struct SocketPool {
    sockets: Vec<Arc<UdpSocket>>,
    current_index: AtomicUsize,
    semaphore: Arc<Semaphore>,
    target_cache: std::collections::HashMap<String, SocketAddr>,
    last_cleanup: std::time::Instant,
}

impl SocketPool {
    pub async fn get_socket(&self) -> Option<Arc<UdpSocket>> {
        let _permit = self.semaphore.acquire().await.ok()?;

        if self.sockets.is_empty() {
            return None;
        }

        let index = self.current_index.load(Ordering::Relaxed);
        let socket = self.sockets[index].clone();
        self.current_index
            .store((index + 1) % self.sockets.len(), Ordering::Relaxed);
        Some(socket)
    }

    pub fn is_empty(&self) -> bool {
        self.sockets.is_empty()
    }

    pub async fn new_optimized(size: usize) -> Self {
        let mut sockets = Vec::with_capacity(size);
        let mut port = 20000;

        for _i in 0..size {
            let socket = match UdpSocket::bind(format!("0.0.0.0:{}", port)).await {
                Ok(socket) => {
                    port += 1;
                    socket
                }
                Err(_) => {
                    match UdpSocket::bind("0.0.0.0:0").await {
                        Ok(socket) => socket,
                        Err(_) => continue, // skip if can't bind
                    }
                }
            };

            if let Err(e) = socket.set_broadcast(true) {
                eprintln!("Failed to set broadcast: {}", e);
            }

            sockets.push(Arc::new(socket));
        }

        Self {
            sockets,
            current_index: AtomicUsize::new(0),
            semaphore: Arc::new(Semaphore::new(size.max(1))),
            target_cache: std::collections::HashMap::new(),
            last_cleanup: std::time::Instant::now(),
        }
    }

    pub async fn get_target_address(&mut self, target: &str, port: u16) -> Option<SocketAddr> {
        let cache_key = format!("{}:{}", target, port);
        if self.last_cleanup.elapsed() > Duration::from_secs(300) {
            self.target_cache.clear();
            self.last_cleanup = std::time::Instant::now();
        }

        if let Some(addr) = self.target_cache.get(&cache_key) {
            return Some(*addr);
        }

        let addrs = format!("{}:{}", target, port)
            .to_socket_addrs()
            .ok()?
            .next()?;

        self.target_cache.insert(cache_key, addrs);
        Some(addrs)
    }

    pub async fn send_batch(
        &self,
        socket: &Arc<UdpSocket>,
        target: SocketAddr,
        packets: &[&[u8]],
    ) -> Result<usize, std::io::Error> {
        let mut total_sent = 0;

        for packet in packets {
            match socket.send_to(packet, target).await {
                Ok(sent) => total_sent += sent,
                Err(e) => return Err(e),
            }
        }

        Ok(total_sent)
    }
}
