use chrono::{DateTime, Duration, Local, Utc};
use ddc::Ddc;
use failure::{format_err, Error};
use humantime::format_duration;
use structopt::StructOpt;
use tokio::time::sleep_until;

#[derive(StructOpt, Debug)]
struct Opts {
    #[structopt(flatten)]
    devices: DeviceOpts,

    #[structopt(flatten)]
    geo: GeoOpts,

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

#[derive(Debug)]
struct GeoSettings {
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

#[tokio::main]
async fn main() {
    let opts = Opts::from_args();
    let mut disps = Displays::new().unwrap();

    println!("found {} devices, beginning loop", disps.len());

    loop {
        update_monitors_from_time(&mut disps, &opts);

        let next_dt = get_next_event::<Local>(&opts.geo, Local::now());
        let wait = (next_dt - Utc::now())
            .to_std()
            .unwrap_or(std::time::Duration::new(0, 0));
        println!(
            "Waiting {} until {}",
            format_duration(wait),
            next_dt.with_timezone(&Local)
        );

        sleep_until(tokio::time::Instant::now() + wait).await;
        println!("awake, time is now: {}", Local::now());
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
            Err(format_err!("failed to retrieve supported devices"))
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
                Ok(_) => println!("set brightness to {}", b),
                Err(e) => println!("failed to set to {}: {}", b, e),
            }
        }
    }
}

fn get_next_event<T: chrono::TimeZone>(opts: &GeoOpts, now: chrono::DateTime<T>) -> DateTime<Utc> {
    let today = now.date().with_timezone(&Utc);
    let geo = get_start_stop_at_date(opts, today);

    let next = if now >= geo.end {
        let tomorrow = today + Duration::days(1);
        get_start_stop_at_date(opts, tomorrow).start
    } else if now < geo.end {
        geo.end
    } else {
        geo.start
    };
    next + Duration::milliseconds(100)
}

fn get_start_stop_at_date(geo: &GeoOpts, date: chrono::Date<Utc>) -> GeoSettings {
    let (start, end) = sun_times::sun_times(date, geo.lat, geo.long, geo.height);
    GeoSettings { start, end }
}

fn update_monitors_from_time(disps: &mut Displays, opts: &Opts) {
    let now = Utc::now();
    let geo = get_start_stop_at_date(&opts.geo, now.date());

    let b = if now < geo.start || now > geo.end {
        opts.brightness
    } else {
        100
    };

    println!("updating brightness to {}", b);
    disps.set_brightness(b);
}
