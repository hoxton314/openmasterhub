//! Notices a resume from system suspend, so the modules can be initialised again.
//!
//! On resume the kernel may reset a module (seen on the 15xKey) without disconnecting it. The
//! hidraw node stays open, but the firmware is back at its boot animation and waits for
//! software mode and fresh images (issue 32). Like the vendor app, re-initialise everything
//! about 2 s after resume.

use std::time::{Duration, Instant};

/// How long to wait after a resume before re-initialising, as the vendor app does.
pub const SETTLE: Duration = Duration::from_secs(2);

/// Suspends shorter than this are not reported.
const MIN_SLEEP: Duration = Duration::from_secs(1);

/// Tracks the total time the system has spent suspended.
pub struct SleepWatch {
    last: Option<Duration>,
    pending: Option<Instant>,
}

impl SleepWatch {
    pub fn new() -> Self {
        Self { last: suspended_total(), pending: None }
    }

    /// Call periodically. Returns true once, [`SETTLE`] after a resume.
    pub fn poll(&mut self) -> bool {
        let now = suspended_total();
        if let Some(slept) = slept(self.last, now) {
            log::info!("resumed after {}s asleep, re-initialising the modules shortly", slept.as_secs());
            self.pending = Some(Instant::now());
        }
        self.last = now.or(self.last);
        if self.pending.is_some_and(|t| t.elapsed() >= SETTLE) {
            self.pending = None;
            return true;
        }
        false
    }
}

/// The suspend that happened between two readings of [`suspended_total`], if long enough.
fn slept(before: Option<Duration>, now: Option<Duration>) -> Option<Duration> {
    let slept = now?.checked_sub(before?)?;
    (slept >= MIN_SLEEP).then_some(slept)
}

/// Time spent suspended since boot: `CLOCK_BOOTTIME` counts suspend, `CLOCK_MONOTONIC` does not.
#[cfg(target_os = "linux")]
fn suspended_total() -> Option<Duration> {
    fn read(clock: libc::clockid_t) -> Option<Duration> {
        let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
        // SAFETY: ts is a valid, writable timespec.
        (unsafe { libc::clock_gettime(clock, &mut ts) } == 0).then(|| Duration::new(ts.tv_sec as u64, ts.tv_nsec as u32))
    }
    // Monotonic first, so the difference cannot go negative when nothing was ever suspended.
    let monotonic = read(libc::CLOCK_MONOTONIC)?;
    read(libc::CLOCK_BOOTTIME)?.checked_sub(monotonic)
}

/// Not detected on other platforms yet.
#[cfg(not(target_os = "linux"))]
fn suspended_total() -> Option<Duration> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_real_suspends_count() {
        let s = Duration::from_secs;
        assert_eq!(slept(Some(s(5)), Some(s(25))), Some(s(20)));
        assert_eq!(slept(Some(s(5)), Some(Duration::from_millis(5100))), None);
        assert_eq!(slept(None, Some(s(25))), None);
        assert_eq!(slept(Some(s(5)), None), None);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn clocks_readable() {
        assert!(suspended_total().is_some());
    }
}
