use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind};

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum TutorialStep {
    Welcome,
    Navigation,
    Configuration,
    InterfaceSelection,
    AttackExecution,
    ResultsView,
    MouseSupport,
    Complete,
}

pub struct TutorialState {
    pub current_step: TutorialStep,
    pub list_state: ListState,
    pub completed_steps: Vec<TutorialStep>,
    pub is_active: bool,
}

impl TutorialState {
    pub fn new() -> Self {
        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Self {
            current_step: TutorialStep::Welcome,
            list_state,
            completed_steps: Vec::new(),
            is_active: false,
        }
    }

    pub fn start(&mut self) {
        self.is_active = true;
        self.current_step = TutorialStep::Welcome;
        self.completed_steps.clear();
        self.list_state.select(Some(0));
    }

    pub fn next_step(&mut self) {
        let steps = Self::get_steps();
        let current_index = steps.iter().position(|s| s == &self.current_step).unwrap_or(0);

        if !self.completed_steps.contains(&self.current_step) {
            self.completed_steps.push(self.current_step.clone());
        }

        if current_index < steps.len() - 1 {
            self.current_step = steps[current_index + 1];
            self.list_state.select(Some(current_index + 1));
        }
    }

    pub fn prev_step(&mut self) {
        let steps = Self::get_steps();
        let current_index = steps.iter().position(|s| s == &self.current_step).unwrap_or(0);

        if current_index > 0 {
            self.current_step = steps[current_index - 1];
            self.list_state.select(Some(current_index - 1));
        }
    }

    pub fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Right | KeyCode::Char('n') | KeyCode::Char('N') => {
                            self.next_step();
                            true
                        }
                        KeyCode::Left | KeyCode::Char('p') | KeyCode::Char('P') => {
                            self.prev_step();
                            true
                        }
                        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('Q') => {
                            self.is_active = false;
                            true
                        }
                        KeyCode::Enter => {
                            if self.current_step == TutorialStep::Complete {
                                self.is_active = false;
                                true
                            } else {
                                self.next_step();
                                true
                            }
                        }
                        _ => false,
                    }
                } else {
                    false
                }
            }
            Event::Mouse(mouse) => {
                if mouse.kind == MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                    // check for next button (right side)
                    if mouse.column >= 70 && mouse.column <= 85 && mouse.row >= 22 && mouse.row <= 24 {
                        self.next_step();
                        true
                    }
                    // check for prev button (left side)
                    else if mouse.column >= 5 && mouse.column <= 20 && mouse.row >= 22 && mouse.row <= 24 {
                        self.prev_step();
                        true
                    }
                    // check for close button
                    else if mouse.column >= 85 && mouse.column <= 95 && mouse.row >= 2 && mouse.row <= 4 {
                        self.is_active = false;
                        true
                    }
                    else {
                        false
                    }
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn get_steps() -> Vec<TutorialStep> {
        vec![
            TutorialStep::Welcome,
            TutorialStep::Navigation,
            TutorialStep::Configuration,
            TutorialStep::InterfaceSelection,
            TutorialStep::AttackExecution,
            TutorialStep::ResultsView,
            TutorialStep::MouseSupport,
            TutorialStep::Complete,
        ]
    }

    pub fn get_step_content(&self) -> (Vec<Line>, Vec<&'static str>) {
        match &self.current_step {
            TutorialStep::Welcome => (
                vec![
                    Line::from(Span::styled("Welcome to Skibidi Rizz tutorial~!", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("This tutorial will tell you through the main features"),
                    Line::from("of the Skibidi Rizz!"),
                    Line::from(""),
                    Line::from("Use ← → arrows or N/P keys to nav between steps."),
                    Line::from("Press ESC or Q to exit the tutorial at any time."),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
            TutorialStep::Navigation => (
                vec![
                    Line::from(Span::styled("Navigation", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("• Use ↑ ↓ arrow keys to nav between sections"),
                    Line::from("• Press ENTER to enter a section"),
                    Line::from("• Press ESC to exit a section"),
                    Line::from(""),
                    Line::from("The left panel shows all available configuration sections."),
                    Line::from("Each section contains related settings for your attack."),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
            TutorialStep::Configuration => (
                vec![
                    Line::from(Span::styled("Configuration", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("• Navigate fields with ↑ ↓ arrow keys"),
                    Line::from("• Press ENTER to edit a field"),
                    Line::from("• Press TAB to cycle through modes"),
                    Line::from("• Press SPACE to toggle boolean options"),
                    Line::from(""),
                    Line::from("Key fields to configure:"),
                    Line::from("  - Target: IP address or hostname"),
                    Line::from("  - Port: Target port number"),
                    Line::from("  - Threads: Number of concurrent workers"),
                    Line::from("  - Rate: Packets per second"),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
            TutorialStep::InterfaceSelection => (
                vec![
                    Line::from(Span::styled("Interface Selection", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("• Press Ctrl+I to open interface selector"),
                    Line::from("• Select the network interface for raw socket operations"),
                    Line::from("• Use ↑ ↓ to navigate, ENTER to select"),
                    Line::from("• Press 'T' to test interface connectivity"),
                    Line::from(""),
                    Line::from("This is important for TCP-based attacks that require raw sockets."),
                    Line::from("The selected interface will be used for packet crafting."),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
            TutorialStep::AttackExecution => (
                vec![
                    Line::from(Span::styled("Attack Execution", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("• Press F1 to start the attack"),
                    Line::from("• Press F2 to stop the attack"),
                    Line::from("• Press F3 to view results"),
                    Line::from(""),
                    Line::from("During the attack, you'll see real-time statistics:"),
                    Line::from("  - Packets sent and received"),
                    Line::from("  - Network bandwidth usage"),
                    Line::from("  - Target response status"),
                    Line::from("  - Attack duration"),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
            TutorialStep::ResultsView => (
                vec![
                    Line::from(Span::styled("Results View", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("• View comprehensive attack statistics"),
                    Line::from("• Analyze performance metrics"),
                    Line::from("• Check network response patterns"),
                    Line::from(""),
                    Line::from("The results screen shows:"),
                    Line::from("  - Total packets and bytes sent"),
                    Line::from("  - Average packets per second"),
                    Line::from("  - Success rate and errors"),
                    Line::from("  - Network latency measurements"),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
            TutorialStep::MouseSupport => (
                vec![
                    Line::from(Span::styled("Mouse Support", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("• Click on sections in the left panel to navigate"),
                    Line::from("• Click on configuration fields to select them"),
                    Line::from("• Use mouse wheel to scroll through lists"),
                    Line::from(""),
                    Line::from("Mouse is supported in:"),
                    Line::from("  - Section navigation panel"),
                    Line::from("  - Interface selection dialog"),
                    Line::from("  - Configuration forms"),
                    Line::from("  - Button areas in footer"),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
            TutorialStep::Complete => (
                vec![
                    Line::from(Span::styled("Tutorial Complete!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
                    Line::from(""),
                    Line::from("You've learned the basics of using VertexAttacker!"),
                    Line::from(""),
                    Line::from("Quick Reference:"),
                    Line::from("  - Ctrl+/: Show/hide keyboard shortcuts"),
                    Line::from("  - Ctrl+I: Select network interface"),
                    Line::from("  - F1: Start attack"),
                    Line::from("  - F2: Stop attack"),
                    Line::from("  - F6/F7: Save/Load configuration"),
                    Line::from(""),
                    Line::from("Remember to use this tool responsibly and only"),
                    Line::from("on systems you have permission to test."),
                ],
                vec!["Getting Started", "Navigation", "Configuration", "Interface Selection", "Attack Execution", "Results", "Mouse Support", "Complete"]
            ),
        }
    }

    pub fn render(&mut self, f: &mut Frame, area: Rect) {
        // Create a popup overlay
        let popup_block = Block::default()
            .title(" Interactive Tutorial ")
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Rgb(20, 20, 30)));

        let inner_area = popup_block.inner(area);
        f.render_widget(Clear, inner_area);
        f.render_widget(popup_block, area);

        // Create layout for tutorial
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Top padding
                Constraint::Length(3), // Title
                Constraint::Min(10), // Content
                Constraint::Length(3), // Progress
                Constraint::Length(3), // Navigation buttons
                Constraint::Length(1), // Bottom padding
            ])
            .split(inner_area);

        // Draw title
        let (content_lines, _) = self.get_step_content();
        let title = &content_lines[0];
        let title_paragraph = Paragraph::new(vec![title.clone()])
            .alignment(Alignment::Center)
            .style(Style::default().add_modifier(Modifier::BOLD));
        f.render_widget(title_paragraph, chunks[1]);

        // Draw content
        let content = Paragraph::new(content_lines[1..].to_vec())
            .block(Block::default())
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left);
        f.render_widget(content, chunks[2]);

        // Draw progress
        let progress_items: Vec<ListItem> = Self::get_steps()
            .iter()
            .enumerate()
            .map(|(_i, step)| {
                let is_current = step == &self.current_step;
                let is_completed = self.completed_steps.contains(step);

                let style = if is_current {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else if is_completed {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let prefix = if is_completed { "✓" } else if is_current { "►" } else { " " };
                ListItem::new(Span::styled(format!(" {} {}", prefix, self.get_step_name(step)), style))
            })
            .collect();

        let progress_list = List::new(progress_items)
            .block(Block::default().title(" Progress ").borders(Borders::ALL))
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

        f.render_stateful_widget(progress_list, chunks[3], &mut self.list_state);

        // Draw navigation buttons
        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ])
            .split(chunks[4]);

        // Previous button
        let prev_text = if self.current_step == TutorialStep::Welcome {
            "    "
        } else {
            " ← Previous "
        };
        let prev_button = Span::styled(prev_text, Style::default().fg(if self.current_step != TutorialStep::Welcome { Color::Blue } else { Color::DarkGray }));
        let prev_paragraph = Paragraph::new(Line::from(prev_button))
            .alignment(Alignment::Center);
        f.render_widget(prev_paragraph, button_chunks[0]);

        // Step indicator
        let steps = Self::get_steps();
        let current_idx = steps.iter().position(|s| s == &self.current_step).unwrap_or(0);
        let step_text = format!("Step {} of {}", current_idx + 1, steps.len());
        let step_indicator = Span::styled(step_text, Style::default().fg(Color::Cyan));
        let step_paragraph = Paragraph::new(Line::from(step_indicator))
            .alignment(Alignment::Center);
        f.render_widget(step_paragraph, button_chunks[1]);

        // Next/Finish button
        let next_text = if self.current_step == TutorialStep::Complete {
            " Finish "
        } else {
            " Next → "
        };
        let next_button = Span::styled(next_text, Style::default().fg(Color::Blue));
        let next_paragraph = Paragraph::new(Line::from(next_button))
            .alignment(Alignment::Center);
        f.render_widget(next_paragraph, button_chunks[2]);

        // Draw close hint
        let close_text = Span::styled(" ESC/Q to close ", Style::default().fg(Color::DarkGray));
        let close_paragraph = Paragraph::new(Line::from(close_text))
            .alignment(Alignment::Right);
        let close_area = Rect {
            x: area.x + area.width - 20,
            y: area.y + 1,
            width: 18,
            height: 1,
        };
        f.render_widget(close_paragraph, close_area);
    }

    fn get_step_name(&self, step: &TutorialStep) -> &'static str {
        match step {
            TutorialStep::Welcome => "Welcome",
            TutorialStep::Navigation => "Navigation",
            TutorialStep::Configuration => "Configuration",
            TutorialStep::InterfaceSelection => "Interface Selection",
            TutorialStep::AttackExecution => "Attack Execution",
            TutorialStep::ResultsView => "Results View",
            TutorialStep::MouseSupport => "Mouse Support",
            TutorialStep::Complete => "Complete",
        }
    }
}