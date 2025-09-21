use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use std::sync::Arc;
use tokio::sync::Mutex;

#[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
use pnet::datalink::Channel::Ethernet;

#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub description: String,
    pub mac: Option<String>,
    pub ips: Vec<String>,
    pub is_up: bool,
}

pub struct InterfaceSelector {
    interfaces: Vec<InterfaceInfo>,
    selected_index: usize,
    list_state: ListState,
    #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
    test_results: Arc<Mutex<Vec<(usize, String)>>>,
}

impl InterfaceSelector {
    pub fn new() -> Self {
        let interfaces = Self::get_available_interfaces();
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            interfaces,
            selected_index: 0,
            list_state,
            #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
            test_results: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn get_available_interfaces() -> Vec<InterfaceInfo> {
        #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
        {
            let ifaces = pnet::datalink::interfaces();
            ifaces
                .into_iter()
                .map(|iface| InterfaceInfo {
                    name: iface.name.clone(),
                    description: iface.description.clone(),
                    mac: iface.mac.map(|mac| mac.to_string()),
                    ips: iface.ips.iter().map(|ip| ip.to_string()).collect(),
                    is_up: iface.is_up(),
                })
                .collect()
        }

        #[cfg(not(all(target_os = "windows", feature = "pnet_datalink")))]
        {
            vec![InterfaceInfo {
                name: "eth0".to_string(),
                description: "Default Ethernet Interface".to_string(),
                mac: None,
                ips: vec!["127.0.0.1".to_string()],
                is_up: true,
            }]
        }
    }

    pub fn selected_interface(&self) -> Option<&InterfaceInfo> {
        self.interfaces.get(self.selected_index)
    }

    pub fn handle_event(&mut self, event: Event) -> Option<InterfaceEvent> {
        match event {
            Event::Key(key_event) => self.handle_key_event(key_event),
            Event::Mouse(mouse_event) => self.handle_mouse_event(mouse_event),
            _ => None,
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<InterfaceEvent> {
        match key.code {
            KeyCode::Up => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.list_state.select(Some(self.selected_index));
                }
                None
            }
            KeyCode::Down => {
                if self.selected_index < self.interfaces.len().saturating_sub(1) {
                    self.selected_index += 1;
                    self.list_state.select(Some(self.selected_index));
                }
                None
            }
            KeyCode::Enter => Some(InterfaceEvent::Select(self.selected_index)),
            KeyCode::Char('t') | KeyCode::Char('T') => {
                #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
                {
                    self.test_interface(self.selected_index);
                }
                None
            }
            KeyCode::Esc => Some(InterfaceEvent::Cancel),
            _ => None,
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Option<InterfaceEvent> {
        if mouse.kind == MouseEventKind::ScrollUp {
            if self.selected_index > 0 {
                self.selected_index -= 1;
                self.list_state.select(Some(self.selected_index));
            }
        } else if mouse.kind == MouseEventKind::ScrollDown {
            if self.selected_index < self.interfaces.len().saturating_sub(1) {
                self.selected_index += 1;
                self.list_state.select(Some(self.selected_index));
            }
        } else if let MouseEventKind::Down(button) = mouse.kind {
            let _item_height = 1;
            let click_y = mouse.row as usize;

            if click_y >= 8 {
                let item_index = click_y - 8;
                if item_index < self.interfaces.len() {
                    self.selected_index = item_index;
                    self.list_state.select(Some(self.selected_index));

                    if button == crossterm::event::MouseButton::Left {
                        return Some(InterfaceEvent::Select(self.selected_index));
                    }
                }
            }
        }
        None
    }

    #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
    fn test_interface(&self, index: usize) {
        if let Some(iface) = self.interfaces.get(index) {
            let test_results = self.test_results.clone();
            let iface_name = iface.name.clone();

            tokio::spawn(async move {
                let mut results = test_results.lock().await;
                results.push((index, "Testing...".to_string()));

                match pnet::datalink::interfaces()
                    .into_iter()
                    .find(|i| i.name == iface_name)
                {
                    Some(network_interface) => {
                        match pnet::datalink::channel(&network_interface, Default::default()) {
                            Ok(Ethernet(_tx, _rx)) => {
                                results.retain(|(i, _)| *i != index);
                                results.push((index, "✓ Interface ready".to_string()));
                            }
                            Ok(_) => {
                                results.retain(|(i, _)| *i != index);
                                results.push((index, "⚠ Not Ethernet".to_string()));
                            }
                            Err(e) => {
                                results.retain(|(i, _)| *i != index);
                                results.push((index, format!("✗ Error: {}", e)));
                            }
                        }
                    }
                    None => {
                        results.retain(|(i, _)| *i != index);
                        results.push((index, "✗ Interface not found".to_string()));
                    }
                }
            });
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        let block = Block::default()
            .title(" Select Network Interface ")
            .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray))
            .border_type(BorderType::Rounded)
            .style(Style::default().bg(Color::Rgb(20, 20, 30)));

        let inner_area = block.inner(area);
        f.render_widget(Clear, inner_area);
        f.render_widget(block, area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),  // Instructions
                Constraint::Length(3),  // Header
                Constraint::Min(5),     // List
                Constraint::Length(3),  // Selected info
                Constraint::Length(2),  // Buttons
            ])
            .split(inner_area);

        // Instructions
        let instructions = Text::from(vec![
            Line::from(Span::styled(
                "Use ↑↓ to navigate, Enter to select, T to test, Esc to cancel",
                Style::default().fg(Color::DarkGray),
            ))
        ]);
        f.render_widget(Paragraph::new(instructions), chunks[0]);

        // Header
        let header = vec![
            Line::from(vec![
                Span::styled("Interface", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("Description", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("  "),
                Span::styled("Status", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            ])
        ];
        f.render_widget(Paragraph::new(Text::from(header)), chunks[1]);

        // List
        let items: Vec<ListItem> = self.interfaces
            .iter()
            .enumerate()
            .map(|(i, iface)| {
                let status = if iface.is_up {
                    Span::styled("UP", Style::default().fg(Color::Green))
                } else {
                    Span::styled("DOWN", Style::default().fg(Color::Red))
                };

                let content = Line::from(vec![
                    Span::styled(
                        format!("{:<15}", iface.name),
                        if i == self.selected_index {
                            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default()
                        },
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<30}", iface.description),
                        Style::default().fg(Color::Gray),
                    ),
                    Span::raw("  "),
                    status,
                ]);

                ListItem::new(content)
            })
            .collect();

        let list = List::new(items)
            .highlight_style(Style::default().bg(Color::Rgb(50, 50, 70)).add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        f.render_stateful_widget(list, chunks[2], &mut self.list_state);

        // Selected interface info
        if let Some(selected) = self.selected_interface() {
            let mut info_lines = vec![
                Line::from(vec![
                    Span::styled("Name: ", Style::default().fg(Color::Cyan)),
                    Span::raw(&selected.name),
                ]),
                Line::from(vec![
                    Span::styled("MAC: ", Style::default().fg(Color::Cyan)),
                    Span::raw(selected.mac.as_deref().unwrap_or("N/A")),
                ]),
                Line::from(vec![
                    Span::styled("IPs: ", Style::default().fg(Color::Cyan)),
                    Span::raw(selected.ips.join(", ")),
                ]),
            ];

            #[cfg(all(target_os = "windows", feature = "pnet_datalink"))]
            {
                let test_result = {
                    let test_results = self.test_results.blocking_lock();
                    test_results.iter().find(|(i, _)| *i == self.selected_index).cloned()
                };

                if let Some((_, result)) = test_result {
                    info_lines.push(Line::from(vec![
                        Span::styled("Test: ", Style::default().fg(Color::Cyan)),
                        Span::styled(
                            result.clone(),
                            if result.contains("✓") {
                                Style::default().fg(Color::Green)
                            } else if result.contains("⚠") {
                                Style::default().fg(Color::Yellow)
                            } else {
                                Style::default().fg(Color::Red)
                            },
                        ),
                    ]));
                }
            }

            let info_block = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray));

            let info_para = Paragraph::new(Text::from(info_lines))
                .block(info_block)
                .wrap(Wrap { trim: true });

            f.render_widget(info_para, chunks[3]);
        }

        // Buttons
        let button_style = Style::default().bg(Color::Blue).fg(Color::White);
        let button_text = vec![
            Span::raw(" [Enter] Select "),
            Span::raw(" "),
            Span::raw(" [T] Test "),
            Span::raw(" "),
            Span::raw(" [Esc] Cancel "),
        ];
        let buttons = Paragraph::new(Line::from(button_text))
            .style(button_style)
            .alignment(Alignment::Center);
        f.render_widget(buttons, chunks[4]);
    }
}

#[derive(Debug, Clone)]
pub enum InterfaceEvent {
    Select(usize),
    Cancel,
}