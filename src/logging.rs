use log::trace;
use structopt::StructOpt;

use clap::arg_enum;

arg_enum! {
    #[derive(Eq, PartialEq, Debug, Clone, Copy)]
    enum WriteStyle {
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

#[derive(StructOpt, Debug)]
pub struct LogOpts {
    /// minimum log level printed to STDERR. Choose from:
    /// trace, debug, info, warn, error, off.
    #[structopt(long = "log-level", default_value = "info")]
    level: log::LevelFilter,

    /// controls when log output is colored. Choose from: auto,
    /// always, and never.
    #[structopt(long = "log-style", default_value)]
    style: WriteStyle,
}

#[inline]
pub fn init_logger(opts: &LogOpts) {
    env_logger::Builder::from_default_env()
        .filter_level(opts.level)
        .write_style(opts.style.into())
        .init();

    trace!("logging initialized")
}
