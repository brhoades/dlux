use structopt::StructOpt;
use serde::Deserialize;

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

#[derive(StructOpt, Debug)]
pub(crate) struct Opts {
    #[structopt(flatten)]
    pub geo: GeoOpts,

    #[structopt(flatten)]
    pub logging: crate::logging::LogOpts,

    /// percentage of the target screen brightness at sunset
    #[structopt(short, long)]
    pub brightness: u16,
}

#[derive(StructOpt, Debug, Deserialize)]
pub(crate) struct GeoOpts {
    /// latitude of your location for sunset calculations
    #[structopt(long, alias = "lat")]
    pub latitude: f64,

    /// longitude of your location for sunset calculations
    #[structopt(long, alias = "long", alias = "lng")]
    pub longitude: f64,

    /// altitude from sea level in meters of your location for sunset calculations
    #[serde(default)]
    #[structopt(long, alias = "height", default_value = "0.0")]
    pub altitude: f64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Config {
    pub geo: GeoOpts,
}
