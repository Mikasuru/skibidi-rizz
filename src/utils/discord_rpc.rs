use discord_rpc_client::Client as DiscordClient;
use std::time::{Duration, Instant};
use crate::types::types::AppState;

pub struct DiscordRPC {
    client: Option<DiscordClient>,
    is_connected: bool,
    last_update: Instant,
    last_state: Option<AppState>,
    last_activity: Instant,
    current_section: Option<String>,
    input_active: bool,
}

impl DiscordRPC {
    pub fn new() -> Self {
        Self {
            client: None,
            is_connected: false,
            last_update: Instant::now(),
            last_state: None,
            last_activity: Instant::now(),
            current_section: None,
            input_active: false,
        }
    }

    pub fn init(&mut self) -> Result<(), String> {
        let client = DiscordClient::new(1398983493089759232);
        self.client = Some(client);
        self.is_connected = true;
        Ok(())
    }

    // upd when user is actively doing smth
    pub fn update_activity(&mut self) {
        self.last_activity = Instant::now();
    }

    // upd when entering a section
    pub fn set_section(&mut self, section: &str) {
        self.current_section = Some(section.to_string());
        self.update_activity();
    }

    // upd when input mode changes
    pub fn set_input_mode(&mut self, active: bool) {
        self.input_active = active;
        if active {
            self.update_activity();
        }
    }

    pub fn update_presence(&mut self, state: &AppState, details: &str) -> Result<(), String> {
        let is_idle = self.last_activity.elapsed() > Duration::from_secs(30); // check for idle

        let state_changed = self.last_state.as_ref() != Some(state);
        let enough_time_passed = self.last_update.elapsed() > Duration::from_secs(5);

        if !state_changed && !enough_time_passed && !is_idle {
            return Ok(());
        }

        if let Some(mut client) = self.client.take() {
            client.start();

            let (state_text, details_text): (String, String) = if is_idle {
                ("Standby".to_string(), "Away from keyboard".to_string())
            } else {
                match state {
                    AppState::Config => {
                        if self.input_active {
                            ("Editing configuration".to_string(), details.to_string())
                        } else if let Some(ref section) = self.current_section {
                            ("Configuring".to_string(), format!("In {} section", section))
                        } else {
                            ("Configuring attack".to_string(), "Browsing sections".to_string())
                        }
                    },
                    AppState::Attack => {
                        if details.contains("stopped") {
                            ("Attack stopped".to_string(), "Ready for new attack".to_string())
                        } else {
                            ("Attack in progress".to_string(), details.to_string())
                        }
                    },
                    AppState::Results => ("Viewing results".to_string(), "Analyzing attack metrics".to_string()),
                }
            };

            let details_owned = details_text;
            let state_owned = state_text;

            let _ = std::thread::spawn(move || {
                let start_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                // dont care about errors
                let _ = client.set_activity(|act| {
                    act.state(&state_owned)
                        .details(&details_owned)
                        .assets(|assets| {
                            assets.large_image("vertex_logo")
                                .large_text("VertexAttacker")
                        })
                        .timestamps(|ts| ts.start(start_time))
                });
            });
        }

        // create a new client for next update
        self.client = Some(DiscordClient::new(1398983493089759232));
        self.last_update = Instant::now();
        self.last_state = Some(state.clone());

        Ok(())
    }

    pub fn shutdown(&mut self) {
        self.client = None;
        self.is_connected = false;
    }
}

impl Default for DiscordRPC {
    fn default() -> Self {
        Self::new()
    }
}