mod daemon;

mod probe;

use std::convert::TryInto;
use structopt::StructOpt;

use lib::types::*;

#[structopt(
    name = "dlux",
    about = "Dynamic hardware monitor brightness adjustment"
)]
#[derive(StructOpt, Debug)]
enum Command {
    Daemon(daemon::Opts),
    Start(lib::config::Opts),
    Probe,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts: lib::config::Config = match Command::from_args() {
        Command::Daemon(opts) => opts.config.try_into(),
        Command::Start(opts) => opts.try_into(),
        Probe => return probe::run().await,
    }?;

    lib::logging::init_logger(&opts.logging);
    daemon::run(opts).await
}
