use clap::builder::styling::{AnsiColor, Color, Style};

pub static OK: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::BrightGreen)));

pub static INFO: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Yellow)));

pub static ALERT: Style = Style::new().fg_color(Some(Color::Ansi(AnsiColor::Red)));

pub static TABINF: Style = Style::new()
    .fg_color(Some(Color::Ansi(AnsiColor::Black)))
    .bg_color(Some(Color::Ansi(AnsiColor::Cyan)));

// Emojis

pub const CHECK: &str = "\u{2713}";
pub const CROSS: &str = "\u{274c}";
pub const LARRW: &str = "\u{2192}";

// Progress bar styles
use indicatif::{MultiProgress, ProgressBar, ProgressStyle, style::TemplateError};

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
pub struct InstallProgress<'a> {
    name: &'a str,
    bar: ProgressBar,
}

impl<'a> InstallProgress<'a> {
    pub fn with_multiprogress(name: &'a str, mp: &MultiProgress) -> Result<Self, TemplateError> {
        let bar = mp.add(ProgressBar::no_length());
        bar.set_style(ProgressStyle::with_template(&format!(
            " {name:<25} {{spinner:.blue}} {{decimal_bytes}}"
        ))?);
        Ok(Self { name, bar })
    }

    pub fn set_length(&self, size: u64) -> Result<(), TemplateError> {
        self.bar.set_length(size);
        self.bar.set_style(
            ProgressStyle::with_template(&format!(
                " {:<25} {{bar}} {{decimal_bytes}}/{{decimal_total_bytes}}",
                self.name
            ))?
            .progress_chars("--"),
        );
        self.bar.tick();
        Ok(())
    }

    #[inline]
    pub fn inc(&self, size: usize) {
        self.bar.inc(size as u64);
    }

    pub fn success(&self) -> Result<(), TemplateError> {
        self.bar.set_style(ProgressStyle::with_template(&format!(
            "\t{OK}{CHECK}{}{{msg}}{OK:#}",
            self.name
        ))?);
        self.bar.tick();
        Ok(())
    }

    pub fn error(&self) -> Result<(), TemplateError> {
        self.bar.set_style(ProgressStyle::with_template(&format!(
            "\t{ALERT}{CROSS}{}{{msg}}{ALERT:#}",
            self.name
        ))?);
        self.bar.tick();
        Ok(())
    }
}
