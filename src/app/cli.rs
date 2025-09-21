use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "skibidi-rizz",
    about = "Plugins for kurinium",
    version,
    author
)]
pub struct Args {
    // Target IP address or hostname
    #[arg(short, long, value_name = "IP/HOSTNAME")]
    pub target: Option<String>,

    // Target port
    #[arg(short, long, default_value_t = 80, value_name = "PORT")]
    pub port: u16,

    // ATK mode (flood, amplification, fragmentation, slowloris, burst, tcp, tcpconnect, http, udp, portscan, dnsquery, dnsflood, ssdp, cache, spoofed)
    #[arg(short, long, default_value = "flood", value_name = "MODE")]
    pub mode: String,

    // Number of threads to use
    #[arg(short = 'T', long, default_value_t = 4, value_name = "THREADS")]
    pub threads: usize,

    // Packets per second rate
    #[arg(short = 'r', long, default_value_t = 1000, value_name = "PPS")]
    pub rate: u64,

    // Duration in seconds
    #[arg(short, long, default_value_t = 60, value_name = "SECONDS")]
    pub duration: u64,

    // Packet size in bytes
    #[arg(short = 's', long, default_value_t = 1024, value_name = "BYTES")]
    pub packet_size: usize,

    // Custom payload (hex or string)
    #[arg(long, value_name = "PAYLOAD")]
    pub payload: Option<String>,

    // Use random payload
    #[arg(long, default_value_t = false)]
    pub random_payload: bool,

    // Use random source ports
    #[arg(long, default_value_t = false)]
    pub random_ports: bool,

    // Evasion mode (fixed, random, adaptive, exponential)
    #[arg(long, default_value = "fixed")]
    pub evasion: String,

    // Configuration file to load
    #[arg(short, long, value_name = "FILE")]
    pub config: Option<PathBuf>,

    // Save configuration to file
    #[arg(long, value_name = "FILE")]
    pub save_config: Option<PathBuf>,

    // Launch attack immediately without TUI
    #[arg(long, default_value_t = false)]
    pub no_tui: bool,

    // Enable verbose output
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    // Preset configuration (basic, anti-ddos, amplification, stealth, multi-vector, high-throughput)
    #[arg(long, value_name = "PRESET")]
    pub preset: Option<String>,

    // Secondary attack mode
    #[arg(long, value_name = "MODE")]
    pub secondary_attack: Option<String>,

    // Variance percentage for timing jitter (0-100)
    #[arg(long, default_value_t = 0, value_name = "PERCENT")]
    pub variance: u8,

    // Burst size for burst mode
    #[arg(long, default_value_t = 10, value_name = "COUNT")]
    pub burst_size: u32,

    // Enable Discord RPC
    #[arg(long, default_value_t = true)]
    pub discord_rpc: bool,

    // Theme name (tokyo-night, dracula, gruvbox, solarized, monokai, nord)
    #[arg(long, default_value = "tokyo-night")]
    pub theme: String,
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        // Validate target
        if self.target.is_none() && self.no_tui {
            return Err("Target is required when --no-tui is specified".to_string());
        }

        // Validate port range
        if self.port == 0 {
            return Err("Port must be between 1 and 65535".to_string());
        }

        // Validate packet size
        if self.packet_size < 1 || self.packet_size > 65507 {
            return Err("Packet size must be between 1 and 65507 bytes".to_string());
        }

        // Validate thread count
        if self.threads == 0 || self.threads > 1024 {
            return Err("Thread count must be between 1 and 1024".to_string());
        }

        // Validate duration
        if self.duration == 0 || self.duration > 86400 {
            return Err("Duration must be between 1 and 86400 seconds (24 hours)".to_string());
        }

        // Validate rate
        if self.rate == 0 || self.rate > 1_000_000 {
            return Err("Rate must be between 1 and 1,000,000 PPS".to_string());
        }

        // Validate variance
        if self.variance > 100 {
            return Err("Variance percentage must be between 0 and 100".to_string());
        }

        // Validate attack modes
        let valid_modes = ["flood", "amplification", "fragmentation", "slowloris", "burst",
                           "tcp", "tcpconnect", "http", "udp", "portscan", "dnsquery",
                           "dnsflood"];
        if !valid_modes.contains(&self.mode.as_str()) {
            return Err(format!(
                "Invalid attack mode. Valid modes: {}",
                valid_modes.join(", ")
            ));
        }

        // Validate evasion modes
        let valid_evasion = ["fixed", "random", "adaptive", "exponential"];
        if !valid_evasion.contains(&self.evasion.as_str()) {
            return Err(format!(
                "Invalid evasion mode. Valid modes: {}",
                valid_evasion.join(", ")
            ));
        }

        // Validate themes
        let valid_themes = [
            "tokyo-night",
            "dracula",
            "gruvbox",
            "solarized",
            "monokai",
            "nord",
        ];
        if !valid_themes.contains(&self.theme.as_str()) {
            return Err(format!(
                "Invalid theme. Valid themes: {}",
                valid_themes.join(", ")
            ));
        }

        Ok(())
    }
}