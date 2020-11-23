use std::str::FromStr;

use serde::Deserialize;
use structopt::StructOpt;

use crate::types::*;

/*
geo:
  latitude: 0.0
  longitude: 0.0
  altitude: 0.0
logging:
  level: debug
  style: auto
day_brightness: 100
night_brightness: 40
devices:
  - model: /dell U2145/i
    manufacturer_id: DEL
    serial: CFV9N9890J5S
  - model: /dell U2145/i
    manufacturer_id: DEL
    serial: CFV9N9890J5S
    day_brightness: 80
    night_brightness: 50
*/

/*
dlux --config config.yaml

or

dlux --day 100 --night 40 --lat 0.0 \
     --long 0.0 --log-level debug
*/

#[derive(StructOpt, Debug, Deserialize)]
pub struct Opts {
    #[structopt(flatten)]
    pub geo: GeoOpts,

    #[structopt(flatten)]
    pub logging: crate::logging::LogOpts,

    /// percentage of the target screen brightness during day
    #[structopt(short, long, default_value = "100")]
    pub day_brightness: u16,

    /// percentage of the target screen brightness after sunset
    #[structopt(short, long)]
    pub night_brightness: u16,
}

#[derive(StructOpt, Debug, Deserialize)]
pub struct GeoOpts {
    /// latitude of your location for sunset calculations
    #[structopt(long, alias = "lat")]
    pub latitude: f64,

    /// longitude of your location for sunset calculations
    #[structopt(long, alias = "long", alias = "lng")]
    pub longitude: f64,

    /// altitude from sea level in meters of your location for sunset calculations
    #[structopt(long, alias = "height", default_value = "0.0")]
    #[serde(default)]
    pub altitude: f64,
}

#[derive(Debug, Deserialize)]
pub struct Config {
    pub geo: GeoOpts,
}

fn parse_percent<T: AsRef<str>>(input: T) -> Result<u16> {
    let input = input.as_ref().parse::<u16>()?;
    if input > 100 {
        Err(format_err!("{} is too large, should be no more than 100", input))
    } else {
        Ok(input)
    }
}
