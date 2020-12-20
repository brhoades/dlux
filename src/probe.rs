use structopt::StructOpt;

use lib::{config, logging::*, prelude::Displays, types::*};

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(flatten)]
    pub logging: LogOpts,
}

pub async fn run(mut opts: Opts) -> Result<()> {
    // force info or higher for below output
    if opts.logging.level > LevelFilter::Info {
        opts.logging.level = LevelFilter::Info
    }
    init_logger(&opts.logging);

    let def = config::DeviceConfig::default();
    let mut disps = Displays::new(vec![&def])?;

    for disp in disps.iter_mut() {
        let edid = disp.display_info();
        info!("device: {}", disp);
        info!("edid: {:?}", edid);
    }

    Ok(())
}
