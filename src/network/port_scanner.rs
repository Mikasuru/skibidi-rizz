use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UdpSocket};
use tokio::time::timeout;
use crate::network::icmp_listener::IcmpListener;

#[derive(Debug, Clone)]
pub struct PortInfo {
    pub port: u16,
    pub protocol: String,
    pub state: PortState,
    pub service: Option<String>,
    pub banner: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PortState {
    Open,
    Closed,
    Filtered,
    Unfiltered,
    OpenFiltered,
}

pub struct EnhancedPortScanner {
    common_ports: Vec<(u16, &'static str)>,
    service_probes: HashMap<u16, Vec<u8>>,
}

impl EnhancedPortScanner {
    pub fn new() -> Self {
        let mut scanner = Self {
            common_ports: vec![
                (21, "FTP"), (22, "SSH"), (23, "Telnet"), (25, "SMTP"), (53, "DNS"),
                (80, "HTTP"), (110, "POP3"), (135, "RPC"), (139, "NetBIOS"), (143, "IMAP"),
                (161, "SNMP"), (194, "IRC"), (443, "HTTPS"), (993, "IMAPS"), (995, "POP3S"),
                (1433, "MSSQL"), (1521, "Oracle"), (3306, "MySQL"), (3389, "RDP"),
                (5432, "PostgreSQL"), (5900, "VNC"), (6379, "Redis"), (8080, "HTTP-Alt"),
                (8443, "HTTPS-Alt"), (8888, "HTTP-Alt"), (9200, "Elasticsearch"), (27017, "MongoDB"),
                (445, "SMB"), (992, "TelnetS"), (1723, "PPTP"), (3128, "Squid"),
                (8000, "HTTP-Alt"), (8801, "HTTP-Alt"), (10000, "Webmin"),
            ],
            service_probes: HashMap::new(),
        };

        scanner.initialize_service_probes();
        scanner
    }

    fn initialize_service_probes(&mut self) {
        // HTTP/HTTPS probes
        self.service_probes.insert(80, b"GET / HTTP/1.0\r\n\r\n".to_vec());
        self.service_probes.insert(8080, b"GET / HTTP/1.0\r\n\r\n".to_vec());
        self.service_probes.insert(8000, b"GET / HTTP/1.0\r\n\r\n".to_vec());
        self.service_probes.insert(8888, b"GET / HTTP/1.0\r\n\r\n".to_vec());

        // SSH probe
        self.service_probes.insert(22, b"SSH-2.0-Scanner\r\n".to_vec());

        // FTP probe
        self.service_probes.insert(21, b"USER anonymous\r\n".to_vec());

        // SMTP probe
        self.service_probes.insert(25, b"EHLO scanner\r\n".to_vec());

        // POP3 probe
        self.service_probes.insert(110, b"CAPA\r\n".to_vec());

        // MySQL probe
        self.service_probes.insert(3306, vec![0x20, 0x00, 0x00, 0x01, 0x85, 0xa6, 0x3f, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        // Redis probe
        self.service_probes.insert(6379, b"PING\r\n".to_vec());

        // MongoDB probe
        self.service_probes.insert(27017, vec![0x3f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xd4, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    pub async fn scan_ports(
        &self,
        target: &str,
        ports: &[u16],
        scan_tcp: bool,
        scan_udp: bool,
        timeout_ms: u64,
        detect_services: bool,
    ) -> Vec<PortInfo> {
        let mut results = Vec::new();
        let _target_ip = match target.parse::<Ipv4Addr>() {
            Ok(ip) => ip,
            Err(_) => return results,
        };

        if scan_tcp {
            let tcp_results = self.scan_tcp_ports(target, ports, timeout_ms, detect_services).await;
            results.extend(tcp_results);
        }

        if scan_udp {
            let udp_results = self.scan_udp_ports(target, ports, timeout_ms).await;
            results.extend(udp_results);
        }

        results
    }

    async fn scan_tcp_ports(
        &self,
        target: &str,
        ports: &[u16],
        timeout_ms: u64,
        detect_services: bool,
    ) -> Vec<PortInfo> {
        let mut results = Vec::new();
        let mut tasks = Vec::new();

        for &port in ports {
            let target_str = target.to_string();
            let service_map = self.get_service_info(port);
            let detect = detect_services;

            tasks.push(tokio::spawn(async move {
                Self::scan_tcp_port(&target_str, port, timeout_ms, detect, service_map).await
            }));
        }

        for task in tasks {
            if let Ok(result) = task.await {
                results.push(result);
            }
        }

        results
    }

    async fn scan_tcp_port(
        target: &str,
        port: u16,
        timeout_ms: u64,
        detect_services: bool,
        service_info: Option<&'static str>,
    ) -> PortInfo {
        let target_addr = format!("{}:{}", target, port);

        match timeout(Duration::from_millis(timeout_ms), TcpStream::connect(&target_addr)).await {
            Ok(Ok(_stream)) => {
                let mut port_info = PortInfo {
                    port,
                    protocol: "TCP".to_string(),
                    state: PortState::Open,
                    service: service_info.map(|s| s.to_string()),
                    banner: None,
                };

                if detect_services {
                    if let Ok(mut stream) = TcpStream::connect(&target_addr).await {
                        let scanner = Self::new();
                        port_info.banner = Self::grab_banner(&mut stream, port, &scanner.service_probes).await;
                    }
                }

                port_info
            }
            Ok(Err(_)) => PortInfo {
                port,
                protocol: "TCP".to_string(),
                state: PortState::Closed,
                service: service_info.map(|s| s.to_string()),
                banner: None,
            },
            Err(_) => PortInfo {
                port,
                protocol: "TCP".to_string(),
                state: PortState::Filtered,
                service: service_info.map(|s| s.to_string()),
                banner: None,
            },
        }
    }

    async fn scan_udp_ports(
        &self,
        target: &str,
        ports: &[u16],
        timeout_ms: u64,
    ) -> Vec<PortInfo> {
        let mut results = Vec::new();

        // Set up ICMP listener for UDP scanning
        let icmp_listener = IcmpListener::new();
        let mut icmp_rx = match icmp_listener.start(Some("eth0")).await {
            Ok(rx) => rx,
            Err(_) => {
                // If ICMP fails, we can't reliably detect closed UDP ports
                return ports.iter().map(|&port| {
                    let service_info = self.get_service_info(port);
                    PortInfo {
                        port,
                        protocol: "UDP".to_string(),
                        state: PortState::OpenFiltered,
                        service: service_info.map(|s| s.to_string()),
                        banner: None,
                    }
                }).collect();
            }
        };

        let target_ip = match target.parse::<Ipv4Addr>() {
            Ok(ip) => ip,
            Err(_) => return results,
        };

        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(_) => return results,
        };

        // Send UDP probes
        for &port in ports {
            let target_addr = format!("{}:{}", target, port);
            let probe_data = self.get_udp_probe(port);

            icmp_listener.register_probe(target_ip, port);

            if socket.send_to(&probe_data, &target_addr).await.is_ok() {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }

        // Listen for ICMP errors
        let mut ports_with_errors = std::collections::HashSet::new();
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_millis(timeout_ms);

        while start_time.elapsed() < timeout_duration {
            if let Ok((_target_ip, port)) = icmp_rx.try_recv() {
                ports_with_errors.insert(port);
            }

            icmp_listener.cleanup_old_probes(timeout_duration);
            tokio::time::sleep(Duration::from_millis(1)).await;
        }

        // Generate results
        for &port in ports {
            let service_info = self.get_service_info(port);
            let state = if ports_with_errors.contains(&port) {
                PortState::Closed
            } else {
                PortState::OpenFiltered
            };

            results.push(PortInfo {
                port,
                protocol: "UDP".to_string(),
                state,
                service: service_info.map(|s| s.to_string()),
                banner: None,
            });
        }

        results
    }

    async fn grab_banner(stream: &mut TcpStream, port: u16, probes: &HashMap<u16, Vec<u8>>) -> Option<String> {
        let probe = probes.get(&port)?;

        if let Err(_) = timeout(Duration::from_millis(1000), stream.write_all(probe)).await {
            return None;
        }

        let mut buffer = [0u8; 1024];
        match timeout(Duration::from_millis(1000), stream.read(&mut buffer)).await {
            Ok(Ok(n)) if n > 0 => {
                let banner = String::from_utf8_lossy(&buffer[..n]);
                Some(banner.lines().next().unwrap_or("").to_string())
            }
            _ => None,
        }
    }

    fn get_service_info(&self, port: u16) -> Option<&'static str> {
        self.common_ports
            .iter()
            .find(|&&(p, _)| p == port)
            .map(|(_, service)| *service)
    }

    fn get_udp_probe(&self, port: u16) -> Vec<u8> {
        match port {
            53 => b"\x00\x00\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00".to_vec(), // DNS query
            161 => b"\x30\x26\x02\x01\x00\x04\x06\x70\x75\x62\x6c\x69\x63\xa0\x19\x02\x04\x00\x00\x00\x00\x02\x01\x00\x02\x01\x00\x30\x0b\x30\x09\x06\x05\x2b\x06\x01\x02\x01\x05\x00".to_vec(), // SNMP
            _ => b"UDP_PROBE".to_vec(),
        }
    }

    pub async fn quick_scan(target: &str) -> Vec<PortInfo> {
        let scanner = Self::new();
        let common_ports: Vec<u16> = scanner.common_ports.iter().map(|&(p, _)| p).collect();
        scanner.scan_ports(target, &common_ports, true, false, 500, true).await
    }

    pub async fn comprehensive_scan(target: &str) -> Vec<PortInfo> {
        let scanner = Self::new();
        let common_ports: Vec<u16> = scanner.common_ports.iter().map(|&(p, _)| p).collect();
        scanner.scan_ports(target, &common_ports, true, true, 1000, true).await
    }
}