use clap::builder::styling::{AnsiColor, Color, Style};

pub static OK: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightGreen)));

pub static INFO: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

pub static ALERT: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));

// Emojis

pub const CHECK: &str = "\u{2713}";
pub const CROSS: &str = "\u{274c}";
