use std::convert::TryInto;

use adaptive_backoff::prelude::*;
use anyhow::{format_err, Error, Result};
use chrono::{DateTime, Duration, Local, Utc};
use futures::future::try_join_all;
use humantime::format_duration;
use log::*;
use structopt::StructOpt;
use tokio::{select, time::sleep};

use lib::{
    alarm::Alarm,
    config::{Config, GeoOpts},
    display::{BrightnessOps, Display, Displays},
};

#[derive(StructOpt, Debug)]
pub struct Opts {
    pub config: std::path::PathBuf,
}

pub async fn run(cfg: lib::config::Config) -> Result<(), Error> {
    let mut disps = Displays::new(&cfg.devices)?;
    let mut alarm = Alarm::new()?;
    info!("discovered {} monitors", disps.len());

    if disps.is_empty() {
        return Err(format_err!(
            "no displays discovered: is i2c-dev loaded and do you have access?"
        ));
    }

    loop {
        update_monitors_from_time(&mut disps, &cfg).await;

        let next_dt = get_next_event::<Local>(&cfg.geo, Local::now());
        alarm.reset(next_dt)?;
        info!(
            "sleeping for {} until {}",
            // round down
            format_duration(std::time::Duration::from_secs(
                (next_dt - Utc::now()).num_seconds().try_into().unwrap()
            )),
            next_dt.with_timezone(&Local)
        );

        alarm.future()?.await?;
        debug!("awake, time is now: {}", Local::now());
    }
}

// A bit delicate: we need to check in local timezone so our dates are correct.
// Tomorrow in UTC != tomorrow Local.
fn get_next_event<T: chrono::TimeZone>(opts: &GeoOpts, now: chrono::DateTime<T>) -> DateTime<Utc> {
    let today = now.with_timezone(&Local);
    let geo = get_start_stop_at_date(opts, today.date_naive());

    let next = if now >= geo.1 {
        let tomorrow = today + Duration::days(1);
        get_start_stop_at_date(opts, tomorrow.date_naive()).0
    } else if now < geo.1 {
        geo.1
    } else {
        geo.0
    };
    next + Duration::milliseconds(100)
}

fn get_start_stop_at_date(
    geo: &GeoOpts,
    date: chrono::NaiveDate,
) -> (DateTime<Utc>, DateTime<Utc>) {
    if let Some((start, end)) =
        sun_times::sun_times(date, geo.latitude, geo.longitude, geo.altitude)
    {
        return (start, end);
    }
    unimplemented!(
        "monitor brightness calculation in arctic regions or in the far future is not supported"
    )
}

async fn update_monitors_from_time<'a>(disps: &mut Displays<'a>, cfg: &Config) {
    let now = Local::now();
    // return _today's_ sunrise and sunset times.
    let geo = get_start_stop_at_date(&cfg.geo, now.date_naive());
    let is_daytime = now > geo.0 && now < geo.1;
    info!(
        "updating brightness of all displays to {} value",
        if is_daytime { "daytime" } else { "nighttime" }
    );

    // Run all updates in parallel, retrying, and if any error bail completely.
    // When resuming from suspend, monitors may not wake up consistently and this
    // ensures they eventually are set properly.
    select! {
        res = try_join_all(disps.iter_mut().map(|d| retry_monitor(d, is_daytime))) => match res {
            Err(e) => {
                error!("failed to set display brightness: {}", e);
                panic!("{}", e);
            },
            Ok(_) => {
                debug!("finished setting monitor brightness");
            },
        },
        _ = sleep(std::time::Duration::from_secs(300)) => {
            error!("timed out setting monitor brightness after 5 mintues");
        }
    };
}

/// retry_monitor retires setting brightness on failure indefinely. It's not expected
/// that errors should return except when dependencies fail.
async fn retry_monitor<'a>(disp: &mut Display<'a>, is_day: bool) -> Result<()> {
    let mut backoff = ExponentialBackoffBuilder::default()
        .factor(1.1)
        .min(std::time::Duration::from_secs(0))
        .max(std::time::Duration::from_secs(5))
        .adaptive()
        .build()
        .unwrap();

    let mut tries: usize = 1;
    while let Err(e) = disp.update_brightness(is_day) {
        debug!(
            "failed to set brightness for {} on try {}: {}",
            disp, tries, e
        );
        let delay = backoff.fail();
        tries += 1;
        trace!(
            "backing off {} for {}",
            disp,
            humantime::format_duration(delay)
        );

        sleep(delay).await;
    }

    Ok(())
}
