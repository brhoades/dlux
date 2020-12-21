use std::collections::HashMap;

use regex::{escape, Regex};
use serde_yaml::to_string;
use structopt::StructOpt;

use lib::{config, logging::*, prelude::*, types::*};

#[derive(StructOpt, Debug)]
pub struct Opts {
    #[structopt(flatten)]
    pub logging: LogOpts,
}

pub async fn run(mut opts: Opts) -> Result<()> {
    if opts.logging.level > LevelFilter::Info {
        opts.logging.level = LevelFilter::Info
    }
    init_logger(&opts.logging);

    let def = config::DeviceConfig::default();
    let mut disps = Displays::new(vec![&def])?;

    let disps = disps
        .iter_mut()
        .map(|d| {
            debug!("parsing edid for {}", d);
            let edid = d.display_info()?;
            debug!("edid: {}", edid);

            Ok(config::DeviceOpts::new(
                Some(Regex::new(&escape(&edid.model))?),
                Some(Regex::new(&escape(&edid.serial))?),
                Some(edid.manufacturer),
            ))
        })
        .collect::<Result<Vec<_>>>()?;

    let yaml = to_string(
        &vec![("devices", disps)]
            .into_iter()
            .collect::<HashMap<_, _>>(),
    )?;

    println!("{}", yaml);

    Ok(())
}
