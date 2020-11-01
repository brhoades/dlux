mod alarm;

use std::convert::TryInto;

use clap::arg_enum;
use failure::{format_err, Error};
use log::{debug, error, info, warn};
use structopt::StructOpt;

use chrono::{DateTime, Duration, Local, Utc};
use ddc::Ddc;
use humantime::format_duration;

use alarm::Alarm;

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(flatten)]
    devices: DeviceOpts,

    #[structopt(flatten)]
    geo: GeoOpts,

    #[structopt(flatten)]
    logging: LogOpts,

    /// out of 100, the brightness to target for the screen
    #[structopt(short, long)]
    brightness: u16,
    // fade in/out time at sunrise
    // #[structopt(short, long)]
    // fade_time: chrono::Duration
}

#[derive(StructOpt, Debug)]
struct GeoOpts {
    #[structopt(long)]
    lat: f64,

    #[structopt(long)]
    long: f64,

    #[structopt(long, default_value = "0.0")]
    height: f64,
}

#[derive(StructOpt, Debug)]
struct DeviceOpts {
    #[structopt(long)]
    model: Vec<String>,

    #[structopt(long)]
    all: bool,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = Opts::from_args();
    init_logger(&opts.logging);

    let mut disps = Displays::new()?;
    let mut alarm = Alarm::new()?;

    info!("discovered {} monitors", disps.len());

    loop {
        update_monitors_from_time(&mut disps, &opts);

        let next_dt = get_next_event::<Local>(&opts.geo, Local::now());
        alarm.reset(next_dt)?;
        info!(
            "sleeping for {} until {}",
            // round down
            format_duration(std::time::Duration::from_secs(
                (next_dt - Utc::now()).num_seconds().try_into().unwrap()
            )),
            next_dt.with_timezone(&Local)
        );
        std::mem::forget(next_dt);

        alarm.future()?.await?;
        debug!("awake, time is now: {}", Local::now());
    }
}

type Devs = Vec<ddc_i2c::I2cDdc<i2c_linux::I2c<std::fs::File>>>;

struct Displays {
    pub devs: Devs,
}

impl Displays {
    fn new() -> Result<Self, Error> {
        let devs = ddc_i2c::I2cDeviceEnumerator::new()?
            .filter_map(|mut i| {
                if i.get_vcp_feature(0x10).is_ok() {
                    Some(i)
                } else {
                    None
                }
            })
            .collect::<Devs>();

        if devs.len() == 0 {
            let devs = ddc_i2c::I2cDeviceEnumerator::new()?;
            let mut cnt = 0;

            for mut i in devs {
                cnt += 1;
                match &i.get_vcp_feature(0x10) {
                    Ok(cap) => {
                        let fd = i.into_inner().into_inner();

                        error!("{:?}: unstable device - originally failed to query brightness but now succeeded\n", fd);
                        debug!("{:?}: now has {:?}", fd, cap);
                    }
                    Err(e) => {
                        warn!("{:?}: {}\n", i.into_inner().into_inner(), e);
                    }
                }
            }

            if cnt != 0 {
                Err(format_err!("failed to discover compatible devices: no compatible monitors were found (of {}). Do your monitors support ddc?", cnt))
            } else {
                Err(format_err!("failed to query any devices: is the i2c-dev module loaded and can your user cannot access /dev/i2c file descriptors?"))
            }
        } else {
            Ok(Displays { devs })
        }
    }

    pub fn len(&self) -> usize {
        self.devs.len()
    }

    // Idempotently set brightness to the passed value..
    pub fn set_brightness(&mut self, b: u16) {
        for d in &mut self.devs {
            match d.set_vcp_feature(0x10, b) {
                Ok(_) => debug!(
                    "set brightness for {:?} to {}",
                    d.inner_ref().inner_ref(),
                    b,
                ),
                Err(e) => error!("failed to set to {}: {}", b, e),
            }
        }
    }
}

// A bit delicate: we need to check in local timezone so our dates are correct.
// Tomorrow in UTC != tomorrow Local.
fn get_next_event<T: chrono::TimeZone>(opts: &GeoOpts, now: chrono::DateTime<T>) -> DateTime<Utc> {
    let today = now.with_timezone(&Local);
    let geo = get_start_stop_at_date(opts, today.date());

    let next = if now >= geo.1 {
        let tomorrow = today + Duration::days(1);
        get_start_stop_at_date(opts, tomorrow.date()).0
    } else if now < geo.1 {
        geo.1
    } else {
        geo.0
    };
    next + Duration::milliseconds(100)
}

fn get_start_stop_at_date<T: chrono::TimeZone>(
    geo: &GeoOpts,
    date: chrono::Date<T>,
) -> (DateTime<Utc>, DateTime<Utc>) {
    let (start, end) =
        sun_times::sun_times(date.with_timezone(&Utc), geo.lat, geo.long, geo.height);
    (start, end)
}

fn update_monitors_from_time(disps: &mut Displays, opts: &Opts) {
    let now = Local::now();
    let geo = get_start_stop_at_date(&opts.geo, now.date());

    let b = if now < geo.0 || now > geo.1 {
        opts.brightness
    } else {
        100
    };

    debug!("updating brightness of all displays to {}", b);
    disps.set_brightness(b);
}

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

#[derive(StructOpt, Debug)]
struct LogOpts {
    #[structopt(long = "log-level", default_value = "info")]
    level: log::LevelFilter,

    #[structopt(long = "log-style", default_value = "auto")]
    style: WriteStyle,
}

#[inline]
fn init_logger(opts: &LogOpts) {
    env_logger::Builder::from_default_env()
        .filter_level(opts.level)
        .write_style(opts.style.into())
        .init();
}
