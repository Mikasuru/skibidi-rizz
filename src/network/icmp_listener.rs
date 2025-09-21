use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

#[cfg(feature = "pnet_datalink")]
use pnet::datalink::{self, Channel::Ethernet};
#[cfg(feature = "pnet_datalink")]
use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
#[cfg(feature = "pnet_datalink")]
use pnet::packet::Packet;
#[cfg(feature = "pnet_datalink")]
use pnet::packet::ipv4::Ipv4Packet;
#[cfg(feature = "pnet_datalink")]
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
#[cfg(feature = "pnet_datalink")]
use pnet::packet::udp::UdpPacket;

pub struct IcmpListener {
    active_probes: Arc<Mutex<HashMap<(Ipv4Addr, u16), Instant>>>,
}

impl IcmpListener {
    pub fn new() -> Self {
        Self {
            active_probes: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[cfg(feature = "pnet_datalink")]
  pub async fn start(&self, interface_name: Option<&str>) -> Result<mpsc::Receiver<(Ipv4Addr, u16)>, String> {
        let (tx, rx) = mpsc::channel(100);
        let probes = self.active_probes.clone();

        // start listener in a separate thread
        let interface_name_owned = if let Some(name) = interface_name {
            name.to_string()
        } else {
            let interfaces = datalink::interfaces();
            let default_name = interfaces
                .iter()
                .find(|iface| !iface.is_loopback() && iface.is_up())
                .map(|iface| iface.name.clone())
                .unwrap_or_else(|| "eth0".to_string());
            default_name
        };

        std::thread::spawn(move || {
            let interface_name = interface_name_owned;

            for interface in datalink::interfaces() {
                if interface.name == interface_name {
                    match datalink::channel(&interface, Default::default()) {
                        Ok(Ethernet(_txrx, mut rx)) => {
                            let _buffer = vec![0u8; 65536];

                            loop {
                                match rx.next() {
                                    Ok(packet) => {
                                        if let Some(eth_packet) = EthernetPacket::new(packet) {
                                            if eth_packet.get_ethertype() == EtherTypes::Ipv4 {
                                                if let Some(ip_packet) = Ipv4Packet::new(eth_packet.payload()) {
                                                    if ip_packet.get_next_level_protocol() == pnet::packet::ip::IpNextHeaderProtocols::Icmp {
                                                        if let Some(icmp_packet) = IcmpPacket::new(ip_packet.payload()) {
                                                            if icmp_packet.get_icmp_type() == IcmpTypes::DestinationUnreachable
                        || icmp_packet.get_icmp_type() == IcmpTypes::TimeExceeded
                        || icmp_packet.get_icmp_type() == IcmpTypes::ParameterProblem {
                                                                let icmp_payload = icmp_packet.payload();
                                                                if icmp_payload.len() > 28 {
                                                                    // skip IP header (20 bytes) and get UDP header
                                                                    if let Some(original_udp) = UdpPacket::new(&icmp_payload[20..]) {
                                                                        let original_dst_ip = ip_packet.get_destination();
                                                                        let original_dst_port = original_udp.get_destination();
                                                                        let mut probes = probes.lock().unwrap();
                                                                        if probes.contains_key(&(original_dst_ip, original_dst_port)) {
                                                                            let _ = tx.blocking_send((original_dst_ip, original_dst_port));
                                                                            probes.remove(&(original_dst_ip, original_dst_port));
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error receiving packet: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                        Ok(_) => return Err("Not an Ethernet interface".to_string()) as Result<mpsc::Receiver<(Ipv4Addr, u16)>, String>,
                        Err(e) => return Err(format!("Error creating channel: {}", e)),
                    }
                }
            }

            Err("Interface not found".to_string())
        });

        Ok(rx)
    }

    #[cfg(not(feature = "pnet_datalink"))]
    pub async fn start(&self, _interface_name: Option<&str>) -> Result<mpsc::Receiver<(Ipv4Addr, u16)>, String> {
        Err("ICMP detection requires the pnet_datalink feature. Build with: cargo build --features pnet_datalink".to_string())
    }

    pub fn register_probe(&self, src_ip: Ipv4Addr, src_port: u16) {
        let mut probes = self.active_probes.lock().unwrap();
        probes.insert((src_ip, src_port), Instant::now());
    }

    pub fn cleanup_old_probes(&self, timeout: Duration) {
        let mut probes = self.active_probes.lock().unwrap();
        probes.retain(|_, time| time.elapsed() < timeout);
    }
}