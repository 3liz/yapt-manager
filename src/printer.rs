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
}

// Progress bar styles
use glyph::{CHECK, CROSS};
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle, style::TemplateError};

/// Source cache progress Bar
#[derive(Clone)]
pub struct CacheProgress {
    bar: ProgressBar,
    name: String,
}

impl CacheProgress {
    pub fn new(name: &str, bar: ProgressBar) -> Result<Self, TemplateError> {
        bar.set_style(ProgressStyle::with_template(&format!(
            "{{spinner:.blue}} {name:.<25} {{msg:.blue}}"
        ))?);
        let name = if bar.is_hidden() {
            name.to_string()
        } else {
            String::new()
        };
        Ok(Self { bar, name })
    }

    #[inline]
    pub fn tick(&self) {
        self.bar.tick();
    }

    #[inline]
    pub fn set_message(&self, msg: &'static str) {
        self.bar.set_message(msg);
    }

    #[inline]
    pub fn finish_with_error(&self) {
        self.finish_with_message(format!("{ALERT}{CROSS} Error{ALERT:#}"));
    }

    #[inline]
    pub fn finish_with_success(&self) {
        self.finish_with_message(format!("{OK}{CHECK} Up to date{OK:#}"));
    }

    #[inline]
    pub fn finish_with_warning(&self, msg: &str) {
        self.finish_with_message(format!("{INFO}{msg}{INFO:#}"));
    }

    fn finish_with_message(&self, msg: String) {
        if self.bar.is_hidden() {
            eprintln!("{:.<25} {msg}", self.name)
        } else {
            self.bar.finish_with_message(msg);
        }
    }
}

pub struct SearchProgress;

/// Search progress Bar
impl SearchProgress {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(target: ProgressDrawTarget) -> Result<ProgressBar, TemplateError> {
        let bar = ProgressBar::with_draw_target(None, target);
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
