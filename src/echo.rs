use clap::builder::styling::{AnsiColor, Color, Style};

pub static OK: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightGreen)));

pub static INFO: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

pub static ALERT: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));

pub static TABINF: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Black)))
    .bg_color(Some(Color::Ansi(AnsiColor::Cyan)));

pub static NOTE: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::BrightBlack)))
    .italic();

// Emojis
pub mod glyph {
    pub const CHECK: &str = "\u{2713}";
    pub const CROSS: &str = "\u{2715}";
    pub const WARN: &str = "\u{26a0}";
    pub const ARROW: &str = "\u{2192}";
    //pub const ASTER: &str = "\u{002a}";
}

// Progress bar styles
use glyph::{CHECK, CROSS};
use indicatif::{ProgressBar, ProgressStyle, style::TemplateError};

pub type StyleResult = Result<ProgressStyle, TemplateError>;

pub struct RefreshStyle {}

impl RefreshStyle {
    pub fn progress(name: &str) -> StyleResult {
        ProgressStyle::with_template(&format!("{{spinner:.blue}} {name:.<25} {{msg:.blue}}"))
    }

    pub fn error_msg() -> String {
        format!("{ALERT}{CROSS} Error{ALERT:#}")
    }

    pub fn ok_msg() -> String {
        format!("{OK}{CHECK} Up to date{OK:#}")
    }

    pub fn warn_msg(msg: &str) -> String {
        format!("{INFO}{msg}{INFO:#}")
    }
}

pub struct SearchProgress;

/// Search progress Bar
impl SearchProgress {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> Result<ProgressBar, TemplateError> {
        let bar = ProgressBar::no_length();
        bar.set_style(ProgressStyle::with_template("Searching  {msg}...")?);
        Ok(bar)
    }
}

/// Install progress Bar
#[derive(Clone)]
pub struct InstallProgress {
    bar: ProgressBar,
}

impl InstallProgress {
    pub fn new(name: &str, bar: ProgressBar) -> Result<Self, TemplateError> {
        bar.set_style(ProgressStyle::with_template(&format!(
            " {name:<25} {{spinner:.blue}} {{decimal_bytes}}"
        ))?);
        Ok(Self { bar })
    }

    pub fn set_length(&self, name: &str, size: u64) -> Result<(), TemplateError> {
        self.bar.set_length(size);
        self.bar.set_style(
            ProgressStyle::with_template(&format!(
                "{INFO} {name} {{bar}} {{decimal_bytes}}/{{decimal_total_bytes}}{INFO:#}"
            ))?
            .progress_chars("-- "),
        );
        self.bar.tick();
        Ok(())
    }

    #[inline]
    pub fn inc(&self, size: usize) {
        self.bar.inc(size as u64);
    }

    #[inline]
    pub fn finish(&self) {
        self.bar.finish_and_clear();
    }
}
