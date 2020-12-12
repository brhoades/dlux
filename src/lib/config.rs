use std::convert::{TryFrom, TryInto};

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

    #[serde(flatten)]
    #[structopt(flatten)]
    pub brightness: BrightnessOpt,

    #[structopt(flatten)]
    pub logging: crate::logging::LogOpts,

    #[structopt(skip)]
    pub devices: Vec<DeviceOpt>,
}

#[derive(Debug, StructOpt, Default, Deserialize)]
pub struct BrightnessOpt {
    /// percentage of the target screen brightness during day
    #[structopt(short, long = "day-brightness", parse(try_from_str = parse_brightness_percent))]
    pub day_brightness: Option<u16>,

    /// percentage of the target screen brightness after sunset
    #[structopt(short, long = "night-brightness", parse(try_from_str = parse_brightness_percent))]
    pub night_brightness: Option<u16>,
}

#[derive(StructOpt, Debug, Deserialize)]
pub struct GeoOpts {
    /// latitude of your location for sunset calculations
    #[structopt(long, alias = "lat", parse(try_from_str = parse_geo_coord))]
    pub latitude: f64,

    /// longitude of your location for sunset calculations
    #[structopt(long, alias = "long", alias = "lng", parse(try_from_str = parse_geo_coord))]
    pub longitude: f64,

    /// altitude from sea level in meters of your location for sunset calculations
    #[structopt(long, alias = "height", default_value = "0.0")]
    #[serde(default)]
    pub altitude: f64,
}

fn parse_brightness_percent<T: AsRef<str>>(input: T) -> Result<u16> {
    match input.as_ref().parse::<u16>()? {
        0..=4 => Err(format_err!("minimum of 5% is allowed")),
        input @ 0..=100 => Ok(input),
        _ => Err(format_err!(
            "option is a relative percentage and should be below 100"
        )),
    }
}

fn parse_geo_coord<T: AsRef<str>>(input: T) -> Result<f64> {
    let input = input.as_ref().parse::<f64>()?;
    if input < -180.0 || input > 180.0 {
        Err(format_err!(
            "coordinate is out of range: -180 <= coord <= 180"
        ))
    } else {
        Ok(input)
    }
}

#[derive(Debug, Deserialize)]
pub struct DeviceOpt {
    pub model: Option<String>,
    pub manufacturer_id: Option<String>,
    pub serial: Option<String>,

    pub day_brightness: Option<u16>,
    pub night_brightness: Option<u16>,
}

#[derive(Default, Debug, Clone)]
pub struct DeviceMatcher<T: Clone> {
    model: Option<T>,
    mfg: Option<T>,
    serial: Option<T>,
}

impl<T> std::fmt::Display for DeviceMatcher<T>
where
    T: AsRef<str> + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            Self {
                model: None,
                mfg: None,
                serial: None,
            } => write!(f, "matches any device"),
            Self {
                model: _,
                mfg: _,
                serial: Some(serial),
            } => write!(f, "matches serial {}", serial.as_ref()),
            Self {
                model: Some(model),
                mfg: None,
                serial: None,
            } => write!(f, "matches model {}", model.as_ref()),
            Self {
                model: None,
                mfg: Some(mfg),
                serial: None,
            } => write!(f, "matches manufacturer {}", mfg.as_ref()),
            Self {
                model: Some(model),
                mfg: Some(mfg),
                serial: None,
            } => write!(
                f,
                "matches model {} and manufacturer {}",
                model.as_ref(),
                mfg.as_ref()
            ),
        }
    }
}

// DeviceOpt + Opts -> DeviceConfig
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    // model, mfg, serial
    pub matcher: DeviceMatcher<String>,

    pub day_brightness: f64,
    pub night_brightness: f64,
}

impl DeviceConfig {
    fn try_from_opts<'a>(opts: DeviceOpt, defaults: &BrightnessOpt) -> Result<DeviceConfig> {
        let matcher = DeviceMatcher {
            model: opts.model,
            mfg: opts.manufacturer_id,
            serial: opts.serial,
        };

        let day_brightness = opts.day_brightness.or(defaults.day_brightness).ok_or_else(
            || format_err!("day brightness was absent for rule that {}; it must be provided top-level or in all devices", matcher)
        )?;
        let night_brightness = opts.night_brightness.or(defaults.night_brightness).ok_or_else(
            || format_err!("night brightness was absent for rule that {}; it must be provided top-level or in all devices", matcher)
        )?;

        Ok(DeviceConfig {
            matcher,
            day_brightness: day_brightness as f64 / 100.0,
            night_brightness: night_brightness as f64 / 100.0,
        })
    }
}

// A normalized output for both config and cli options.
#[derive(Debug)]
pub struct Config {
    pub geo: GeoOpts,
    pub devices: Vec<DeviceConfig>,
    pub logging: crate::logging::LogOpts,
}

impl Config {
    pub fn new() -> Result<Self> {
        Opts::from_args().try_into()
    }
}

impl TryFrom<std::path::PathBuf> for Config {
    type Error = anyhow::Error;

    fn try_from(path: std::path::PathBuf) -> Result<Self> {
        let opts: Opts = serde_yaml::from_reader(std::fs::File::open(path)?)?;
        let geo = opts.geo;
        let logging = opts.logging;
        let brightness = opts.brightness;
        let devices = opts
            .devices
            .into_iter()
            .map(|opt| DeviceConfig::try_from_opts(opt, &brightness))
            .collect::<Result<Vec<_>>>()?;

        Ok(Config {
            geo,
            logging,
            devices,
        })
    }
}

impl TryFrom<Opts> for Config {
    type Error = Error;

    fn try_from(opts: Opts) -> Result<Self> {
        let brightness = opts.brightness;
        // Fudge a "All" device matcher.
        let devices = vec![DeviceConfig {
            day_brightness: brightness.day_brightness.ok_or_else(|| {
                format_err!(
                    "must specify --day-brightness for target daytime brightness percentage"
                )
            })? as f64
                / 100.0,
            night_brightness: brightness.night_brightness.ok_or_else(|| {
                format_err!(
                    "must specify --night-brightness for target nighttime brightness percentage"
                )
            })? as f64
                / 100.0,
            matcher: DeviceMatcher::default(),
        }];

        Ok(Config {
            devices,
            geo: opts.geo,
            logging: opts.logging,
        })
    }
}
