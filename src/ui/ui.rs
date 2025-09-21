use crate::app::app::App;
use crate::config::config::{Theme, CONFIG_SECTIONS};
use crate::types::types::{AppState, ConfigField};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Axis, Block, Borders, Chart, Dataset, List, ListItem, Paragraph, Wrap, Clear, Table, Row, HighlightSpacing},
    Frame,
};
use std::collections::HashSet;
use std::sync::atomic::Ordering;
use std::time::{Duration};

// dirty region tracking for optimized rendering
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DirtyRegion {
    Header,
    Navigation,
    ConfigForm,
    TargetStatus,
    Logs,
    FieldHelp,
    Footer,
    FullScreen, // Force full redraw
}

pub struct RenderCache {
    pub dirty_regions: HashSet<DirtyRegion>,
    pub frame_count: u32,
}

impl RenderCache {
    pub fn new() -> Self {
        Self {
            dirty_regions: HashSet::new(),
            frame_count: 0,
        }
    }

    pub fn mark_dirty(&mut self, region: DirtyRegion) {
        self.dirty_regions.insert(region);
    }

    pub fn mark_all_dirty(&mut self) {
        self.dirty_regions.insert(DirtyRegion::FullScreen);
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_regions.clear();
    }

    pub fn needs_full_redraw(&self) -> bool {
        self.dirty_regions.contains(&DirtyRegion::FullScreen) || self.frame_count == 0
    }

    pub fn is_region_dirty(&self, region: &DirtyRegion) -> bool {
        self.needs_full_redraw() || self.dirty_regions.contains(region)
    }
}

thread_local! {
    pub static RENDER_CACHE: std::cell::RefCell<RenderCache> = std::cell::RefCell::new(RenderCache::new());
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = (r.width - popup_width) / 2;
    let popup_y = (r.height - popup_height) / 2;

    Rect::new(
        r.x + popup_x,
        r.y + popup_y,
        popup_width,
        popup_height,
    )
}

pub fn ui(f: &mut Frame, app: &mut App) {
    RENDER_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let is_first_frame = cache.frame_count == 0;

        cache.frame_count += 1;
        if is_first_frame {
            cache.mark_all_dirty();
        } else {
            cache.mark_all_dirty();
        }

        if app.show_cheat_sheet {
            cache.mark_all_dirty();
        }

        let theme = Theme::get_current(app);
        let background = Block::default().style(Style::default().bg(theme.bg_dark));

        if cache.needs_full_redraw() {
            f.render_widget(background, f.size());
        }

        if app.show_cheat_sheet {
            draw_cheat_sheet(f, app, &theme);
            cache.clear_dirty();
            return;
        }

        if app.show_interface_selector {
            if let Some(ref mut selector) = app.interface_selector {
                // Create a centered area for the interface selector
                let area = centered_rect(80, 70, f.size());
                f.render_widget(Clear, area);
                selector.render(f, area);
            }
            cache.clear_dirty();
            return;
        }

        if app.show_tutorial {
            // Create a centered area for the tutorial
            let area = centered_rect(90, 85, f.size());
            app.tutorial.render(f, area);
            cache.clear_dirty();
            return;
        }

        match app.state {
            AppState::Config => draw_config_screen_optimized(f, app, &theme, &mut cache),
            AppState::Attack => draw_attack_screen_optimized(f, app, &theme, &mut cache),
            AppState::Results => draw_results_screen_optimized(f, app, &theme, &mut cache),
        }

        cache.clear_dirty();
    });
}

fn draw_config_screen_optimized(f: &mut Frame, app: &App, theme: &Theme, cache: &mut RenderCache) {
    if cache.needs_full_redraw() {
        draw_config_screen(f, app, theme);
        return;
    }

    let size = f.size();

    if cache.is_region_dirty(&DirtyRegion::Header) {
        draw_header_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Navigation) {
        draw_navigation_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::ConfigForm) {
        draw_config_form_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::TargetStatus) {
        draw_target_status_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Logs) {
        draw_logs_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::FieldHelp) {
        draw_field_help_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Footer) {
        draw_cool_footer_partial(f, app, theme, size);
    }
}

fn draw_header_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let header_area = Rect {
        x: 0,
        y: 0,
        width: size.width,
        height: 3,
    };

    let header_block = Block::default()
        .title(" VertexAttacker ")
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main));

    let mut header_content = Vec::new();
    let state_text = match app.state {
        AppState::Config => "Configuration",
        AppState::Attack => "Attack in Progress",
        AppState::Results => "Attack Results",
    };

    header_content.push(Line::from(vec![
        Span::styled("State: ", Style::default().fg(theme.text_dim)),
        Span::styled(state_text, Style::default().fg(theme.text_bright)),
    ]));

    let version = " v1.0.0 ";
    let version_text = format!("└{}", "─".repeat(header_area.width as usize - version.len() - 2));

    header_content.push(Line::from(Span::styled(
        version_text,
        Style::default().fg(theme.text_dim),
    )));

    let header = Paragraph::new(header_content)
        .block(header_block)
        .alignment(Alignment::Left);

    f.render_widget(Clear, header_area);
    f.render_widget(header, header_area);
}

fn draw_navigation_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let nav_area = Rect {
        x: 0,
        y: 3,
        width: 32,
        height: size.height - 5,
    };

    let nav_block = Block::default()
        .title(" Sections (Click to navigate) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main));

    let mut items = Vec::new();

    for (i, section) in CONFIG_SECTIONS.iter().enumerate() {
        let (name, _, _) = section;
        let style = if i == app.selected_section {
            Style::default()
                .fg(theme.text_bright)
                .bg(theme.bg_float)
                .add_modifier(Modifier::BOLD)
        } else if app.section_active {
            Style::default().fg(theme.text_dim)
        } else {
            Style::default().fg(theme.text_normal)
        };

        items.push(Line::from(Span::styled(format!(" {} ", name), style)));
    }

    let nav_list = List::new(items)
        .block(nav_block)
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
        .highlight_spacing(HighlightSpacing::Always);

    f.render_widget(Clear, nav_area);
    f.render_widget(nav_list, nav_area);
}

fn draw_config_form_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let form_area = Rect {
        x: 32,
        y: 3,
        width: size.width - 32,
        height: size.height - 5,
    };

    let form_block = Block::default()
        .title(" Configuration ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main));

    f.render_widget(Clear, form_area);
    f.render_widget(form_block, form_area);

    // Draw fields based on current section
    let current_section = &CONFIG_SECTIONS[app.selected_section];
    let fields = current_section.2;

    let mut form_items = Vec::new();
    let mut field_y = form_area.y + 1;

    for (i, field) in fields.iter().enumerate() {
        let field_style = if app.section_active && {
            let (_, _, fields) = CONFIG_SECTIONS[app.selected_section];
            fields.get(i).map_or(false, |&f| f == app.selected_field)
        } {
            Style::default()
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.text_normal)
        };

        let field_line = get_field_display_line(app, field, field_style, theme);
        form_items.push((field_line, field_y));

        field_y += 2;
        if field_y >= form_area.bottom() - 1 {
            break;
        }
    }

    // Render each field
    for (line, y) in form_items {
        let field_area = Rect {
            x: form_area.x + 1,
            y,
            width: form_area.width - 2,
            height: 1,
        };

        let paragraph = Paragraph::new(line)
            .style(Style::default().bg(theme.bg_main));

        f.render_widget(paragraph, field_area);
    }
}

fn get_field_display_line<'a>(app: &'a App, field: &ConfigField, style: Style, _theme: &'a Theme) -> Line<'a> {
    match field {
        ConfigField::Target => {
            let value = if app.input_mode && app.selected_field == ConfigField::Target {
                format!("Target: {}_", app.input_buffer)
            } else {
                format!("Target: {}", app.config.target)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::Port => {
            let value = if app.input_mode && app.selected_field == ConfigField::Port {
                format!("Port: {}_", app.input_buffer)
            } else {
                format!("Port: {}", app.config.port)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::Threads => {
            let value = if app.input_mode && app.selected_field == ConfigField::Threads {
                format!("Threads: {}_", app.input_buffer)
            } else {
                format!("Threads: {}", app.config.threads)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::Duration => {
            let value = if app.input_mode && app.selected_field == ConfigField::Duration {
                format!("Duration (s): {}_", app.input_buffer)
            } else {
                format!("Duration (s): {}", app.config.duration)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::PacketSize => {
            let value = if app.input_mode && app.selected_field == ConfigField::PacketSize {
                format!("Packet Size: {}_", app.input_buffer)
            } else {
                format!("Packet Size: {}", app.config.packet_size)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::Mode => {
            let mode_name = format!("{:?}", app.config.mode);
            let value = format!("Attack Mode: {}", mode_name);
            Line::from(Span::styled(value, style))
        }
        ConfigField::CustomPayload => {
            let value = if app.input_mode && app.selected_field == ConfigField::CustomPayload {
                format!("Payload: {}_", app.input_buffer)
            } else if app.config.custom_payload.is_empty() {
                "Payload: [Default]".to_string()
            } else {
                format!("Payload: {}...", &app.config.custom_payload[..20.min(app.config.custom_payload.len())])
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::RandomPayload => {
            let value = format!("Random Payload: {}", if app.config.random_payload { "ON" } else { "OFF" });
            Line::from(Span::styled(value, style))
        }
        ConfigField::RandomPorts => {
            let value = format!("Random Ports: {}", if app.config.random_ports { "ON" } else { "OFF" });
            Line::from(Span::styled(value, style))
        }
        ConfigField::Rate => {
            let value = if app.input_mode && app.selected_field == ConfigField::Rate {
                format!("Rate (PPS): {}_", app.input_buffer)
            } else {
                format!("Rate (PPS): {}", app.config.rate)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::EvasMode => {
            let value = format!("Evasion Mode: {:?}", app.config.evasion_mode);
            Line::from(Span::styled(value, style))
        }
        ConfigField::SizeStrategy => {
            let value = format!("Size Strategy: {:?}", app.config.size_strategy);
            Line::from(Span::styled(value, style))
        }
        ConfigField::SecondaryAttack => {
            let value = format!("Secondary Attack: {:?}", app.config.secondary_attack);
            Line::from(Span::styled(value, style))
        }
        ConfigField::VariancePercentage => {
            let value = if app.input_mode && app.selected_field == ConfigField::VariancePercentage {
                format!("Variance %: {}_", app.input_buffer)
            } else {
                format!("Variance %: {}", app.config.variance_percentage)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::BurstSize => {
            let value = if app.input_mode && app.selected_field == ConfigField::BurstSize {
                format!("Burst Size: {}_", app.input_buffer)
            } else {
                format!("Burst Size: {}", app.config.burst_size)
            };
            Line::from(Span::styled(value, style))
        }
        ConfigField::RotateUserAgent => {
            let value = format!("Rotate UA: {}", if app.config.rotate_user_agent { "ON" } else { "OFF" });
            Line::from(Span::styled(value, style))
        }
        ConfigField::Theme => {
            let theme_name = match app.theme_index {
                0 => "Tokyo Night",
                1 => "Dracula",
                2 => "Gruvbox",
                3 => "Solarized",
                4 => "Monokai",
                _ => "Nord",
            };
            let value = format!("Theme: {}", theme_name);
            Line::from(Span::styled(value, style))
        }
        ConfigField::RpcEnabled => {
            let value = format!("Discord RPC: {}", if app.rpc_enabled { "ON" } else { "OFF" });
            Line::from(Span::styled(value, style))
        }
        ConfigField::AutoSave => {
            let value = format!("Auto Save: {}", if app.auto_save { "ON" } else { "OFF" });
            Line::from(Span::styled(value, style))
        }
        ConfigField::Preset => {
            let preset_name = match app.preset_index {
                0 => "Basic",
                1 => "Anti-DDoS",
                2 => "Amplification",
                3 => "Stealth",
                4 => "Multi-Vector",
                5 => "High Throughput",
                _ => "Custom",
            };
            let value = format!("Preset: {}", preset_name);
            Line::from(Span::styled(value, style))
        }
    }
}

fn draw_target_status_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    if !app.is_config_state() && !app.is_attack_state() {
        return;
    }

    let status_area = Rect {
        x: 0,
        y: size.height - 9,
        width: size.width,
        height: 3,
    };

    let status_block = Block::default()
        .title(" Target Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main));

    let mut status_content = Vec::new();

    if let Some(stats_arc) = &app.stats_arc {
        let status = stats_arc.target_status.lock().unwrap();

        if let Some(check_time) = status.last_checked {
            let elapsed = check_time.elapsed();
            if elapsed < Duration::from_secs(5) {
                if status.is_online {
                    status_content.push(Line::from(vec![
                        Span::styled("● ", Style::default().fg(theme.green)),
                        Span::styled(
                            format!("Target {}:{} is reachable (UDP)", app.config.target, app.config.port),
                            Style::default().fg(theme.text_normal),
                        ),
                    ]));
                } else {
                    status_content.push(Line::from(vec![
                        Span::styled("● ", Style::default().fg(theme.red)),
                        Span::styled(
                            format!("Target {}:{} is not reachable (UDP)", app.config.target, app.config.port),
                            Style::default().fg(theme.text_normal),
                        ),
                    ]));
                }
            } else {
                status_content.push(Line::from(vec![
                    Span::styled("● ", Style::default().fg(theme.yellow)),
                    Span::styled(
                        format!("Checking target {}:{}...", app.config.target, app.config.port),
                        Style::default().fg(theme.text_normal),
                    ),
                ]));
            }
        } else {
            status_content.push(Line::from(vec![
                Span::styled("● ", Style::default().fg(theme.yellow)),
                Span::styled(
                    format!("Checking target {}:{}...", app.config.target, app.config.port),
                    Style::default().fg(theme.text_normal),
                ),
            ]));
        }

        drop(status);
    }

    let status_paragraph = Paragraph::new(status_content)
        .block(status_block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, status_area);
    f.render_widget(status_paragraph, status_area);
}

fn draw_logs_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let logs_area = Rect {
        x: 0,
        y: size.height - 6,
        width: size.width,
        height: 3,
    };

    let logs_block = Block::default()
        .title(" Activity Log ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main));

    let logs = app.logs.lock().unwrap();
      let log_text: Vec<Line> = logs
        .iter()
        .rev()
        .take(2)
        .rev()
        .map(|log| {
            // format: "[timestamp] message"
            let log_str = log.as_str();
            let mut parts = log_str.splitn(2, "] ");
            let timestamp_part = parts.next().unwrap_or("");
            let message = parts.next().unwrap_or("");
            let timestamp = timestamp_part.trim_start_matches('[');

            let color = if message.contains("ERROR") {
                theme.red
            } else if message.contains("WARN") {
                theme.yellow
            } else if message.contains("SUCCESS") || message.contains("started") || message.contains("completed") {
                theme.green
            } else {
                theme.text_normal
            };

            Line::from(vec![
                Span::styled(format!("[{}] ", timestamp), Style::default().fg(theme.text_dim)),
                Span::styled(message.to_string(), Style::default().fg(color)),
            ])
        })
        .collect();

    let logs_paragraph = Paragraph::new(log_text)
        .block(logs_block)
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, logs_area);
    f.render_widget(logs_paragraph, logs_area);
}

fn draw_field_help_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    if !app.section_active {
        return;
    }

    let help_area = Rect {
        x: 0,
        y: size.height - 3,
        width: size.width,
        height: 1,
    };

    let current_field = app.selected_field;
    let help_text = match current_field {
        ConfigField::Target => "Enter target IP address or hostname",
        ConfigField::Port => "Enter target port number (1-65535)",
        ConfigField::Threads => "Number of concurrent threads to use",
        ConfigField::Duration => "Attack duration in seconds",
        ConfigField::PacketSize => "Size of UDP packets in bytes",
        ConfigField::Mode => "Attack type configuration",
        ConfigField::CustomPayload => "Custom payload data for packets",
        ConfigField::RandomPayload => "Generate random packet payloads",
        ConfigField::RandomPorts => "Use random source ports",
        ConfigField::Rate => "Packets per second to send",
        ConfigField::EvasMode => "Evasion technique to use",
        ConfigField::SizeStrategy => "Packet size variation strategy",
        ConfigField::SecondaryAttack => "Secondary attack method",
        ConfigField::VariancePercentage => "Timing variance percentage",
        ConfigField::BurstSize => "Packet burst size",
        ConfigField::RotateUserAgent => "Rotate user agent string",
        ConfigField::Theme => "UI color theme selection",
        ConfigField::RpcEnabled => "Discord rich presence integration",
        ConfigField::AutoSave => "Automatically save configuration",
        ConfigField::Preset => "Quick configuration templates",
    };

    let help_paragraph = Paragraph::new(Line::from(Span::styled(
        format!(" {} ", help_text),
        Style::default().fg(theme.text_dim).bg(theme.bg_main),
    )));

    f.render_widget(Clear, help_area);
    f.render_widget(help_paragraph, help_area);
}

fn draw_cool_footer_partial(f: &mut Frame, _app: &App, theme: &Theme, size: Rect) {
    let footer_area = Rect {
        x: 0,
        y: size.height - 1,
        width: size.width,
        height: 1,
    };

    let divider = Line::from(vec![
        Span::styled("└", Style::default().fg(theme.border)),
        Span::styled(
            "─".repeat(footer_area.width as usize - 2),
            Style::default().fg(theme.border),
        ),
        Span::styled("┘", Style::default().fg(theme.border)),
    ]);

    let footer_text = Line::from(Span::styled(
        "  Ctrl + T for Tutorial |  Ctrl + / for shortcuts | Use arrow keys to navigate  ",
        Style::default().fg(theme.text_dim),
    ));

    let footer = Paragraph::new(vec![divider, footer_text])
        .style(Style::default().bg(theme.bg_dark));

    f.render_widget(footer, footer_area);
}

// Attack screen optimized rendering
fn draw_attack_screen_optimized(f: &mut Frame, app: &App, theme: &Theme, cache: &mut RenderCache) {
    if cache.needs_full_redraw() {
        draw_attack_screen(f, app, theme);
        return;
    }

    let size = f.size();

    if cache.is_region_dirty(&DirtyRegion::Header) {
        draw_header_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Navigation) {
        draw_attack_info_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::ConfigForm) {
        draw_attack_stats_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Logs) {
        draw_logs_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Footer) {
        draw_cool_footer_partial(f, app, theme, size);
    }
}

fn draw_attack_info_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let info_area = Rect {
        x: 0,
        y: 3,
        width: size.width,
        height: 4,
    };

    let info_block = Block::default()
        .title(" Attack Information ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.red))
        .style(Style::default().bg(theme.bg_main));

    let mut info_content = Vec::new();
    info_content.push(Line::from(vec![
        Span::styled("Target: ", Style::default().fg(theme.text_dim)),
        Span::styled(
            format!("{}:{}", app.config.target, app.config.port),
            Style::default().fg(theme.text_bright),
        ),
        Span::styled(" | ", Style::default().fg(theme.text_dim)),
        Span::styled("Mode: ", Style::default().fg(theme.text_dim)),
        Span::styled(
            format!("{:?}", app.config.mode),
            Style::default().fg(theme.text_bright),
        ),
    ]));

    info_content.push(Line::from(vec![
        Span::styled("Threads: ", Style::default().fg(theme.text_dim)),
        Span::styled(
            format!("{}", app.config.threads),
            Style::default().fg(theme.text_bright),
        ),
        Span::styled(" | ", Style::default().fg(theme.text_dim)),
        Span::styled("Rate: ", Style::default().fg(theme.text_dim)),
        Span::styled(
            format!("{}/s", format_pps(app.config.rate)),
            Style::default().fg(theme.text_bright),
        ),
    ]));

    let info_paragraph = Paragraph::new(info_content)
        .block(info_block)
        .alignment(Alignment::Left);

    f.render_widget(Clear, info_area);
    f.render_widget(info_paragraph, info_area);
}

fn draw_attack_stats_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let stats_area = Rect {
        x: 0,
        y: 7,
        width: size.width,
        height: size.height - 14,
    };

    let stats_block = Block::default()
        .title(" Attack Statistics ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.green))
        .style(Style::default().bg(theme.bg_main));

    f.render_widget(Clear, stats_area);
    f.render_widget(stats_block, stats_area);

    if let Some(stats_arc) = &app.stats_arc {
        // Calculate stats
        let packets_sent = stats_arc.packets_sent.load(Ordering::Relaxed);
        let bytes_sent = stats_arc.bytes_sent.load(Ordering::Relaxed);
        let missed_pkgs = stats_arc.missed_pkgs.load(Ordering::Relaxed);
        let elapsed = stats_arc.get_elapsed();

        let pps = if elapsed > 0.0 {
            packets_sent as f64 / elapsed
        } else {
            0.0
        };
        let bps = if elapsed > 0.0 {
            (bytes_sent as f64 * 8.0) / elapsed
        } else {
            0.0
        };
        let success_rate = if packets_sent + missed_pkgs > 0 {
            (packets_sent as f64 / (packets_sent + missed_pkgs) as f64) * 100.0
        } else {
            0.0
        };

        // Main stats
        let main_stats_area = Rect {
            x: stats_area.x + 1,
            y: stats_area.y + 1,
            width: stats_area.width - 2,
            height: 8,
        };

        let mut main_stats = Vec::new();
        main_stats.push(Line::from(vec![
            Span::styled("Packets Sent: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}", format_number(packets_sent as f64)),
                Style::default().fg(theme.green).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" | ", Style::default().fg(theme.text_dim)),
            Span::styled("PPS: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}/s", format_pps(pps as u64)),
                Style::default().fg(theme.cyan),
            ),
        ]));

        main_stats.push(Line::from(vec![
            Span::styled("Bytes Sent: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}", format_bytes(bytes_sent)),
                Style::default().fg(theme.blue),
            ),
            Span::styled(" | ", Style::default().fg(theme.text_dim)),
            Span::styled("BPS: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}/s", format_bytes(bps as u64)),
                Style::default().fg(theme.magenta),
            ),
        ]));

        main_stats.push(Line::from(vec![
            Span::styled("Success Rate: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{:.2}%", success_rate),
                Style::default().fg(if success_rate > 90.0 { theme.green } else { theme.yellow }),
            ),
        ]));

        let main_paragraph = Paragraph::new(main_stats);
        f.render_widget(main_paragraph, main_stats_area);

        // Network visualization
        let viz_area = Rect {
            x: stats_area.x + 1,
            y: stats_area.y + 9,
            width: stats_area.width - 2,
            height: stats_area.height - 10,
        };

        draw_network_visualization(f, viz_area, app, theme);
    }
}

// Results screen optimized rendering
fn draw_results_screen_optimized(f: &mut Frame, app: &App, theme: &Theme, cache: &mut RenderCache) {
    if cache.needs_full_redraw() {
        draw_results_screen(f, app, theme);
        return;
    }

    let size = f.size();

    if cache.is_region_dirty(&DirtyRegion::Header) {
        draw_header_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Navigation) {
        draw_results_summary_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::ConfigForm) {
        draw_results_details_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Logs) {
        draw_logs_partial(f, app, theme, size);
    }

    if cache.is_region_dirty(&DirtyRegion::Footer) {
        draw_cool_footer_partial(f, app, theme, size);
    }
}

fn draw_results_summary_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let summary_area = Rect {
        x: 0,
        y: 3,
        width: size.width,
        height: 5,
    };

    let summary_block = Block::default()
        .title(" Attack Summary ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main));

    let mut summary_content = Vec::new();

    if let Some(stats_arc) = &app.stats_arc {
        let start_time = stats_arc.start_time.unwrap_or_else(|| tokio::time::Instant::now());
        let end_time = stats_arc.start_time.unwrap_or_else(|| tokio::time::Instant::now()); // Fallback
        let duration = end_time.duration_since(start_time);

        summary_content.push(Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{:.2}s", duration.as_secs_f64()),
                Style::default().fg(theme.text_bright),
            ),
            Span::styled(" | ", Style::default().fg(theme.text_dim)),
            Span::styled("Mode: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{:?}", app.config.mode),
                Style::default().fg(theme.text_bright),
            ),
        ]));

        summary_content.push(Line::from(vec![
            Span::styled("Total Packets: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}", stats_arc.packets_sent.load(Ordering::Relaxed)),
                Style::default().fg(theme.green),
            ),
            Span::styled(" | ", Style::default().fg(theme.text_dim)),
            Span::styled("Peak Bandwidth: ", Style::default().fg(theme.text_dim)),
            Span::styled(
                format!("{}/s", format_bytes(stats_arc.peak_bandwidth.load(Ordering::Relaxed))),
                Style::default().fg(theme.yellow),
            ),
        ]));

    }

    let summary_paragraph = Paragraph::new(summary_content)
        .block(summary_block)
        .alignment(Alignment::Left);

    f.render_widget(Clear, summary_area);
    f.render_widget(summary_paragraph, summary_area);
}

fn draw_results_details_partial(f: &mut Frame, app: &App, theme: &Theme, size: Rect) {
    let details_area = Rect {
        x: 0,
        y: 8,
        width: size.width,
        height: size.height - 14,
    };

    let details_block = Block::default()
        .title(" Detailed Results ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main));

    f.render_widget(Clear, details_area);
    f.render_widget(details_block, details_area);

    if let Some(stats_arc) = &app.stats_arc {
        let packets_sent = stats_arc.packets_sent.load(Ordering::Relaxed);
        let bytes_sent = stats_arc.bytes_sent.load(Ordering::Relaxed);
        let missed_pkgs = stats_arc.missed_pkgs.load(Ordering::Relaxed);
        let elapsed = stats_arc.get_elapsed();

        let pps = if elapsed > 0.0 {
            packets_sent as f64 / elapsed
        } else {
            0.0
        };
        let success_rate = if packets_sent + missed_pkgs > 0 {
            (packets_sent as f64 / (packets_sent + missed_pkgs) as f64) * 100.0
        } else {
            0.0
        };

        let mut rows = Vec::new();

        let mode_str = format!("{:?}", app.config.mode);
        let target_str = format!("{}:{}", app.config.target, app.config.port);
        let config_str = format!("{} threads, {}s", app.config.threads, app.config.duration);
        let packets_str = format_number(packets_sent as f64);
        let bytes_str = format_bytes(bytes_sent);
        let pps_str = format_pps(pps as u64);
        let peak_bw_str = format_bytes(stats_arc.peak_bandwidth.load(Ordering::Relaxed));
        let success_str = format!("{:.2}%", success_rate);
        let failed_str = format_number(missed_pkgs as f64);

        rows.push(Row::new(vec![
            "Attack Mode",
            &mode_str,
            "",
            ""
        ]).style(Style::default().fg(theme.text_normal)));

        rows.push(Row::new(vec![
            "Target",
            &target_str,
            "",
            ""
        ]).style(Style::default().fg(theme.text_normal)));

        rows.push(Row::new(vec![
            "Configuration",
            &config_str,
            "",
            ""
        ]).style(Style::default().fg(theme.text_normal)));

        rows.push(Row::new(vec!["", "", "", ""]).bottom_margin(1));

        rows.push(Row::new(vec![
            "Packets Sent",
            &packets_str,
            "Bytes Sent",
            &bytes_str,
        ]).style(Style::default().fg(theme.green)));

        rows.push(Row::new(vec![
            "Average PPS",
            &pps_str,
            "Peak Bandwidth",
            &peak_bw_str,
        ]).style(Style::default().fg(theme.cyan)));

        rows.push(Row::new(vec![
            "Success Rate",
            &success_str,
            "Failed Packets",
            &failed_str,
        ]).style(Style::default().fg(if success_rate > 90.0 { theme.green } else { theme.yellow })));

        let table = Table::new(rows, [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(25),
        ])
        .column_spacing(1)
        .style(Style::default().fg(theme.text_normal));

        let table_area = Rect {
            x: details_area.x + 2,
            y: details_area.y + 2,
            width: details_area.width - 4,
            height: details_area.height - 4,
        };

        f.render_widget(table, table_area);

    }
}

fn draw_config_screen(f: &mut Frame, app: &App, theme: &Theme) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(7),
            Constraint::Length(2),
        ])
        .split(f.size());

    draw_header(
        f,
        layout[0],
        "Setup",
        "Assemble the attack profile step by step",
        theme,
    );

    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(34), Constraint::Min(46)])
        .split(layout[1]);

    draw_config_navigation(f, body[0], app, theme);
    draw_config_panel(f, body[1], app, theme);

    draw_logs(f, layout[2], app, "Event Log", theme);

    draw_cool_footer(f, layout[3], theme);
}


fn draw_config_navigation(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let items: Vec<ListItem> = CONFIG_SECTIONS
        .iter()
        .enumerate()
        .map(|(idx, (title, _description, _))| {
            let prefix = if app.section_active && idx == app.selected_section {
                "▼"
            } else if idx == app.selected_section {
                "▶"
            } else {
                " "
            };
            let style = if idx == app.selected_section {
                if app.section_active {
                    Style::default()
                        .fg(theme.cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                        .fg(theme.text_bright)
                        .add_modifier(Modifier::BOLD)
                }
            } else {
                Style::default().fg(theme.text_normal)
            };
            ListItem::new(Line::from(vec![Span::styled(
                format!("{} {}", prefix, title),
                style,
            )]))
        })
        .collect();
    let block = Block::default()
        .title(" Navigation ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg_float))
        .padding(ratatui::widgets::Padding {
            left: 1,
            right: 1,
            top: 1,
            bottom: 1,
        });

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(theme.bg_main)
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("");

    f.render_widget(list, area);
}

fn draw_config_panel(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Min(8),
            Constraint::Length(4),
        ])
        .split(area);

    draw_target_status(f, sections[0], app, theme);
    draw_config_form(f, sections[1], app, theme);
    draw_field_help(f, sections[2], app, theme);
}
fn draw_config_form(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let section_idx = app.selected_section;
    let (section_title, _, fields) = CONFIG_SECTIONS[section_idx];

    let mut rows = Vec::new();

    if app.section_active {
        rows.push(Line::from(vec![Span::styled(
            format!("{}", section_title),
            Style::default()
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        rows.push(Line::from(Span::styled(
            "─".repeat(30),
            Style::default().fg(theme.border),
        )));

        for field in fields.iter() {
            let selected = app.selected_field == *field;
            let marker = if selected { "❯" } else { " " };
            let label_style = if selected {
                Style::default()
                    .fg(theme.text_bright)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme.text_normal)
            };

            let mut value = app.get_field_value(field);
            if value.len() > 38 {
                value.truncate(37);
                value.push('…');
            }

            let (value_text, value_style) = styled_value(field, &value, selected, app.input_mode, theme);

            rows.push(Line::from(vec![
                Span::styled(
                    format!("{} {}", marker, field_label(field)),
                    label_style,
                ),
                Span::raw(": "),
                Span::styled(value_text, value_style),
            ]));
        }
    } else {
        rows.push(Line::from(vec![Span::styled(
            "Select a section to configure",
            Style::default().fg(theme.text_dim),
        )]));
        rows.push(Line::from(""));
        rows.push(Line::from(vec![Span::styled(
            format!("{}: {}", section_title, CONFIG_SECTIONS[section_idx].1),
            Style::default().fg(theme.text_normal),
        )]));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg_float))
        .padding(ratatui::widgets::Padding {
            left: 2,
            right: 2,
            top: 1,
            bottom: 1,
        });

    let paragraph = Paragraph::new(rows).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn styled_value(field: &ConfigField, raw: &str, selected: bool, editing: bool, theme: &Theme) -> (String, Style) {
    let highlight = if editing {
        Style::default()
            .fg(theme.bg_dark)
            .bg(theme.cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.text_bright)
            .add_modifier(Modifier::BOLD)
    };

    match field {
        ConfigField::RandomPayload
        | ConfigField::RandomPorts
        | ConfigField::SecondaryAttack
        | ConfigField::RotateUserAgent => {
            let enabled = raw.eq_ignore_ascii_case("yes") || raw.eq_ignore_ascii_case("on");
            let text = if enabled { "Enabled" } else { "Disabled" }.to_string();
            let mut style = if enabled {
                Style::default().fg(theme.green)
            } else {
                Style::default().fg(theme.text_dim)
            };
            if selected {
                style = if editing {
                    highlight
                } else {
                    style.add_modifier(Modifier::BOLD)
                };
            }
            (text, style)
        }
        ConfigField::CustomPayload => {
            let mut style = Style::default().fg(theme.text_normal);
            if selected {
                style = if editing {
                    highlight
                } else {
                    style.add_modifier(Modifier::BOLD)
                };
            }
            (
                if raw.is_empty() {
                    "<empty>".to_string()
                } else {
                    raw.to_string()
                },
                style,
            )
        }
        _ => {
            let mut style = Style::default().fg(theme.text_bright);
            if selected {
                style = if editing {
                    highlight
                } else {
                    style.add_modifier(Modifier::BOLD)
                };
            }
            (raw.to_string(), style)
        }
    }
}

fn draw_field_help(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let help_text = if app.section_active {
        field_hint(&app.selected_field)
    } else {
        "Navigate to a section and press ENTER to edit its settings"
    };

    let help = Paragraph::new(vec![Line::from(vec![Span::styled(
        help_text,
        Style::default().fg(theme.text_dim),
    )])])
    .block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg_float))
            .padding(ratatui::widgets::Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            }),
    )
    .wrap(Wrap { trim: true });

    f.render_widget(help, area);
}

fn draw_attack_screen(f: &mut Frame, app: &App, theme: &Theme) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(4),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Min(8),
            Constraint::Length(2),
        ])
        .split(f.size());

    draw_header(
        f,
        layout[0],
        "Attack",
        "Monitoring live throughput and health",
        theme,
    );
    draw_attack_status(f, layout[1], app, theme);
    draw_attack_metrics(f, layout[2], app, theme);
    draw_network_visualization(f, layout[3], app, theme);
    draw_attack_activity(f, layout[4], app, theme);
    draw_cool_footer(f, layout[5], theme);
}


fn draw_attack_status(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let elapsed = app.stats.get_elapsed();
    let status_line = Line::from(vec![
        Span::styled(
            "Running",
            Style::default()
                .fg(theme.red)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" · Target: "),
        Span::styled(
            format!("{}:{}", app.config.target, app.config.port),
            Style::default().fg(theme.text_bright),
        ),
        Span::raw(" · Mode: "),
        Span::styled(
            app.config.mode.to_string(),
            Style::default().fg(theme.blue),
        ),
        Span::raw(" · Elapsed: "),
        Span::styled(
            format!("{:.1}s", elapsed),
            Style::default().fg(theme.green),
        ),
    ]);

    let paragraph = Paragraph::new(vec![status_line])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg_float))
                .padding(ratatui::widgets::Padding {
                    left: 1,
                    right: 1,
                    top: 0,
                    bottom: 0,
                }),
        )
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(paragraph, area);
}

fn draw_attack_metrics(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let metrics_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let packets_sent = app.stats.packets_sent.load(Ordering::Relaxed);
    let bytes_sent = app.stats.bytes_sent.load(Ordering::Relaxed);
    let missed_pkgs = app.stats.missed_pkgs.load(Ordering::Relaxed);
    let elapsed = app.stats.get_elapsed();

    let pps = if elapsed > 0.0 {
        packets_sent as f64 / elapsed
    } else {
        0.0
    };
    let mbps = if elapsed > 0.0 {
        (bytes_sent as f64 * 8.0) / (elapsed * 1_000_000.0)
    } else {
        0.0
    };
    let success_rate = if packets_sent + missed_pkgs > 0 {
        (packets_sent as f64 / (packets_sent + missed_pkgs) as f64) * 100.0
    } else {
        0.0
    };

    let left_lines = vec![
        Line::from(vec![Span::styled(
            "Traffic Metrics",
            Style::default()
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            "─".repeat(20),
            Style::default().fg(theme.border),
        )),
        Line::from(vec![
            Span::styled(
                "Packets Sent",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(": "),
            Span::styled(
                packets_sent.to_string(),
                Style::default().fg(theme.text_bright),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Data Transmitted",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(": "),
            Span::styled(
                format!("{:.2} MB", bytes_sent as f64 / 1_000_000.0),
                Style::default().fg(theme.text_bright),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Failed Packets",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(": "),
            Span::styled(
                missed_pkgs.to_string(),
                Style::default().fg(theme.red),
            ),
        ]),
    ];

    let right_lines = vec![
        Line::from(vec![Span::styled(
            "Performance",
            Style::default()
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            "─".repeat(20),
            Style::default().fg(theme.border),
        )),
        Line::from(vec![
            Span::styled(
                "Current PPS",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(": "),
            Span::styled(
                format!("{:.0}", pps),
                Style::default().fg(theme.blue),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Avg Bandwidth",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(": "),
            Span::styled(
                format!("{:.2} Mbps", mbps),
                Style::default().fg(theme.magenta),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Success Rate",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(": "),
            Span::styled(
                format!("{:.1}%", success_rate),
                Style::default().fg(theme.green),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "Peak Bandwidth",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(": "),
            Span::styled(
                format!("{:.2} Mbps", app.stats.get_peak_bandwidth()),
                Style::default().fg(theme.orange),
            ),
        ]),
    ];

    let left = Paragraph::new(left_lines)
        .block(
            Block::default()
                .title(" Throughput ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg_float))
                .padding(ratatui::widgets::Padding {
                    left: 1,
                    right: 1,
                    top: 1,
                    bottom: 1,
                }),
        )
        .wrap(Wrap { trim: true });

    let right = Paragraph::new(right_lines)
        .block(
            Block::default()
                .title(" Efficiency ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg_float))
                .padding(ratatui::widgets::Padding {
                    left: 1,
                    right: 1,
                    top: 1,
                    bottom: 1,
                }),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(left, metrics_layout[0]);
    f.render_widget(right, metrics_layout[1]);
}

fn draw_attack_activity(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(48), Constraint::Min(32)])
        .split(area);

    draw_packet_capture(f, layout[0], app, theme);
    draw_logs(f, layout[1], app, "Live Log", theme);
}

fn draw_packet_capture(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let packets = app.stats.get_packet_capture();
    let mut items = Vec::new();

    for packet in packets
        .iter()
        .rev()
        .take((area.height.saturating_sub(2)) as usize)
    {
        let elapsed = packet.timestamp.elapsed().as_millis();
        let status = if packet.success { "✓" } else { "✗" };
        let status_color = if packet.success {
            theme.green
        } else {
            theme.red
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(status, Style::default().fg(status_color)),
            Span::raw(" "),
            Span::styled(
                format!("{}:{}", packet.target, packet.port),
                Style::default().fg(theme.text_bright),
            ),
            Span::raw(" · "),
            Span::styled(
                format!("{}B", packet.size),
                Style::default().fg(theme.blue),
            ),
            Span::raw(" · "),
            Span::styled(
                format!("{}ms", elapsed),
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(" · "),
            Span::styled(
                packet.protocol.clone(),
                Style::default().fg(theme.magenta),
            ),
        ])));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "Waiting for packet activity...",
            Style::default().fg(theme.text_dim),
        )])));
    }

    let list = List::new(items).block(
        Block::default()
            .title(" Packet Capture ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.border))
            .style(Style::default().bg(theme.bg_float))
            .padding(ratatui::widgets::Padding {
                left: 1,
                right: 1,
                top: 0,
                bottom: 0,
            }),
    );

    f.render_widget(list, area);
}

fn draw_network_visualization(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let activity = app.stats.get_network_activity();

    if activity.is_empty() {
        let no_data = Paragraph::new("No network activity data")
            .style(Style::default().fg(theme.text_dim))
            .alignment(ratatui::layout::Alignment::Center)
            .block(Block::default()
                .title(" Network Traffic ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg_float)));

        f.render_widget(no_data, area);
        return;
    }

    // datasets for visualization
    let data_points: Vec<(f64, f64)> = activity.iter()
        .map(|(time, bytes)| (*time, *bytes as f64))
        .collect();

    let datasets = vec![
        Dataset::default()
            .name("Bytes/s")
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(theme.cyan))
            .data(&data_points),
    ];

    let max_bytes = activity.iter()
        .map(|(_, bytes)| *bytes as f64)
        .max_by(|a, b| a.partial_cmp(b).unwrap())
        .unwrap_or(1000.0);

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(" Network Traffic (Bytes/s) ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg_float)),
        )
        .x_axis(
            Axis::default()
                .title("Time (seconds ago)")
                .style(Style::default().fg(theme.text_dim))
                .bounds([-60.0, 0.0])
                .labels(vec![
                    Span::styled("-60", Style::default().fg(theme.text_dim)),
                    Span::styled("-30", Style::default().fg(theme.text_dim)),
                    Span::styled("0", Style::default().fg(theme.text_dim)),
                ]),
        )
        .y_axis(
            Axis::default()
                .style(Style::default().fg(theme.text_dim))
                .bounds([0.0, max_bytes * 1.1]),
        );

    f.render_widget(chart, area);
}

fn draw_results_screen(f: &mut Frame, app: &App, theme: &Theme) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(12),
            Constraint::Length(2),
        ])
        .split(f.size());

    draw_header(
        f,
        layout[0],
        "Results",
        "Summary of the last attack session",
        theme,
    );

    let packets_sent = app.stats.packets_sent.load(Ordering::Relaxed);
    let bytes_sent = app.stats.bytes_sent.load(Ordering::Relaxed);
    let missed_pkgs = app.stats.missed_pkgs.load(Ordering::Relaxed);
    let elapsed = app.stats.get_elapsed();
    let success_rate = if packets_sent + missed_pkgs > 0 {
        (packets_sent as f64 / (packets_sent + missed_pkgs) as f64) * 100.0
    } else {
        0.0
    };

    let summary = vec![
        Line::from(vec![Span::styled(
            "Target Information",
            Style::default()
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            "─".repeat(30),
            Style::default().fg(theme.border),
        )),
        Line::from(vec![
            Span::styled("Endpoint: ", Style::default().fg(theme.text_dim)),
            Span::raw(format!("{}:{}", app.config.target, app.config.port)),
        ]),
        Line::from(vec![
            Span::styled("Mode: ", Style::default().fg(theme.text_dim)),
            Span::raw(app.config.mode.to_string()),
        ]),
        Line::from(vec![
            Span::styled("Duration: ", Style::default().fg(theme.text_dim)),
            Span::raw(format!("{:.2} s", elapsed)),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Attack Summary",
            Style::default()
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(Span::styled(
            "─".repeat(30),
            Style::default().fg(theme.border),
        )),
        Line::from(vec![
            Span::styled(
                "Packets Sent: ",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(packets_sent.to_string()),
        ]),
        Line::from(vec![
            Span::styled(
                "Data Sent: ",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(format!("{:.2} MB", bytes_sent as f64 / 1_000_000.0)),
        ]),
        Line::from(vec![
            Span::styled(
                "Failed Packets: ",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(missed_pkgs.to_string()),
        ]),
        Line::from(vec![
            Span::styled(
                "Success Rate: ",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(format!("{:.1}%", success_rate)),
        ]),
        Line::from(vec![
            Span::styled(
                "Average PPS: ",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(format!(
                "{:.1}",
                if elapsed > 0.0 {
                    packets_sent as f64 / elapsed
                } else {
                    0.0
                }
            )),
        ]),
        Line::from(vec![
            Span::styled(
                "Average Bandwidth: ",
                Style::default().fg(theme.text_dim),
            ),
            Span::raw(format!(
                "{:.2} Mbps",
                if elapsed > 0.0 {
                    (bytes_sent as f64 * 8.0) / (elapsed * 1_000_000.0)
                } else {
                    0.0
                }
            )),
        ]),
    ];

    let paragraph = Paragraph::new(summary)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .style(Style::default().bg(theme.bg_float))
                .padding(ratatui::widgets::Padding {
                    left: 2,
                    right: 2,
                    top: 1,
                    bottom: 1,
                }),
        )
        .wrap(Wrap { trim: true });

    f.render_widget(paragraph, layout[1]);

    draw_cool_footer(f, layout[2], theme);
}

fn draw_logs(f: &mut Frame, area: Rect, app: &App, title: &str, theme: &Theme) {
    let logs = app.get_logs();
    let capacity = area.height.saturating_sub(2) as usize;
    let mut items: Vec<ListItem> = logs
        .iter()
        .rev()
        .take(capacity)
        .map(|entry| {
            ListItem::new(Line::from(vec![Span::styled(
                entry.clone(),
                Style::default().fg(theme.text_dim),
            )]))
        })
        .collect();

    if items.is_empty() {
        items.push(ListItem::new(Line::from(vec![Span::styled(
            "No events recorded yet.",
            Style::default().fg(theme.text_dim),
        )])));
    }

    let block = Block::default()
        .title(format!(" {} ", title))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg_float))
        .padding(ratatui::widgets::Padding {
            left: 1,
            right: 1,
            top: 0,
            bottom: 0,
        });

    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_target_status(f: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    let mut is_online = false;
    let mut response_time = 0.0;
    let mut open_ports = 0;
    let mut slowing_down = false;
    let mut resolved_ip = None;
    let mut location = None;

    if let Some(stats_arc) = &app.stats_arc {
        let status = stats_arc.target_status.lock().unwrap();
        is_online = status.is_online;
        response_time = status.response_time_ms;
        open_ports = status.open_ports.len();
        slowing_down = status.is_degraded;
        resolved_ip = status.resolved_ip.clone();
        location = status
            .country
            .clone()
            .map(|country| match status.city.clone() {
                Some(city) => format!("{}, {}", city, country),
                None => country,
            });
    }

    let reachability = if is_online { "Online" } else { "Offline" };
    let reach_style = if is_online {
        Style::default()
            .fg(theme.green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(theme.red)
            .add_modifier(Modifier::BOLD)
    };

    let lines = vec![
        Line::from(vec![
            Span::styled("Target", Style::default().fg(theme.text_dim)),
            Span::raw(": "),
            Span::styled(
                format!("{}:{}", app.config.target, app.config.port),
                Style::default().fg(theme.text_bright),
            ),
            Span::raw(" · "),
            Span::styled(reachability, reach_style),
        ]),
        Line::from(vec![
            Span::styled("Latency", Style::default().fg(theme.text_dim)),
            Span::raw(": "),
            Span::styled(
                if response_time > 0.0 {
                    format!("{:.0} ms", response_time)
                } else {
                    "n/a".to_string()
                },
                Style::default().fg(theme.text_bright),
            ),
            Span::raw(" · "),
            Span::styled(
                format_health_label(response_time, slowing_down),
                Style::default().fg(format_health_color(response_time, slowing_down, theme)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Open Ports", Style::default().fg(theme.text_dim)),
            Span::raw(": "),
            Span::styled(
                open_ports.to_string(),
                Style::default().fg(theme.text_bright),
            ),
        ]),
        Line::from(vec![
            Span::styled("Resolved", Style::default().fg(theme.text_dim)),
            Span::raw(": "),
            Span::styled(
                resolved_ip.unwrap_or_else(|| "Pending DNS lookup".to_string()),
                Style::default().fg(theme.text_normal),
            ),
        ]),
        Line::from(vec![
            Span::styled("Location", Style::default().fg(theme.text_dim)),
            Span::raw(": "),
            Span::styled(
                location.unwrap_or_else(|| "Unknown".to_string()),
                Style::default().fg(theme.text_normal),
            ),
        ]),
    ];

    let block = Block::default()
        .title(" Target Information ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border))
        .style(Style::default().bg(theme.bg_float))
        .padding(ratatui::widgets::Padding {
            left: 1,
            right: 1,
            top: 1,
            bottom: 1,
        });

    f.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_header(f: &mut Frame, area: Rect, title: &str, subtitle: &str, theme: &Theme) {
    let block = Block::default().style(Style::default().bg(theme.bg_main));
    f.render_widget(block, area);

    let content = Paragraph::new(vec![
        Line::from(vec![
            Span::styled(
                " Skibidi Rizz ",
                Style::default()
                    .fg(theme.cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  >>  "),
            Span::styled(
                title,
                Style::default()
                    .fg(theme.text_bright)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![Span::styled(
            subtitle,
            Style::default().fg(theme.text_dim),
        )]),
    ])
    .alignment(ratatui::layout::Alignment::Left)
    .block(Block::default().padding(ratatui::widgets::Padding {
        left: 2,
        right: 0,
        top: 0,
        bottom: 0,
    }));

    f.render_widget(content, area);
}

fn draw_cool_footer(f: &mut Frame, area: Rect, theme: &Theme) {
    let text = Line::from(vec![
        Span::raw("Press "),
        Span::styled(
            "Ctrl + /",
            Style::default()
                .fg(theme.cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" for keyboard shortcuts"),
    ]);

    let paragraph = Paragraph::new(text)
        .block(
            Block::default()
                .style(Style::default().bg(theme.bg_main))
                .padding(ratatui::widgets::Padding {
                    left: 2,
                    right: 2,
                    top: 0,
                    bottom: 0,
                }),
        )
        .alignment(ratatui::layout::Alignment::Center);

    f.render_widget(paragraph, area);
}

fn field_label(field: &ConfigField) -> &'static str {
    match field {
        ConfigField::Target => "Target",
        ConfigField::Port => "Port",
        ConfigField::Threads => "Threads",
        ConfigField::Rate => "Packets/s",
        ConfigField::Duration => "Duration",
        ConfigField::PacketSize => "Packet Size",
        ConfigField::Mode => "Attack Mode",
        ConfigField::CustomPayload => "Custom Payload",
        ConfigField::RandomPayload => "Random Payload",
        ConfigField::RandomPorts => "Random Ports",
        ConfigField::EvasMode => "Evasion Mode",
        ConfigField::SizeStrategy => "Size Strategy",
        ConfigField::SecondaryAttack => "Multi-Vector",
        ConfigField::VariancePercentage => "Variance %",
        ConfigField::BurstSize => "Burst Size",
        ConfigField::RotateUserAgent => "Rotate UA",
        ConfigField::Preset => "Preset",
        ConfigField::Theme => "Theme",
        ConfigField::RpcEnabled => "Discord RPC",
        ConfigField::AutoSave => "Auto Save",
    }
}

fn field_hint(field: &ConfigField) -> &'static str {
    match field {
        ConfigField::Target => "Hostname or IP address to direct traffic towards.",
        ConfigField::Port => {
            "Service port on the target. Combine with random port mode for rotation."
        }
        ConfigField::Threads => {
            "Number of asynchronous workers that will emit packets in parallel."
        }
        ConfigField::Rate => "Desired packets-per-second budget across all workers.",
        ConfigField::Duration => "Total attack runtime in seconds before stopping automatically.",
        ConfigField::PacketSize => "Size of each packet in bytes after payload padding.",
        ConfigField::Mode => "Protocol flavour and technique to apply for this run.",
        ConfigField::CustomPayload => {
            "Optional raw payload appended to each packet before padding."
        }
        ConfigField::RandomPayload => {
            "Fill each payload with random bytes instead of deterministic data."
        }
        ConfigField::RandomPorts => "Rotate destination ports to evade basic filtering.",
        ConfigField::EvasMode => {
            "Timing profile used to stagger packets (fixed, random, adaptive, etc.)."
        }
        ConfigField::SizeStrategy => {
            "How packet sizes evolve over time (fixed, random, oscillating)."
        }
        ConfigField::SecondaryAttack => "Launch a secondary vector alongside the primary mode.",
        ConfigField::VariancePercentage => "Percentage of timing jitter injected for evasion.",
        ConfigField::BurstSize => "Packets fired per burst when burst logic is enabled.",
        ConfigField::RotateUserAgent => "Cycle through HTTP User-Agent strings for L7 modes.",
        ConfigField::Preset => "Quick configuration templates for common scenarios.",
        ConfigField::Theme => "Choose the color scheme for the interface.",
        ConfigField::RpcEnabled => "Show Discord rich presence when running.",
        ConfigField::AutoSave => "Automatically save configuration on exit.",
    }
}

fn format_health_label(response_time: f64, slowing_down: bool) -> &'static str {
    if slowing_down {
        "Degraded"
    } else if response_time <= 0.0 {
        "Unknown"
    } else if response_time < 50.0 {
        "Excellent"
    } else if response_time < 120.0 {
        "Stable"
    } else if response_time < 200.0 {
        "Strained"
    } else {
        "Critical"
    }
}

fn format_health_color(response_time: f64, slowing_down: bool, theme: &Theme) -> Color {
    if slowing_down {
        theme.orange
    } else if response_time <= 0.0 {
        theme.text_dim
    } else if response_time < 50.0 {
        theme.green
    } else if response_time < 120.0 {
        theme.yellow
    } else if response_time < 200.0 {
        theme.orange
    } else {
        theme.red
    }
}

fn format_number(value: f64) -> String {
    if value >= 10_000.0 {
        format!("{:.0}k", value / 1_000.0)
    } else if value >= 1_000.0 {
        format!("{:.1}k", value / 1_000.0)
    } else {
        format!("{:.0}", value)
    }
}

fn format_pps(pps: u64) -> String {
    if pps >= 1_000_000 {
        format!("{:.1}M", pps as f64 / 1_000_000.0)
    } else if pps >= 1_000 {
        format!("{:.1}k", pps as f64 / 1_000.0)
    } else {
        format!("{:.0}", pps)
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1_000_000_000 {
        format!("{:.2} GB", bytes as f64 / 1_000_000_000.0)
    } else if bytes >= 1_000_000 {
        format!("{:.2} MB", bytes as f64 / 1_000_000.0)
    } else if bytes >= 1_000 {
        format!("{:.2} KB", bytes as f64 / 1_000.0)
    } else {
        format!("{:.0} B", bytes)
    }
}

pub fn draw_cheat_sheet(f: &mut Frame, _app: &App, theme: &Theme) {
    let area = centered_rect(80, 90, f.size());

    let modal = Block::default()
        .title(" Keyboard Shortcuts ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.cyan))
        .style(Style::default().bg(theme.bg_main))
        .padding(ratatui::widgets::Padding {
            left: 4,
            right: 4,
            top: 2,
            bottom: 2,
        });

    let sections = [
        ("Navigation", vec![
            ("↑/↓", "Navigate sections/fields"),
            ("Enter", "Select section/Edit field"),
            ("Esc", "Go back/Cancel"),
            ("Tab", "Next field"),
        ]),
        ("Configuration", vec![
            ("Space", "Toggle boolean fields"),
            ("F1", "Launch attack"),
            ("F2", "Stop attack"),
            ("F3", "Show results"),
        ]),
        ("Configuration", vec![
            ("F6", "Save configuration"),
            ("F7", "Load configuration"),
        ]),
        ("General", vec![
            ("Ctrl + /", "Show/hide this cheat sheet"),
            ("Q", "Quit application"),
        ]),
    ];

    let mut content = Vec::new();

    // Add header
    content.push(Line::from(Span::styled(
        "Quick Reference",
        Style::default()
            .fg(theme.cyan)
            .add_modifier(Modifier::BOLD),
    )));
    content.push(Line::from(""));

    // Add sections
    for (section_name, shortcuts) in sections.iter() {
        content.push(Line::from(Span::styled(
            format!("{}:", section_name),
            Style::default()
                .fg(theme.text_bright)
                .add_modifier(Modifier::BOLD),
        )));

        for (key, desc) in shortcuts {
            content.push(Line::from(vec![
                Span::styled(
                    format!("  {:<12}", key),
                    Style::default()
                        .fg(theme.cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(desc.to_string(), Style::default().fg(theme.text_normal)),
            ]));
        }
        content.push(Line::from(""));
    }

    // Add footer
    content.push(Line::from(Span::styled(
        "Press any key to close",
        Style::default().fg(theme.text_dim),
    )));

    let paragraph = Paragraph::new(content)
        .block(modal)
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: true });

    f.render_widget(Clear, area); // Clear the area
    f.render_widget(paragraph, area);
}


