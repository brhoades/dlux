mod daemon;

use failure::{format_err, Error};

use daemon::Opts;
use crate::lib::{
    config::{Config, GeoOpts},
    logging,
};

enum SubCommand {
    Daemon(daemon::Opts),
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let cfg = Opts::new()?;
    logging::init_logger(&cfg.logging);
}
