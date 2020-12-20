use std::convert::{TryFrom, TryInto};

use regex::Regex;
use serde::{Serialize, Deserialize};
use structopt::StructOpt;

use crate::{logging::*, prelude::*, types::*};

#[derive(StructOpt, Debug, Deserialize)]
pub struct Opts {
    #[structopt(flatten)]
    pub geo: GeoOpts,

    #[serde(flatten)]
    #[structopt(flatten)]
    pub brightness: BrightnessOpts,

    #[structopt(flatten)]
    pub logging: crate::logging::LogOpts,

    /// exclusively manage devices matched by the devices list.
    /// If set, unmatched devices are ignored, otherwise unmatched devices
    /// use the global configuration. The default behavior for CLI is
    /// that all devices are handled.
    #[structopt(skip)]
    #[serde(default)]
    pub device_match_exclusive: bool,

    /// device matchers with optional device-specific overrides.
    #[structopt(skip)]
    pub devices: Vec<DeviceOpts>,
}

#[derive(Debug, StructOpt, Default, Deserialize)]
pub struct BrightnessOpts {
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

/// defines a devices' matching critera and its optional brightness overrides.
/// Absent matching values behave as wildcards, while present ones are all AND'd together.
/// If serial is specified, it is a whole case insensitive match and overrides anything else
/// present.
///
/// Model and Manufacturer ID are case sensitive regular expressions. You may include flags to
/// toggle case sensitivity [as outlined in the regex crate](https://docs.rs/regex/1.4.2/regex/#grouping-and-flags),
/// for example "(?i)&dell U2720Q" is case insensitive.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DeviceOpts {
    #[serde(default, with = "serde_regex", skip_serializing_if = "Option::is_none")]
    pub model: Option<Regex>,
    #[serde(default, with = "serde_regex", skip_serializing_if = "Option::is_none")]
    pub manufacturer_id: Option<Regex>,
    // XXX: override exclusivity.
    pub serial: Option<String>,

    /// Forces a specific day brightness for matching devices,
    /// overriding global configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_brightness: Option<u16>,
    /// Forces a specific night brightness for matching devices,
    /// overriding global configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub night_brightness: Option<u16>,
}

impl DeviceOpts {
    pub fn new(model: Option<Regex>, manufacturer_id: Option<Regex>, serial: Option<String>) -> DeviceOpts {
        Self {
            model,
            manufacturer_id, serial,
            ..Self::default()
        }
    }
}

// DeviceOpts + Opts -> DeviceConfig
#[derive(Debug, Clone, Default)]
pub struct DeviceConfig {
    /// Defines what devices get the provided overrid.es
    pub matcher: DeviceMatcher,

    /// Forces a specific day brightness for matching devices,
    /// overriding global configuration.
    pub day_brightness: f64,
    /// Forces a specific night brightness for matching devices,
    /// overriding global configuration.
    pub night_brightness: f64,
}

impl DeviceConfig {
    fn try_from_opts<'a>(opts: DeviceOpts, defaults: &BrightnessOpts) -> Result<DeviceConfig> {
        let matcher = DeviceMatcher {
            model: opts.model,
            mfg: opts.manufacturer_id,
            serial: opts.serial,
        };
        trace!("parsed matcher: {}", matcher);

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

// Normalized output for both config and CLI options.
#[derive(Debug)]
pub struct Config {
    pub geo: GeoOpts,
    pub devices: Vec<DeviceConfig>,
    pub logging: LogOpts,
}

impl Config {
    pub fn new<T: IntoIterator<Item = DeviceOpts>>(
        geo: GeoOpts,
        logging: LogOpts,
        brightness: BrightnessOpts,
        devices: T,
        exclusive_match: bool,
    ) -> Result<Self> {
        let mut devices = devices
            .into_iter()
            .map(|opts| DeviceConfig::try_from_opts(opts, &brightness))
            .collect::<Result<Vec<_>>>()?;

        // Fudge a wildcard matcher if there are no devices or if exclusive_match
        // is false.
        if exclusive_match || devices.is_empty() {
            // at the end so it matches at lowest priority
            devices.push(DeviceConfig {
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
            });
        }

        Ok(Config {
            devices,
            geo,
            logging,
        })
    }

    pub fn from_args() -> Result<Self> {
        Opts::from_args().try_into()
    }
}

impl TryFrom<std::path::PathBuf> for Config {
    type Error = anyhow::Error;

    fn try_from(path: std::path::PathBuf) -> Result<Self> {
        let opts: Opts = serde_yaml::from_reader(std::fs::File::open(path)?)?;

        Config::new(
            opts.geo,
            opts.logging,
            opts.brightness,
            opts.devices,
            opts.device_match_exclusive,
        )
    }
}

impl TryFrom<Opts> for Config {
    type Error = Error;

    fn try_from(opts: Opts) -> Result<Self> {
        Config::new(opts.geo, opts.logging, opts.brightness, opts.devices, true)
    }
}
