use crate::network::attack::start_atkworkers;
use crate::config::config::CONFIG_SECTIONS;
use crate::types::types::*;
use crate::ui::ui::{DirtyRegion, RENDER_CACHE};
use crate::ui::interface_selector::{InterfaceSelector, InterfaceEvent};
use crate::ui::tutorial::TutorialState;
use std::collections::VecDeque;
use std::fs;
use std::io;
use std::sync::{atomic::Ordering, Arc, Mutex};

pub struct App {
    pub state: AppState,
    pub config: AtkConfig,
    pub stats: AtkStats,
    pub stats_arc: Option<Arc<AtkStats>>,
    pub selected_field: ConfigField,
    pub mode_index: usize,
    pub theme_index: usize,
    pub input_mode: bool,
    pub input_buffer: String,
    pub logs: Arc<Mutex<VecDeque<String>>>,

    // Navigation state
    pub selected_section: usize,
    pub section_active: bool,

    // Settings
    pub rpc_enabled: bool,
    pub auto_save: bool,

    // Preset selection
    pub preset_index: usize, 
    pub selected_preset: Option<AttackPreset>,

    pub show_cheat_sheet: bool, // Cheat sheet modal state
    pub attack_handle: Option<tokio::task::JoinHandle<()>>, // Attack task handle

    // Interface selection
    pub interface_selector: Option<InterfaceSelector>,
    pub show_interface_selector: bool,
    pub selected_interface: Option<String>,

    // Tutorial
    pub tutorial: TutorialState,
    pub show_tutorial: bool,
}

impl App {
    pub fn new() -> App {
        App {
            state: AppState::Config,
            config: AtkConfig::default(),
            stats: AtkStats::new(),
            stats_arc: None,
            selected_field: ConfigField::Target,
            mode_index: 0,
            theme_index: 0,
            input_mode: false,
            input_buffer: String::new(),
            logs: Arc::new(Mutex::new(VecDeque::new())),
            // Navigation state
            selected_section: 0,
            section_active: false,

            // Preset modal state
            rpc_enabled: true,
            auto_save: false,

            // Preset selection
            preset_index: 0,
            selected_preset: None,

            // Cheat sheet modal state
            show_cheat_sheet: false,

            // Attack task handle
            attack_handle: None,

            // Interface selection
            interface_selector: None,
            show_interface_selector: false,
            selected_interface: None,

            // Tutorial
            tutorial: TutorialState::new(),
            show_tutorial: false,
        }
    }

    pub fn add_log(&self, message: String) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::Logs);
        });

        let mut logs = self.logs.lock().unwrap();
        logs.push_back(format!(
            "[{}] {}",
            chrono::Local::now().format("%H:%M:%S"),
            message
        ));
        if logs.len() > 100 {
            logs.pop_front();
        }
    }

    pub fn is_config_state(&self) -> bool {
        self.state == AppState::Config
    }

    pub fn is_attack_state(&self) -> bool {
        self.state == AppState::Attack
    }

    pub fn is_results_state(&self) -> bool {
        self.state == AppState::Results
    }

  
    pub fn next_field(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
            cache.mark_dirty(DirtyRegion::FieldHelp);
        });

        // if section is active, only nav within current section
        if self.section_active {
            let (_, _, fields) = CONFIG_SECTIONS[self.selected_section];
            let current_idx = fields.iter().position(|&f| f == self.selected_field).unwrap_or(0);
            let next_idx = (current_idx + 1) % fields.len();
            self.selected_field = fields[next_idx];

            // if this is a settings field and we're in settings section, handle it
            if self.selected_section == 4 {
                self.handle_settings_field();
            }
        } else {
            self.selected_field = match self.selected_field {
                ConfigField::Target => ConfigField::Port,
                ConfigField::Port => ConfigField::Threads,
                ConfigField::Threads => ConfigField::Rate,
                ConfigField::Rate => ConfigField::Duration,
                ConfigField::Duration => ConfigField::PacketSize,
                ConfigField::PacketSize => ConfigField::Mode,
                ConfigField::Mode => ConfigField::CustomPayload,
                ConfigField::CustomPayload => ConfigField::RandomPayload,
                ConfigField::RandomPayload => ConfigField::RandomPorts,
                ConfigField::RandomPorts => ConfigField::EvasMode,
                ConfigField::EvasMode => ConfigField::SizeStrategy,
                ConfigField::SizeStrategy => ConfigField::SecondaryAttack,
                ConfigField::SecondaryAttack => ConfigField::VariancePercentage,
                ConfigField::VariancePercentage => ConfigField::BurstSize,
                ConfigField::BurstSize => ConfigField::RotateUserAgent,
                ConfigField::RotateUserAgent => ConfigField::Preset,
                ConfigField::Preset => ConfigField::Theme,
                ConfigField::Theme => ConfigField::RpcEnabled,
                ConfigField::RpcEnabled => ConfigField::AutoSave,
                ConfigField::AutoSave => ConfigField::Target,
            };
        }
    }

    pub fn previous_field(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
            cache.mark_dirty(DirtyRegion::FieldHelp);
        });

        // if section is active, only nav within current section
        if self.section_active {
            let (_, _, fields) = CONFIG_SECTIONS[self.selected_section];
            let current_idx = fields.iter().position(|&f| f == self.selected_field).unwrap_or(0);
            let prev_idx = if current_idx == 0 {
                fields.len() - 1
            } else {
                current_idx - 1
            };
            self.selected_field = fields[prev_idx];

            // if this is a settings field and we're in settings section, handle it
            if self.selected_section == 4 {
                self.handle_settings_field();
            }
        } else {
            self.selected_field = match self.selected_field {
                ConfigField::Target => ConfigField::BurstSize,
                ConfigField::Port => ConfigField::Target,
                ConfigField::Threads => ConfigField::Port,
                ConfigField::Rate => ConfigField::Threads,
                ConfigField::Duration => ConfigField::Rate,
                ConfigField::PacketSize => ConfigField::Duration,
                ConfigField::Mode => ConfigField::PacketSize,
                ConfigField::CustomPayload => ConfigField::Mode,
                ConfigField::RandomPayload => ConfigField::CustomPayload,
                ConfigField::RandomPorts => ConfigField::RandomPayload,
                ConfigField::EvasMode => ConfigField::RandomPorts,
                ConfigField::SizeStrategy => ConfigField::EvasMode,
                ConfigField::SecondaryAttack => ConfigField::SizeStrategy,
                ConfigField::VariancePercentage => ConfigField::SecondaryAttack,
                ConfigField::BurstSize => ConfigField::VariancePercentage,
                ConfigField::RotateUserAgent => ConfigField::BurstSize,
                ConfigField::Preset => ConfigField::RotateUserAgent,
                ConfigField::Theme => ConfigField::Preset,
                ConfigField::RpcEnabled => ConfigField::Theme,
                ConfigField::AutoSave => ConfigField::RpcEnabled,
            };
        }
    }

    pub fn handle_enter(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });

        match self.selected_field {
            ConfigField::Mode => self.cycle_mode(),
            ConfigField::RandomPayload
            | ConfigField::RandomPorts
            | ConfigField::SecondaryAttack => self.toggle_boolean_field(),
            ConfigField::EvasMode => self.cycle_evasion_mode(),
            ConfigField::SizeStrategy => self.cycle_size_strategy(),
            ConfigField::Preset => {
                // Handle preset selection
                if self.section_active && self.selected_section == 4 {
                    self.apply_preset();
                }
            }
            ConfigField::Theme | ConfigField::RpcEnabled | ConfigField::AutoSave => {
                // Only handle settings if we're in the settings section
                if self.selected_section == 5 && self.section_active {
                    self.handle_settings_field();
                }
            }
            _ => self.start_input(),
        }
    }

    pub fn handle_tab(&mut self) {
        if matches!(self.selected_field, ConfigField::Mode) {
            self.cycle_mode();
        }
    }

    pub fn handle_space(&mut self) {
        if matches!(
            self.selected_field,
            ConfigField::RandomPayload | ConfigField::RandomPorts | ConfigField::SecondaryAttack
        ) {
            self.toggle_boolean_field();
        } else if matches!(
            self.selected_field,
            ConfigField::Theme | ConfigField::RpcEnabled | ConfigField::AutoSave
        ) {
            // only handle settings if in the settings section
            if self.selected_section == 4 && self.section_active {
                self.handle_settings_field();
            }
        }
    }

    pub fn handle_char(&mut self, c: char) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });
        self.input_buffer.push(c);
    }

    pub fn handle_backspace(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });
        self.input_buffer.pop();
    }

    pub fn cancel_input(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
            cache.mark_dirty(DirtyRegion::FieldHelp);
        });
        self.input_mode = false;
        self.input_buffer.clear();
    }

    fn toggle_boolean_field(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });

        match self.selected_field {
            ConfigField::RandomPayload => self.config.random_payload = !self.config.random_payload,
            ConfigField::RandomPorts => self.config.random_ports = !self.config.random_ports,
            ConfigField::SecondaryAttack => {
                self.config.secondary_attack = !self.config.secondary_attack
            }
            _ => {}
        }
    }

    fn cycle_mode(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });

        self.mode_index = (self.mode_index + 1) % 15;
        self.config.mode = match self.mode_index {
            0 => AtkMode::Flood,
            1 => AtkMode::Amplification,
            2 => AtkMode::Fragmentation,
            3 => AtkMode::Slowloris,
            4 => AtkMode::Burst,
            5 => AtkMode::DNSQuery,
            6 => AtkMode::PortScan,
            7 => AtkMode::UDP,
            8 => AtkMode::TCP,
            9 => AtkMode::TCPConnect,
            10 => AtkMode::HTTP,
            11 => AtkMode::DNSFlood,
            _ => AtkMode::Flood,
        };
    }

    fn cycle_evasion_mode(&mut self) {
        self.config.evasion_mode = match self.config.evasion_mode {
            EvasMode::Fixed => EvasMode::Random,
            EvasMode::Random => EvasMode::Burst,
            EvasMode::Burst => EvasMode::Exponential,
            EvasMode::Exponential => EvasMode::Adaptive,
            EvasMode::Adaptive => EvasMode::Fixed,
        };
    }

    fn cycle_size_strategy(&mut self) {
        self.config.size_strategy = match self.config.size_strategy {
            SizeStrategy::Fixed => SizeStrategy::Random,
            SizeStrategy::Random => SizeStrategy::Oscillating,
            SizeStrategy::Oscillating => SizeStrategy::Fixed,
        };
    }

    fn start_input(&mut self) {
        self.input_mode = true;
        // @note: Discord RPC input mode tracking is handled in main.rs
        self.input_buffer = match self.selected_field {
            ConfigField::Target => self.config.target.clone(),
            ConfigField::Port => self.config.port.to_string(),
            ConfigField::Threads => self.config.threads.to_string(),
            ConfigField::Rate => self.config.rate.to_string(),
            ConfigField::Duration => self.config.duration.to_string(),
            ConfigField::PacketSize => self.config.packet_size.to_string(),
            ConfigField::CustomPayload => self.config.custom_payload.clone(),
            ConfigField::VariancePercentage => self.config.variance_percentage.to_string(),
            ConfigField::BurstSize => self.config.burst_size.to_string(),
            ConfigField::RotateUserAgent => {
                if self.config.rotate_user_agent {
                    "true".to_string()
                } else {
                    "false".to_string()
                }
            }
            _ => String::new(),
        };
    }

    pub fn finish_input(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
            cache.mark_dirty(DirtyRegion::FieldHelp);
            cache.mark_dirty(DirtyRegion::TargetStatus);
        });

        self.input_mode = false;
        match self.selected_field {
            ConfigField::Target => {
                if !self.input_buffer.trim().is_empty() {
                    self.config.target = self.input_buffer.trim().to_string();
                }
            }
            ConfigField::Port => {
                if let Ok(port) = self.input_buffer.parse::<u16>() {
                    if port > 0 {
                        self.config.port = port;
                    }
                }
            }
            ConfigField::Threads => {
                if let Ok(threads) = self.input_buffer.parse::<usize>() {
                    if threads > 0 {
                        self.config.threads = threads.min(100);
                    }
                }
            }
            ConfigField::Rate => {
                if let Ok(rate) = self.input_buffer.parse::<u64>() {
                    if rate > 0 {
                        self.config.rate = rate.min(1000000);
                    }
                }
            }
            ConfigField::Duration => {
                if let Ok(duration) = self.input_buffer.parse::<u64>() {
                    if duration > 0 {
                        self.config.duration = duration.min(3600);
                    }
                }
            }
            ConfigField::PacketSize => {
                if let Ok(size) = self.input_buffer.parse::<usize>() {
                    if size > 0 {
                        self.config.packet_size = size.min(65507);
                    }
                }
            }
            ConfigField::CustomPayload => {
                self.config.custom_payload = self.input_buffer.trim().to_string();
            }
            ConfigField::VariancePercentage => {
                if let Ok(variance) = self.input_buffer.parse::<u8>() {
                    self.config.variance_percentage = variance.min(100);
                }
            }
            ConfigField::BurstSize => {
                if let Ok(burst) = self.input_buffer.parse::<u32>() {
                    if burst > 0 {
                        self.config.burst_size = burst.min(1000);
                    }
                }
            }
            ConfigField::RotateUserAgent => {
                if let Ok(enabled) = self.input_buffer.parse::<bool>() {
                    self.config.rotate_user_agent = enabled;
                } else if self.input_buffer.trim().to_lowercase() == "true"
                    || self.input_buffer.trim() == "1"
                {
                    self.config.rotate_user_agent = true;
                } else if self.input_buffer.trim().to_lowercase() == "false"
                    || self.input_buffer.trim() == "0"
                {
                    self.config.rotate_user_agent = false;
                }
            }
            _ => {}
        }
        self.input_buffer.clear();
    }

    pub async fn start_attack(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty(); // @note: state transition requires full redraw
        });

        self.state = AppState::Attack;
        self.stats = AtkStats::new();
        self.stats.start();

        // Update config with selected interface
        self.config.interface = self.selected_interface.clone();

        // create Arc for sharing with workers
        let stats_arc = Arc::new(self.stats.clone());
        self.stats_arc = Some(stats_arc.clone());

        self.add_log("Attack started".to_string());

        // start the worker threads in background
        let handle = tokio::spawn(start_atkworkers(
            self.config.clone(),
            stats_arc,
            self.logs.clone(),
        ));

        // store the attack task handle
        self.attack_handle = Some(handle);

        self.add_log(format!(
            "Deploying {} workers to target {}:{}",
            self.config.threads, self.config.target, self.config.port
        ));

        if let Some(ref interface) = self.config.interface {
            self.add_log(format!("Using interface: {}", interface));
        }
    }

    pub async fn start_attack_direct(&mut self, logs: Arc<Mutex<VecDeque<String>>>) {
        self.state = AppState::Attack;
        self.stats = AtkStats::new();
        self.stats.start();

        // create Arc for sharing with workers
        let stats_arc = Arc::new(self.stats.clone());
        self.stats_arc = Some(stats_arc.clone());

        // Use the provided logs instead of self.logs
        let mut log_queue = logs.lock().unwrap();
        log_queue.push_back("Attack started".to_string());

        // start the worker threads in background
        let handle = tokio::spawn(start_atkworkers(
            self.config.clone(),
            stats_arc,
            logs.clone(),
        ));

        // store the attack task handle
        self.attack_handle = Some(handle);

        log_queue.push_back(format!(
            "Deploying {} workers to target {}:{}",
            self.config.threads, self.config.target, self.config.port
        ));
    }

    pub fn stop_attack(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty(); // State transition requires full redraw
        });

        self.stats.stop();
        self.state = AppState::Results;

        // stop the attack stats first
        if let Some(stats_arc) = &self.stats_arc {
            stats_arc.stop();
        }

        // abort the attack task if it exists
        if let Some(handle) = self.attack_handle.take() {
            handle.abort();
        }

        self.add_log("Attack terminated by user".to_string());
    }

    pub fn show_results(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty(); // @note: state transition requires full redraw
        });

        self.state = AppState::Results;
    }

    pub fn reset_to_config(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty(); // @note: state transition requires full redraw
        });

        self.state = AppState::Config;

        // clean up
        if let Some(handle) = self.attack_handle.take() {
            handle.abort();
        }
    }

    pub fn get_field_value(&self, field: &ConfigField) -> String {
        if self.input_mode && &self.selected_field == field {
            return self.input_buffer.clone();
        }

        match field {
            ConfigField::Target => self.config.target.clone(),
            ConfigField::Port => self.config.port.to_string(),
            ConfigField::Threads => self.config.threads.to_string(),
            ConfigField::Rate => self.config.rate.to_string(),
            ConfigField::Duration => self.config.duration.to_string(),
            ConfigField::PacketSize => self.config.packet_size.to_string(),
            ConfigField::Mode => format!(
                "{} - {}",
                self.config.mode.to_string(),
                self.config.mode.description()
            ),
            ConfigField::CustomPayload => {
                if self.config.custom_payload.is_empty() {
                    "(empty)".to_string()
                } else {
                    self.config.custom_payload.clone()
                }
            }
            ConfigField::RandomPayload => if self.config.random_payload {
                "Yes"
            } else {
                "No"
            }
            .to_string(),
            ConfigField::RandomPorts => if self.config.random_ports {
                "Yes"
            } else {
                "No"
            }
            .to_string(),
            ConfigField::EvasMode => self.config.evasion_mode.to_string().to_string(),
            ConfigField::SizeStrategy => self.config.size_strategy.to_string().to_string(),
            ConfigField::SecondaryAttack => if self.config.secondary_attack {
                "Yes"
            } else {
                "No"
            }
            .to_string(),
            ConfigField::VariancePercentage => format!("{}%", self.config.variance_percentage),
            ConfigField::BurstSize => self.config.burst_size.to_string(),
            ConfigField::RotateUserAgent => {
                if self.config.rotate_user_agent {
                    "Yes".to_string()
                } else {
                    "No".to_string()
                }
            }
            ConfigField::Preset => {
                let presets = [
                    "Basic",
                    "AntiDDoS",
                    "Amplification",
                    "Stealth",
                    "MultiVector",
                    "HighThroughput",
                    "Custom",
                ];
                presets[self.preset_index].to_string()
            }
            ConfigField::Theme => {
                let themes = [
                    AppTheme::TokyoNight,
                    AppTheme::Dracula,
                    AppTheme::Gruvbox,
                    AppTheme::Solarized,
                    AppTheme::Monokai,
                    AppTheme::Nord,
                ];
                themes[self.theme_index].to_string().to_string()
            }
            ConfigField::RpcEnabled => if self.rpc_enabled {
                "Enabled"
            } else {
                "Disabled"
            }
            .to_string(),
            ConfigField::AutoSave => if self.auto_save {
                "Enabled"
            } else {
                "Disabled"
            }
            .to_string(),
        }
    }

    pub fn get_logs(&self) -> Vec<String> {
        let log_queue = self.logs.lock().unwrap();
        log_queue.iter().cloned().collect()
    }

    // nav methods
    pub fn next_section(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::Navigation);
        });

        if self.selected_section < CONFIG_SECTIONS.len() - 1 {
            self.selected_section += 1;
        } else {
            self.selected_section = 0;
        }
    }

    pub fn previous_section(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::Navigation);
        });

        if self.selected_section > 0 {
            self.selected_section -= 1;
        } else {
            self.selected_section = CONFIG_SECTIONS.len() - 1;
        }
    }

    pub fn enter_section(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::Navigation);
            cache.mark_dirty(DirtyRegion::ConfigForm);
            cache.mark_dirty(DirtyRegion::FieldHelp);
        });

        self.section_active = true;
        let (_, _, fields) = CONFIG_SECTIONS[self.selected_section];
        if let Some(first_field) = fields.first() {
            self.selected_field = *first_field;
        }
    }

    pub fn exit_section(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::Navigation);
            cache.mark_dirty(DirtyRegion::ConfigForm);
            cache.mark_dirty(DirtyRegion::FieldHelp);
        });

        self.section_active = false;
        self.cancel_input();
    }

    pub fn is_section_active(&self) -> bool {
        self.section_active
    }

    pub fn sync_stats(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });

        if let Some(stats_arc) = &self.stats_arc {
            // Sync the atomic values
            self.stats.packets_sent.store(
                stats_arc.packets_sent.load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
            self.stats.bytes_sent.store(
                stats_arc.bytes_sent.load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
            self.stats.missed_pkgs.store(
                stats_arc.missed_pkgs.load(Ordering::Relaxed),
                Ordering::Relaxed,
            );
        }
    }

    // cheat sheet methods
    pub fn toggle_cheat_sheet(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty(); // @note: cheat sheet overlay requires full redraw
        });
        self.show_cheat_sheet = !self.show_cheat_sheet;
    }

    pub fn hide_cheat_sheet(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty(); // @note: cheat sheet overlay requires full redraw
        });
        self.show_cheat_sheet = false;
    }

    // Interface selector methods
    pub fn toggle_interface_selector(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty();
        });

        if self.show_interface_selector {
            self.show_interface_selector = false;
            self.interface_selector = None;
        } else {
            self.interface_selector = Some(InterfaceSelector::new());
            self.show_interface_selector = true;
        }
    }

    pub fn hide_interface_selector(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty();
        });
        self.show_interface_selector = false;
        self.interface_selector = None;
    }

    pub fn handle_interface_event(&mut self, event: InterfaceEvent) {
        match event {
            InterfaceEvent::Select(_index) => {
                if let Some(ref selector) = self.interface_selector {
                    if let Some(interface) = selector.selected_interface() {
                        self.add_log(format!("Selected interface: {}", interface.name));
                        self.selected_interface = Some(interface.name.clone());
                    }
                }
                self.hide_interface_selector();
            }
            InterfaceEvent::Cancel => {
                self.hide_interface_selector();
            }
        }
    }

    // settings methods
    pub fn toggle_rpc(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });
        self.rpc_enabled = !self.rpc_enabled;
    }

    pub fn toggle_auto_save(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });
        self.auto_save = !self.auto_save;
    }

    pub fn next_theme(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty(); // @note: theme change requires full redraw
        });
        self.theme_index = (self.theme_index + 1) % 6; // 6 themes
    }

    pub fn handle_settings_field(&mut self) {
        match self.selected_field {
            ConfigField::Theme => self.next_theme(),
            ConfigField::RpcEnabled => self.toggle_rpc(),
            ConfigField::AutoSave => self.toggle_auto_save(),
            _ => {}
        }
    }

    // preset methods
    pub fn next_preset(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });
        self.preset_index = (self.preset_index + 1) % 7; // 7 presets
    }

    pub fn prev_preset(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });
        self.preset_index = if self.preset_index == 0 { 6 } else { self.preset_index - 1 };
    }

    pub fn apply_preset(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_dirty(DirtyRegion::ConfigForm);
        });

        let preset = match self.preset_index {
            0 => AttackPreset::Basic,
            1 => AttackPreset::AntiDDoS,
            2 => AttackPreset::Amplification,
            3 => AttackPreset::Stealth,
            4 => AttackPreset::MultiVector,
            5 => AttackPreset::HighThroughput,
            _ => AttackPreset::Custom,
        };

        self.selected_preset = Some(preset.clone());
        let preset_config = preset.get_config(&self.config.target, self.config.port);

        // apply preset configuration
        self.config.threads = preset_config.threads;
        self.config.rate = preset_config.rate;
        self.config.duration = preset_config.duration;
        self.config.packet_size = preset_config.packet_size;
        self.config.mode = preset_config.mode;
        self.config.evasion_mode = preset_config.evasion_mode;
        self.config.size_strategy = preset_config.size_strategy;
        self.config.secondary_attack = preset_config.secondary_attack;
        self.config.variance_percentage = preset_config.variance_percentage;
        self.config.burst_size = preset_config.burst_size;

        self.add_log(format!("Applied preset: {:?}", preset));
    }


    // save config to file
    pub fn save_config(&self, filename: &str) -> io::Result<()> {
        let config_data = serde_json::to_string_pretty(&self.config)?;
        fs::write(filename, config_data)?;
        self.add_log(format!("Configuration saved to {}", filename));
        Ok(())
    }

    // load config from file
    pub fn load_config(&mut self, filename: &str) -> io::Result<()> {
        let config_data = fs::read_to_string(filename)?;
        let loaded_config: AtkConfig = serde_json::from_str(&config_data)?;
        self.config = loaded_config;
        self.add_log(format!("Configuration loaded from {}", filename));
        Ok(())
    }

    // tutorial methods
    pub fn toggle_tutorial(&mut self) {
        RENDER_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.mark_all_dirty();
        });

        if self.show_tutorial {
            self.show_tutorial = false;
        } else {
            self.tutorial.start();
            self.show_tutorial = true;
        }
    }
}
