use serde::Deserialize;
use structopt::StructOpt;
pub use log::{trace, debug, info, error, warn};

use clap::arg_enum;

arg_enum! {
    #[derive(Debug, Deserialize, Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
    pub enum LevelFilter {
        Trace,
        Debug,
        Info,
        Warn,
        Error,
    }
}

impl From<LevelFilter> for log::LevelFilter {
    fn from(w: LevelFilter) -> Self {
        match w {
            LevelFilter::Trace => Self::Trace,
            LevelFilter::Debug => Self::Debug,
            LevelFilter::Info => Self::Info,
            LevelFilter::Warn => Self::Warn,
            LevelFilter::Error => Self::Error,
        }
    }
}

impl Default for LevelFilter {
    fn default() -> Self {
        LevelFilter::Info
    }
}

arg_enum! {
    #[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize, PartialOrd, Ord)]
    pub enum WriteStyle {
        Auto,
        Always,
        Never,
    }
}

impl From<WriteStyle> for env_logger::WriteStyle {
    fn from(w: WriteStyle) -> Self {
        match w {
            WriteStyle::Auto => Self::Auto,
            WriteStyle::Always => Self::Always,
            WriteStyle::Never => Self::Never,
        }
    }
}

impl Default for WriteStyle {
    fn default() -> Self {
        WriteStyle::Auto
    }
}

#[derive(StructOpt, Deserialize, Debug)]
pub struct LogOpts {
    /// minimum log level printed to STDERR. Choose from:
    /// trace, debug, info, warn, error, off.
    #[structopt(long = "log-level", default_value = "info")]
    #[serde(default)]
    pub level: LevelFilter,

    /// controls when log output is colored. Choose from: auto,
    /// always, and never. RUST_LOG env var may override this parameter.
    #[structopt(long = "log-style", default_value)]
    #[serde(default)]
    pub style: WriteStyle,
}

/// initializes logging from configuration. It uses the
/// configuration initially but allows environment overrides.
pub fn init_logger(opts: &LogOpts) {
    env_logger::Builder::default()
        .filter_level(opts.level.into())
        .write_style(opts.style.into())
        .parse_default_env()
        .init();

    trace!("logging initialized")
}
