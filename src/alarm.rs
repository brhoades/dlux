use std::task::{Context, Poll};

use chrono::{DateTime, TimeZone, Utc};
use failure::{format_err, Error};
use nix::sys::time::TimeSpec;
use nix::sys::timerfd::{ClockId, Expiration, TimerFd, TimerFlags, TimerSetTimeFlags};
use tokio::io::unix::AsyncFd;

pub struct Alarm {
    fd: TimerFd,
    afd: AsyncFd<TimerFd>,
    set: bool,
}

impl<'a> Alarm {
    /// Creates a new Alarm via `timerfd_create` and returns any errors. The alarm
    /// is not ready for use and must have [set](Alarm.reset.html) called prior to
    /// use.
    pub fn new() -> Result<Self, Error> {
        let fd = TimerFd::new(ClockId::CLOCK_BOOTTIME, TimerFlags::TFD_NONBLOCK)?;
        let t = Self {
            fd,
            afd: AsyncFd::new(fd)?,
            set: false,
        };
        Ok(t)
    }

    /// Creates a new, ready-to-use Alarm via `timerfd_create` and returns any errors.
    /// The passed [DateTime](chrono::DateTime) is used to call [set](Alarm.set.html).
    #[allow(dead_code)]
    pub fn at_time<T: chrono::TimeZone>(dt: DateTime<T>) -> Result<Self, Error> {
        let mut t = Self::new()?;
        t.set(dt)?;
        Ok(t)
    }

    /// Sets the alarm to fire at the datetime provided. It returns errors from calling
    /// `timerfd_settime` if there are any.
    ///
    /// This function must be called at least once prior to use.
    #[allow(dead_code)]
    pub fn set<T: chrono::TimeZone>(&mut self, dt: DateTime<T>) -> Result<(), Error> {
        self.reset(dt)
    }

    /// Sets the alarm to fire at the datetime provided. It returns errors from calling
    /// `timerfd_settime` if there are any.
    ///
    /// This function must be called at least once prior to use.
    pub fn reset<T: chrono::TimeZone>(&mut self, dt: DateTime<T>) -> Result<(), Error> {
        let delay = (dt.with_timezone(&Utc) - Utc::now())
            .to_std()
            .unwrap_or(std::time::Duration::new(0, 100));

        self.fd.set(
            Expiration::OneShot(TimeSpec::from(delay)),
            TimerSetTimeFlags::empty(),
        )?;
        self.set = true;

        Ok(())
    }

    /// Creates a future that, when polled, waits until the last-set time is reached.
    /// Will error if [set](Alarm.set.html) was never called.
    ///
    /// Since there is exactly one timer file descriptor per Alarm, only one future may
    /// exist at a time. Once a future is polled to completion, it's safe to reset the alarm
    /// and call it again.
    pub fn future(&'a mut self) -> Result<FutureAlarm<'a>, Error> {
        if !self.set {
            return Err(format_err!("timer must be set before waiting"));
        }

        Ok(FutureAlarm { afd: &mut self.afd })
    }
}

/// FutureAlarm mutably borrows the alarm's file descriptor so only may exist
/// at once. When awaited, the future will complete once the alarm's set datetime
/// is reached.
pub struct FutureAlarm<'a> {
    afd: &'a mut AsyncFd<TimerFd>,
}

impl std::future::Future for FutureAlarm<'_> {
    type Output = Result<(), Error>;

    fn poll(self: std::pin::Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        match self.afd.poll_read_ready(ctx) {
            Poll::Ready(Ok(mut res)) => {
                // consume the guard
                res.clear_ready();
                Poll::Ready(Ok(()))
            }
            Poll::Ready(Err(e)) => Poll::Ready(Err(format_err!("failed to read fd: {}", e))),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// suspend-aware wait until date. See `man timerfd_create(2)`.
/// This implementation leaks fds since it creates a new one and never
/// cleans them up.
#[allow(dead_code)]
fn wait_until<T: TimeZone>(dt: DateTime<T>) -> Result<(), Error> {
    let delay = (dt.with_timezone(&Utc) - Utc::now())
        .to_std()
        .unwrap_or(std::time::Duration::new(0, 100));

    let timer = TimerFd::new(ClockId::CLOCK_BOOTTIME, TimerFlags::TFD_CLOEXEC)?;
    timer.set(
        Expiration::OneShot(TimeSpec::from(delay)),
        TimerSetTimeFlags::empty(),
    )?;

    Ok(timer.wait()?)
}
