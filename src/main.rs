mod app;
mod network;
mod types;
mod ui;
mod utils;
mod config;

use app::cli::Args;
use clap::Parser;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::time::{Duration, Instant};

use ui::ui::{RENDER_CACHE, DirtyRegion};
use config::config::CONFIG_SECTIONS;
use types::types::{ConfigField, AtkMode, EvasMode};

use app::app::App;
use utils::discord_rpc::DiscordRPC;
use ctrlc;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

fn apply_cli_args(app: &mut App, args: &Args) {
    if let Some(ref target) = args.target {
        app.config.target = target.clone();
    }
    app.config.port = args.port;
    app.config.threads = args.threads;
    app.config.rate = args.rate;
    app.config.duration = args.duration;
    app.config.packet_size = args.packet_size;
    app.config.mode = match args.mode.as_str() {
        "flood" => AtkMode::Flood,
        "amplification" => AtkMode::Amplification,
        "fragmentation" => AtkMode::Fragmentation,
        "slowloris" => AtkMode::Slowloris,
        "burst" => AtkMode::Burst,
        "tcp" => AtkMode::TCP,
        "tcpconnect" => AtkMode::TCPConnect,
        "http" => AtkMode::HTTP,
        "udp" => AtkMode::UDP,
        "portscan" => AtkMode::PortScan,
        "dnsquery" => AtkMode::DNSQuery,
        "dnsflood" => AtkMode::DNSFlood,
        _ => AtkMode::Flood,
    };

    app.config.evasion_mode = match args.evasion.as_str() {
        "fixed" => EvasMode::Fixed,
        "random" => EvasMode::Random,
        "adaptive" => EvasMode::Adaptive,
        "exponential" => EvasMode::Exponential,
        _ => EvasMode::Fixed,
    };

    if let Some(ref payload) = args.payload {
        app.config.custom_payload = payload.clone();
    }
    app.config.random_payload = args.random_payload;
    app.config.random_ports = args.random_ports;
    app.config.variance_percentage = args.variance;
    app.config.burst_size = args.burst_size;
    app.theme_index = match args.theme.as_str() {
        "tokyo-night" => 0,
        "dracula" => 1,
        "gruvbox" => 2,
        "solarized" => 3,
        "monokai" => 4,
        "nord" => 5,
        _ => 0,
    };

    if let Some(ref preset) = args.preset {
        app.preset_index = match preset.as_str() {
            "basic" => 0,
            "anti-ddos" => 1,
            "amplification" => 2,
            "stealth" => 3,
            "multi-vector" => 4,
            "high-throughput" => 5,
            _ => 0,
        };
        app.apply_preset();
    }

    if args.secondary_attack.is_some() {
        app.config.secondary_attack = true;
    }

    app.rpc_enabled = args.discord_rpc;
    if args.no_tui {
        app.auto_save = true;
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    discord_rpc: &mut DiscordRPC,
    args: &Args,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new();
    apply_cli_args(&mut app, args); // apply CLI args to app configuration

    loop {
        if app.is_attack_state() {
            app.sync_stats();
        }
        if app.is_config_state() || app.is_attack_state() {
            if let Some(stats_arc) = &app.stats_arc {
                let status = stats_arc.target_status.lock().unwrap();
                let check_udp = status.last_checked.is_none()
                    || status.last_checked.unwrap().elapsed() > std::time::Duration::from_secs(3);

                drop(status);

                if check_udp {
                    RENDER_CACHE.with(|cache| {
                        let mut cache = cache.borrow_mut();
                        cache.mark_dirty(DirtyRegion::TargetStatus);
                    });

                    crate::types::types::AtkStats::upd_target_status(
                        stats_arc,
                        &app.config.target,
                        app.config.port,
                    );
                }
            }
        }

        terminal.draw(|f| crate::ui::ui::ui(f, &mut app))?;
        if crossterm::event::poll(std::time::Duration::from_millis(16))? {
            match event::read()? {
                Event::Key(key) => {
                    if key.kind == KeyEventKind::Press {
                        discord_rpc.update_activity();

                    if key.code == KeyCode::Char('/') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.toggle_cheat_sheet();
                        continue;
                    }

                    if key.code == KeyCode::Char('i') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.toggle_interface_selector();
                        continue;
                    }
                    if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
                        app.toggle_tutorial();
                        continue;
                    }

                    if app.show_interface_selector {
                        if let Some(ref mut selector) = app.interface_selector {
                            if let Some(event) = selector.handle_event(Event::Key(key)) {
                                app.handle_interface_event(event);
                            }
                        }
                        continue;
                    }

                    if app.show_tutorial {
                        let handled = app.tutorial.handle_event(Event::Key(key));
                        if !app.tutorial.is_active {
                            app.show_tutorial = false;
                        }
                        if handled {
                            continue;
                        }
                    }

                    if app.show_cheat_sheet {
                        app.hide_cheat_sheet();
                        continue;
                    } else if app.input_mode {
                        match key.code {
                            KeyCode::Enter => app.finish_input(),
                            KeyCode::Esc => app.cancel_input(),
                            KeyCode::Backspace => app.handle_backspace(),
                            KeyCode::Char(c) => app.handle_char(c),
                            _ => {}
                        }
                    } else if app.is_config_state() {
                        if app.is_section_active() {
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Up => app.previous_field(),
                                KeyCode::Down => app.next_field(),
                                KeyCode::Enter => {
                                    let was_input_mode = app.input_mode;
                                    app.handle_enter();
                                    if !was_input_mode && app.input_mode {
                                        discord_rpc.set_input_mode(true);
                                        discord_rpc.update_activity();
                                    }
                                }
                                KeyCode::Left => {
                                    if app.section_active && app.selected_field == ConfigField::Preset {
                                        app.prev_preset();
                                    } else {
                                        app.previous_field();
                                    }
                                }
                                KeyCode::Right => {
                                    if app.section_active && app.selected_field == ConfigField::Preset {
                                        app.next_preset();
                                    } else {
                                        app.next_field();
                                    }
                                },
                                KeyCode::Tab => app.handle_tab(),
                                KeyCode::Esc => app.exit_section(),
                                KeyCode::Char(' ') => app.handle_space(),
                                KeyCode::F(1) => {
                                    app.start_attack().await;
                                    // Update Discord RPC
                                    if let Err(e) = discord_rpc.update_presence(
                                        &app.state,
                                        &format!("Target: {}:{}", app.config.target, app.config.port)
                                    ) {
                                        eprintln!("Failed to update Discord RPC: {}", e);
                                    }
                                }
                                KeyCode::F(2) => {
                                    if app.is_attack_state() {
                                        app.stop_attack();
                                        // Update Discord RPC
                                        if let Err(e) = discord_rpc.update_presence(
                                            &app.state,
                                            "Attack stopped"
                                        ) {
                                            eprintln!("Failed to update Discord RPC: {}", e);
                                        }
                                    }
                                }
                                KeyCode::F(3) => {
                                    if app.is_attack_state() {
                                        app.show_results();
                                        // Update Discord RPC
                                        if let Err(e) = discord_rpc.update_presence(
                                            &app.state,
                                            "Viewing attack results"
                                        ) {
                                            eprintln!("Failed to update Discord RPC: {}", e);
                                        }
                                    }
                                }
                                KeyCode::F(6) => {
                                    // save config to JSON
                                    if let Err(e) = app.save_config("config.json") {
                                        app.add_log(format!("Failed to save config: {}", e));
                                    }
                                }
                                KeyCode::F(7) => {
                                    // load config from JSON
                                    if let Err(e) = app.load_config("config.json") {
                                        app.add_log(format!("Failed to load config: {}", e));
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            // nav mode
                            match key.code {
                                KeyCode::Char('q') => break,
                                KeyCode::Up => {
                                    app.previous_section();
                                    discord_rpc.set_section(&CONFIG_SECTIONS[app.selected_section].0);
                                    discord_rpc.update_activity();
                                }
                                KeyCode::Down => {
                                    app.next_section();
                                    discord_rpc.set_section(&CONFIG_SECTIONS[app.selected_section].0);
                                    discord_rpc.update_activity();
                                }
                                KeyCode::Enter => app.enter_section(),
                                KeyCode::F(1) => {
                                    app.start_attack().await;
                                    if let Err(e) = discord_rpc.update_presence(
                                        &app.state,
                                        &format!("Target: {}:{}", app.config.target, app.config.port)
                                    ) {
                                        eprintln!("Failed to update Discord RPC: {}", e);
                                    }
                                }
                                KeyCode::F(6) => {
                                    // save configuration to JSON
                                    if let Err(e) = app.save_config("config.json") {
                                        app.add_log(format!("Failed to save config: {}", e));
                                    }
                                }
                                KeyCode::F(7) => {
                                    // load configuration from JSON
                                    if let Err(e) = app.load_config("config.json") {
                                        app.add_log(format!("Failed to load config: {}", e));
                                    }
                                }
                                _ => {}
                            }
                        }
                    } else if app.is_attack_state() {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::F(2) => app.stop_attack(),
                            KeyCode::F(3) => app.show_results(),
                            _ => {}
                        }
                    } else if app.is_results_state() {
                        match key.code {
                            KeyCode::Char('q') => break,
                            KeyCode::F(1) => {
                                app.reset_to_config();
                                if let Err(e) = discord_rpc.update_presence(
                                    &app.state,
                                    "Configuring new attack"
                                ) {
                                    eprintln!("Failed to update Discord RPC: {}", e);
                                }
                            }
                            _ => {}
                        }
                    }
                }
                }
                Event::Mouse(mouse) => {
                    // mouse nav disabled cause of bugs
                    if app.show_interface_selector {
                        if let Some(ref mut selector) = app.interface_selector {
                            if let Some(event) = selector.handle_event(Event::Mouse(mouse.clone())) {
                                app.handle_interface_event(event);
                                continue;
                            }
                        }
                    }

                    if app.show_tutorial {
                        let handled = app.tutorial.handle_event(Event::Mouse(mouse.clone()));
                        if !app.tutorial.is_active {
                            app.show_tutorial = false;
                        }
                        if handled {
                            continue;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        eprintln!("\nFor usage information, run: {} --help", std::env::args().next().unwrap_or_else(|| "skibidi-rizz".to_string()));
        std::process::exit(1);
    }

    if args.no_tui {
        if args.target.is_none() {
            eprintln!("Error: Target is required when using --no-tui");
            std::process::exit(1);
        }

        println!("Starting attack with configuration:");
        println!("  Target: {}:{}", args.target.as_ref().unwrap(), args.port);
        println!("  Mode: {}", args.mode);
        println!("  Threads: {}", args.threads);
        println!("  Rate: {} PPS", args.rate);
        println!("  Duration: {} seconds", args.duration);
        println!("  Packet Size: {} bytes", args.packet_size);
        if args.verbose {
            println!("  Evasion: {}", args.evasion);
            println!("  Random Payload: {}", args.random_payload);
            println!("  Random Ports: {}", args.random_ports);
        }
        println!("\nPress Ctrl+C to stop the attack...");

        // create config from args
        let mut app = App::new();
        apply_cli_args(&mut app, &args);

        // set up Ctrl+C handler
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        ctrlc::set_handler(move || {
            r.store(false, Ordering::Relaxed);
            println!("\nStopping attack...");
        }).unwrap_or_else(|e| {
            eprintln!("Warning: Could not set Ctrl+C handler: {}", e);
        });

        // start the attack
        let logs = Arc::new(Mutex::new(VecDeque::new()));
        app.start_attack_direct(logs.clone()).await;

        // run for the specified duration
        let start_time = Instant::now();
        while running.load(Ordering::Relaxed) && start_time.elapsed().as_secs() < args.duration {
            if let Some(stats) = &app.stats_arc {
                let packets_sent = stats.packets_sent.load(Ordering::Relaxed);
                let bytes_sent = stats.bytes_sent.load(Ordering::Relaxed);
                println!("Packets sent: {}, Bytes sent: {}", packets_sent, bytes_sent);
            }

            // print recent logs
            let logs_guard = logs.lock().unwrap();
            for log in logs_guard.iter().rev().take(5) {
                println!("LOG: {}", log);
            }
            drop(logs_guard);

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        // stop the attack
        app.stop_attack();
        println!("Attack completed!");

        // final stats
        if let Some(stats) = &app.stats_arc {
            let packets_sent = stats.packets_sent.load(Ordering::Relaxed);
            let bytes_sent = stats.bytes_sent.load(Ordering::Relaxed);
            println!("Final stats:");
            println!("  Total packets sent: {}", packets_sent);
            println!("  Total bytes sent: {}", bytes_sent);

            let duration = start_time.elapsed().as_secs_f64();
            if duration > 0.0 {
                let pps = packets_sent as f64 / duration;
                let bps = bytes_sent as f64 / duration;
                println!("  Average PPS: {:.0}", pps);
                println!("  Average BPS: {:.0}", bps);
            }
        }

        return Ok(());
    }

    let mut discord_rpc = DiscordRPC::new();
    if args.discord_rpc {
        if let Err(e) = discord_rpc.init() {
            eprintln!("Discord RPC failed: {}", e);
        }
    }

    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let res = run_app(&mut terminal, &mut discord_rpc, &args).await;

    discord_rpc.shutdown();

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}