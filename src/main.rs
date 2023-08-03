mod daemon;
mod probe;

use std::convert::TryInto;
use structopt::StructOpt;

use lib::types::*;

#[derive(StructOpt, Debug)]
#[structopt(
    name = "dlux",
    about = "Dynamic hardware monitor brightness adjustment"
)]
enum Command {
    Daemon(daemon::Opts),
    Start(lib::config::Opts),
    Probe(probe::Opts),
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let opts: lib::config::Config = match Command::from_args() {
        Command::Daemon(opts) => opts.config.try_into(),
        Command::Start(opts) => opts.try_into(),
        Command::Probe(opts) => return probe::run(opts).await,
    }?;

    lib::logging::init_logger(&opts.logging);
    daemon::run(opts).await
}
