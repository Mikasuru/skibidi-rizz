use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::time::Instant;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AtkMode {
    Flood,
    Amplification,
    Fragmentation,
    Slowloris,
    Burst,
    DNSQuery,
    PortScan,
    UDP,
    TCP,
    TCPConnect,
    HTTP,
    DNSFlood,
}

impl AtkMode {
    pub fn to_string(&self) -> &'static str {
        match self {
            AtkMode::Flood => "Flood",
            AtkMode::Amplification => "Amplification",
            AtkMode::Fragmentation => "Fragmentation",
            AtkMode::Slowloris => "Slowloris",
            AtkMode::Burst => "Burst",
            AtkMode::DNSQuery => "DNS Query",
            AtkMode::PortScan => "Port Scan",
            AtkMode::UDP => "UDP",
            AtkMode::TCP => "TCP",
            AtkMode::TCPConnect => "TCP Connect",
            AtkMode::HTTP => "HTTP",
            AtkMode::DNSFlood => "DNS Flood",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            AtkMode::Flood => "UDP flood attack",
            AtkMode::Amplification => "DNS amplification attack",
            AtkMode::Fragmentation => "IP fragmentation attack",
            AtkMode::Slowloris => "Slow HTTP attack",
            AtkMode::Burst => "Burst traffic pattern",
            AtkMode::DNSQuery => "DNS query attack",
            AtkMode::PortScan => "TCP/UDP port scanning with service detection",
            AtkMode::UDP => "UDP protocol attack",
            AtkMode::TCP => "TCP protocol attack",
            AtkMode::TCPConnect => "TCP connect scan",
            AtkMode::HTTP => "HTTP flood attack",
            AtkMode::DNSFlood => "DNS flood attack",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum EvasMode {
    Fixed,
    Random,
    Adaptive,
    Exponential,
    Burst,
}

impl EvasMode {
    pub fn to_string(&self) -> &'static str {
        match self {
            EvasMode::Fixed => "Fixed",
            EvasMode::Random => "Random",
            EvasMode::Adaptive => "Adaptive",
            EvasMode::Exponential => "Exponential",
            EvasMode::Burst => "Burst",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SizeStrategy {
    Fixed,
    Random,
    Oscillating,
}

impl std::fmt::Display for SizeStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SizeStrategy::Fixed => write!(f, "Fixed"),
            SizeStrategy::Random => write!(f, "Random"),
            SizeStrategy::Oscillating => write!(f, "Oscillating"),
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct AtkConfig {
    pub target: String,
    pub port: u16,
    pub threads: usize,
    pub rate: u64,
    pub duration: u64,
    pub packet_size: usize,
    pub mode: AtkMode,
    pub custom_payload: String,
    pub random_payload: bool,
    pub random_ports: bool,
    pub evasion_mode: EvasMode,
    pub size_strategy: SizeStrategy,
    pub secondary_attack: bool,
    pub variance_percentage: u8,
    pub burst_size: u32,
    pub rotate_user_agent: bool,
    pub user_agents: Vec<String>,
    pub interface: Option<String>,
}

impl Default for AtkConfig {
    fn default() -> Self {
        Self {
            target: "192.168.1.1".to_string(),
            port: 17091,
            threads: 5,
            rate: 1000,
            duration: 60,
            packet_size: 512,
            mode: AtkMode::Flood,
            custom_payload: String::new(),
            random_payload: false,
            random_ports: false,
            evasion_mode: EvasMode::Random,
            size_strategy: SizeStrategy::Oscillating,
            secondary_attack: false,
            variance_percentage: 25,
            burst_size: 10,
            rotate_user_agent: false,
            user_agents: vec![
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:123.0) Gecko/20100101 Firefox/123.0".to_string(),
                "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".to_string(),
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36".to_string(),
            ],
            interface: None,
        }
    }
}

#[derive(Default)]
pub struct AtkStats {
    pub packets_sent: AtomicU64,
    pub bytes_sent: AtomicU64,
    pub missed_pkgs: AtomicU64,
    pub peak_bandwidth: AtomicU64,
    pub last_bytes_count: AtomicU64,
    pub last_bandwidth_update: AtomicU64,
    pub start_time: Option<Instant>,
    pub is_running: AtomicBool,
    pub pps_history: Arc<Mutex<VecDeque<u64>>>,
    pub bandwidth_history: Arc<Mutex<VecDeque<f64>>>,
    pub target_status: Arc<Mutex<TargetStatus>>,
    pub packet_capture: Arc<Mutex<VecDeque<PacketInfo>>>,
    pub network_activity: Arc<Mutex<VecDeque<(Instant, u64)>>> , // timestamp, bytes
    pub auto_stop_condition: AutoStopCondition,
}

#[derive(Debug, Clone)]
pub struct PacketInfo {
    pub timestamp: Instant,
    pub target: String,
    pub port: u16,
    pub size: usize,
    pub protocol: String,
    pub success: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AutoStopCondition {
    None,
}

impl Default for AutoStopCondition {
    fn default() -> Self {
        AutoStopCondition::None
    }
}

#[derive(Default, Debug, Clone)]
pub struct TargetStatus {
    pub is_online: bool,
    pub response_time_ms: f64,
    pub last_checked: Option<Instant>,
    pub open_ports: Vec<u16>,
    pub is_degraded: bool,
    pub baseline_response: f64,
    // Lookup information
    pub resolved_ip: Option<String>,
    pub country: Option<String>,
    pub city: Option<String>,
}

impl Clone for AtkStats {
    fn clone(&self) -> Self {
        Self {
            packets_sent: AtomicU64::new(
                self.packets_sent.load(std::sync::atomic::Ordering::Relaxed),
            ),
            bytes_sent: AtomicU64::new(self.bytes_sent.load(std::sync::atomic::Ordering::Relaxed)),
            missed_pkgs: AtomicU64::new(
                self.missed_pkgs
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            peak_bandwidth: AtomicU64::new(
                self.peak_bandwidth
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            last_bytes_count: AtomicU64::new(
                self.last_bytes_count
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            last_bandwidth_update: AtomicU64::new(
                self.last_bandwidth_update
                    .load(std::sync::atomic::Ordering::Relaxed),
            ),
            start_time: self.start_time,
            is_running: AtomicBool::new(self.is_running.load(std::sync::atomic::Ordering::Relaxed)),
            pps_history: Arc::clone(&self.pps_history),
            bandwidth_history: Arc::clone(&self.bandwidth_history),
            target_status: Arc::clone(&self.target_status),
            packet_capture: Arc::clone(&self.packet_capture),
            network_activity: Arc::clone(&self.network_activity),
            auto_stop_condition: self.auto_stop_condition.clone(),
        }
    }
}

impl AtkStats {
    pub fn new() -> Self {
        Self {
            packets_sent: AtomicU64::new(0),
            bytes_sent: AtomicU64::new(0),
            missed_pkgs: AtomicU64::new(0),
            peak_bandwidth: AtomicU64::new(0),
            last_bytes_count: AtomicU64::new(0),
            last_bandwidth_update: AtomicU64::new(0),
            start_time: None,
            is_running: AtomicBool::new(false),
            pps_history: Arc::new(Mutex::new(VecDeque::new())),
            bandwidth_history: Arc::new(Mutex::new(VecDeque::new())),
            target_status: Arc::new(Mutex::new(TargetStatus::default())),
            packet_capture: Arc::new(Mutex::new(VecDeque::new())),
            network_activity: Arc::new(Mutex::new(VecDeque::new())),
            auto_stop_condition: AutoStopCondition::None,
        }
    }

    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
        self.is_running.store(true, Ordering::Relaxed);
    }

    pub fn stop(&self) {
        self.is_running.store(false, Ordering::Relaxed);
    }
}

impl AtkStats {
    pub fn upd_target_status(this: &Arc<Self>, target: &str, port: u16) {
        let mut status = this.target_status.lock().unwrap();

        // target availability
        if let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") {
            socket
                .set_read_timeout(Some(std::time::Duration::from_millis(1000)))
                .ok();

            let start = std::time::Instant::now();
            let probe_data = b"PROBE";

            match socket.send_to(probe_data, format!("{}:{}", target, port)) {
                Ok(_) => {
                    let mut buf = [0u8; 1024];
                    match socket.recv_from(&mut buf) {
                        Ok(_) => {
                            status.response_time_ms = start.elapsed().as_millis() as f64;
                            status.is_online = true;

                            // set baseline on first check
                            if status.baseline_response == 0.0 {
                                status.baseline_response = status.response_time_ms;
                            }
                            status.is_degraded =
                                status.response_time_ms > status.baseline_response * 2.0;
                        }
                        Err(_) => {
                            status.is_online = false;
                        }
                    }
                }
                Err(_) => {
                    status.is_online = false;
                }
            }

            status.last_checked = Some(tokio::time::Instant::now());
        }
    }

    pub fn add_packet(&self, bytes: u64) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes, Ordering::Relaxed);
        self.add_network_activity(bytes);
    }

    pub fn add_failed(&self) {
        self.missed_pkgs.fetch_add(1, Ordering::Relaxed);
    }

    pub fn update_history(&self, pps: u64, bandwidth_mbps: f64) {
        {
            let mut pps_hist = self.pps_history.lock().unwrap();
            pps_hist.push_back(pps);
            if pps_hist.len() > 60 {
                pps_hist.pop_front();
            }
        }
        {
            let mut bw_hist = self.bandwidth_history.lock().unwrap();
            bw_hist.push_back(bandwidth_mbps);
            if bw_hist.len() > 60 {
                bw_hist.pop_front();
            }
        }
    }

    pub fn get_elapsed(&self) -> f64 {
        if let Some(start) = self.start_time {
            start.elapsed().as_secs_f64()
        } else {
            0.0
        }
    }

    pub fn update_bandwidth(&self, _bytes_delta: u64) {
        let current_time_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;

        let last_update_ms = self.last_bandwidth_update.load(Ordering::Relaxed);
        let last_bytes = self.last_bytes_count.load(Ordering::Relaxed);
        let current_bytes = self.bytes_sent.load(Ordering::Relaxed);

        if last_update_ms > 0 && current_time_ms > last_update_ms {
            let time_diff_ms = current_time_ms - last_update_ms;
            let bytes_diff = current_bytes.saturating_sub(last_bytes);

            if time_diff_ms > 0 {
                let bandwidth_bps = (bytes_diff as f64 * 8_000.0) / time_diff_ms as f64;
                let bandwidth_bps_u64 = bandwidth_bps as u64;

                let peak_bps = self.peak_bandwidth.load(Ordering::Relaxed);
                if bandwidth_bps_u64 > peak_bps {
                    self.peak_bandwidth
                        .store(bandwidth_bps_u64, Ordering::Relaxed);
                }
            }
        }

        self.last_bytes_count
            .store(current_bytes, Ordering::Relaxed);
        self.last_bandwidth_update
            .store(current_time_ms, Ordering::Relaxed);
    }

    pub fn get_peak_bandwidth(&self) -> f64 {
        self.peak_bandwidth.load(Ordering::Relaxed) as f64 / 1_000_000.0
    }

    pub fn get_packet_capture(&self) -> Vec<PacketInfo> {
        let capture = self.packet_capture.lock().unwrap();
        capture.iter().cloned().collect()
    }

    pub fn add_network_activity(&self, bytes: u64) {
        let mut activity = self.network_activity.lock().unwrap();
        activity.push_back((Instant::now(), bytes));
        while activity.len() > 300 {  // 5 samples per second for 60 seconds
            activity.pop_front();
        }
    }

    pub fn get_network_activity(&self) -> Vec<(f64, u64)> {
        let activity = self.network_activity.lock().unwrap();
        let now = Instant::now();
        activity
            .iter()
            .map(|(time, bytes)| {
                let seconds_ago = now.duration_since(*time).as_secs_f64();
                (-seconds_ago, *bytes)
            })
            .collect()
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum AppState {
    Config,
    Attack,
    Results,
}

#[derive(PartialEq, Clone, Copy)]
pub enum ConfigField {
    Target,
    Port,
    Threads,
    Rate,
    Duration,
    PacketSize,
    Mode,
    CustomPayload,
    RandomPayload,
    RandomPorts,
    EvasMode,
    SizeStrategy,
    SecondaryAttack,
    VariancePercentage,
    BurstSize,
    RotateUserAgent,
    Preset,
    Theme,
    RpcEnabled,
    AutoSave,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AttackPreset {
    Basic,
    AntiDDoS,
    Amplification,
    Stealth,
    MultiVector,
    HighThroughput,
    Custom,
}

impl AttackPreset {
    pub fn get_config(&self, target: &str, port: u16) -> AtkConfig {
        match self {
            AttackPreset::Basic => AtkConfig {
                target: target.to_string(),
                port,
                threads: 50,
                rate: 10000,
                duration: 30,
                packet_size: 1024,
                mode: AtkMode::Flood,
                custom_payload: String::new(),
                random_payload: false,
                random_ports: false,
                evasion_mode: EvasMode::Fixed,
                size_strategy: SizeStrategy::Fixed,
                secondary_attack: false,
                variance_percentage: 20,
                burst_size: 10,
                rotate_user_agent: false,
                user_agents: vec![],
                interface: None,
            },
            AttackPreset::AntiDDoS => AtkConfig {
                target: target.to_string(),
                port,
                threads: 150,
                rate: 75000,
                duration: 60,
                packet_size: 1400,
                mode: AtkMode::Amplification,
                custom_payload: String::new(),
                random_payload: true,
                random_ports: true,
                evasion_mode: EvasMode::Random,
                size_strategy: SizeStrategy::Oscillating,
                secondary_attack: true,
                variance_percentage: 50,
                burst_size: 50,
                rotate_user_agent: false,
                user_agents: vec![],
                interface: None,
            },
            AttackPreset::Amplification => AtkConfig {
                target: target.to_string(),
                port,
                threads: 100,
                rate: 50000,
                duration: 45,
                packet_size: 512,
                mode: AtkMode::Amplification,
                custom_payload: String::new(),
                random_payload: false,
                random_ports: false,
                evasion_mode: EvasMode::Random,
                size_strategy: SizeStrategy::Fixed,
                secondary_attack: false,
                variance_percentage: 30,
                burst_size: 25,
                rotate_user_agent: false,
                user_agents: vec![],
                interface: None,
            },
            AttackPreset::Stealth => AtkConfig {
                target: target.to_string(),
                port,
                threads: 20,
                rate: 5000,
                duration: 120,
                packet_size: 64,
                mode: AtkMode::Slowloris,
                custom_payload: String::new(),
                random_payload: true,
                random_ports: true,
                evasion_mode: EvasMode::Adaptive,
                size_strategy: SizeStrategy::Random,
                secondary_attack: false,
                variance_percentage: 80,
                burst_size: 5,
                rotate_user_agent: false,
                user_agents: vec![],
                interface: None,
            },
            AttackPreset::MultiVector => AtkConfig {
                target: target.to_string(),
                port,
                threads: 200,
                rate: 100000,
                duration: 90,
                packet_size: 1024,
                mode: AtkMode::Amplification,
                custom_payload: String::new(),
                random_payload: true,
                random_ports: true,
                evasion_mode: EvasMode::Exponential,
                size_strategy: SizeStrategy::Oscillating,
                secondary_attack: true,
                variance_percentage: 70,
                burst_size: 100,
                rotate_user_agent: false,
                user_agents: vec![],
                interface: None,
            },
            AttackPreset::HighThroughput => AtkConfig {
                target: target.to_string(),
                port,
                threads: 400,
                rate: 300000,
                duration: 120,
                packet_size: 1472, // Maximum UDP packet size
                mode: AtkMode::Amplification,
                custom_payload: String::new(),
                random_payload: false,
                random_ports: true, // Use random ports to avoid filtering
                evasion_mode: EvasMode::Random,
                size_strategy: SizeStrategy::Fixed,
                secondary_attack: true,
                variance_percentage: 10,
                burst_size: 500,
                rotate_user_agent: true, // Enable UA rotation for HTTP attacks
                user_agents: vec![
                    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36".to_string(),
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36"
                        .to_string(),
                    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36".to_string(),
                ],
                interface: None,
            },
            AttackPreset::Custom => AtkConfig::default(),
        }
    }
}

impl std::fmt::Display for AttackPreset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AppTheme {
    TokyoNight,
    Dracula,
    Gruvbox,
    Solarized,
    Monokai,
    Nord,
}

impl AppTheme {
    pub fn to_string(&self) -> &'static str {
        match self {
            AppTheme::TokyoNight => "Tokyo Night",
            AppTheme::Dracula => "Dracula",
            AppTheme::Gruvbox => "Gruvbox",
            AppTheme::Solarized => "Solarized",
            AppTheme::Monokai => "Monokai",
            AppTheme::Nord => "Nord",
        }
    }
}
