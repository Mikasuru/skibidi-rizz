use ratatui::style::Color;
use crate::types::types::ConfigField;

// Configuration sections for the UI
pub const CONFIG_SECTIONS: &[(&str, &str, &[ConfigField])] = &[
    (
        "Target & Network",
        "Choose where traffic should be delivered",
        &[
            ConfigField::Target,
            ConfigField::Port,
            ConfigField::RandomPorts,
        ],
    ),
    (
        "Attack Profile",
        "Adjust throughput and scheduling parameters",
        &[
            ConfigField::Threads,
            ConfigField::Rate,
            ConfigField::Duration,
            ConfigField::Mode,
        ],
    ),
    (
        "Payload & Identity",
        "Control packet contents and presentation",
        &[
            ConfigField::PacketSize,
            ConfigField::CustomPayload,
            ConfigField::RandomPayload,
            ConfigField::RotateUserAgent,
        ],
    ),
    (
        "Evasion & Timing",
        "Fine-tune pacing, bursts, and multi-vector options",
        &[
            ConfigField::EvasMode,
            ConfigField::SizeStrategy,
            ConfigField::VariancePercentage,
            ConfigField::BurstSize,
            ConfigField::SecondaryAttack,
        ],
    ),
    (
        "Presets",
        "Quick configuration templates for common scenarios",
        &[
            ConfigField::Preset,
        ],
    ),
    (
        "Settings",
        "Customize application behavior and appearance",
        &[
            ConfigField::Theme,
            ConfigField::RpcEnabled,
            ConfigField::AutoSave,
        ],
    ),
];

// Theme structure for UI styling
pub struct Theme {
    pub bg_dark: Color,
    pub bg_main: Color,
    pub bg_float: Color,
    pub border: Color,
    pub text_dim: Color,
    pub text_normal: Color,
    pub text_bright: Color,
    pub cyan: Color,
    pub blue: Color,
    pub magenta: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub orange: Color,
}

impl Theme {
    pub fn tokyo_night() -> Self {
        Self {
            bg_dark: Color::Rgb(26, 27, 38),      // #1a1b26
            bg_main: Color::Rgb(32, 33, 48),       // #202130
            bg_float: Color::Rgb(36, 37, 54),      // #242536
            border: Color::Rgb(77, 80, 113),       // #4d5071
            text_dim: Color::Rgb(122, 125, 153),   // #7a7d99
            text_normal: Color::Rgb(169, 177, 214), // #a9b1d6
            text_bright: Color::Rgb(205, 214, 244), // #cdd6f4
            cyan: Color::Rgb(137, 221, 255),       // #89ddff
            blue: Color::Rgb(122, 162, 247),      // #7aa2f7
            magenta: Color::Rgb(187, 154, 247),    // #bb9af7
            green: Color::Rgb(158, 206, 106),      // #9ece6a
            red: Color::Rgb(242, 139, 130),        // #f28b82
            yellow: Color::Rgb(250, 227, 176),      // #fae3b0
            orange: Color::Rgb(255, 184, 108),      // #ffb86c
        }
    }

    pub fn dracula() -> Self {
        Self {
            bg_dark: Color::Rgb(40, 42, 54),       // #282a36
            bg_main: Color::Rgb(68, 71, 90),        // #44475a
            bg_float: Color::Rgb(98, 114, 164),     // #6272a4
            border: Color::Rgb(98, 114, 164),       // #6272a4
            text_dim: Color::Rgb(189, 195, 199),   // #bdc3c7
            text_normal: Color::Rgb(248, 248, 242),  // #f8f8f2
            text_bright: Color::Rgb(255, 255, 255), // #ffffff
            cyan: Color::Rgb(139, 233, 253),       // #8be9fd
            blue: Color::Rgb(80, 250, 123),         // #50fa7b
            magenta: Color::Rgb(189, 147, 249),     // #bd93f9
            green: Color::Rgb(80, 250, 123),        // #50fa7b
            red: Color::Rgb(255, 85, 85),          // #ff5555
            yellow: Color::Rgb(241, 250, 140),      // #f1fa8c
            orange: Color::Rgb(255, 184, 108),      // #ffb86c
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            bg_dark: Color::Rgb(29, 32, 33),       // #1d2021
            bg_main: Color::Rgb(40, 40, 40),        // #282828
            bg_float: Color::Rgb(60, 56, 54),       // #3c3836
            border: Color::Rgb(124, 111, 100),      // #7c6f64
            text_dim: Color::Rgb(189, 174, 147),   // #bdae93
            text_normal: Color::Rgb(235, 219, 178),  // #ebdbb2
            text_bright: Color::Rgb(251, 241, 199), // #fbf1c7
            cyan: Color::Rgb(131, 165, 152),       // #83a598
            blue: Color::Rgb(131, 165, 152),       // #83a598
            magenta: Color::Rgb(211, 134, 155),     // #d3869b
            green: Color::Rgb(184, 187, 38),       // #b8bb26
            red: Color::Rgb(204, 36, 29),          // #cc241d
            yellow: Color::Rgb(250, 189, 47),      // #fabd2f
            orange: Color::Rgb(254, 128, 25),       // #fe8019
        }
    }

    pub fn solarized() -> Self {
        Self {
            bg_dark: Color::Rgb(0, 43, 54),        // #002b36
            bg_main: Color::Rgb(7, 54, 66),        // #073642
            bg_float: Color::Rgb(88, 110, 117),     // #586e75
            border: Color::Rgb(101, 123, 131),      // #657b83
            text_dim: Color::Rgb(147, 161, 161),   // #93a1a1
            text_normal: Color::Rgb(253, 246, 227),  // #fdf6e3
            text_bright: Color::Rgb(238, 232, 213),  // #eee8d5
            cyan: Color::Rgb(131, 148, 150),       // #839496
            blue: Color::Rgb(131, 148, 150),       // #839496
            magenta: Color::Rgb(211, 54, 130),      // #d33682
            green: Color::Rgb(133, 153, 0),         // #859900
            red: Color::Rgb(220, 50, 47),          // #dc322f
            yellow: Color::Rgb(181, 137, 0),       // #b58900
            orange: Color::Rgb(203, 75, 22),        // #cb4b16
        }
    }

    pub fn monokai() -> Self {
        Self {
            bg_dark: Color::Rgb(39, 40, 34),       // #272822
            bg_main: Color::Rgb(60, 63, 65),       // #3c3f41
            bg_float: Color::Rgb(77, 81, 87),      // #4d5157
            border: Color::Rgb(97, 97, 97),        // #616161
            text_dim: Color::Rgb(153, 153, 153),   // #999999
            text_normal: Color::Rgb(248, 248, 242),  // #f8f8f2
            text_bright: Color::Rgb(255, 255, 255), // #ffffff
            cyan: Color::Rgb(117, 255, 244),       // #75fff4
            blue: Color::Rgb(102, 217, 239),       // #66d9ef
            magenta: Color::Rgb(174, 129, 255),     // #ae81ff
            green: Color::Rgb(166, 226, 46),       // #a6e22e
            red: Color::Rgb(249, 38, 114),          // #f92672
            yellow: Color::Rgb(230, 219, 116),      // #e6db74
            orange: Color::Rgb(255, 137, 81),       // #ff8951
        }
    }

    pub fn nord() -> Self {
        Self {
            bg_dark: Color::Rgb(46, 52, 64),       // #2e3440
            bg_main: Color::Rgb(59, 66, 82),        // #3b4252
            bg_float: Color::Rgb(67, 76, 94),       // #434c5e
            border: Color::Rgb(67, 76, 94),        // #434c5e
            text_dim: Color::Rgb(136, 162, 178),   // #88a2bc
            text_normal: Color::Rgb(216, 222, 233),  // #d8dee9
            text_bright: Color::Rgb(236, 239, 244),  // #eceff4
            cyan: Color::Rgb(136, 192, 208),       // #88c0d0
            blue: Color::Rgb(129, 161, 193),       // #81a1c1
            magenta: Color::Rgb(180, 142, 173),     // #b48ead
            green: Color::Rgb(163, 190, 140),       // #a3be8c
            red: Color::Rgb(191, 97, 106),         // #bf616a
            yellow: Color::Rgb(235, 203, 139),      // #ebcb8b
            orange: Color::Rgb(208, 135, 112),      // #d08770
        }
    }

    pub fn get_current(app: &crate::app::app::App) -> Self {
        match app.theme_index {
            0 => Self::tokyo_night(),
            1 => Self::dracula(),
            2 => Self::gruvbox(),
            3 => Self::solarized(),
            4 => Self::monokai(),
            5 => Self::nord(),
            _ => Self::tokyo_night(),
        }
    }
}