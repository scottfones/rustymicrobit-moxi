//! Power policy.

use embassy_time::Duration;

/// Active power mode.
pub const POWER_MODE: PowerMode = PowerMode::High;

/// Operating power mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PowerMode {
    High,
    Low,
}

impl PowerMode {
    /// Polling delay.
    #[must_use]
    pub const fn interval(self) -> Duration {
        match self {
            Self::High => Duration::from_secs(10),
            Self::Low => Duration::from_secs(30),
        }
    }
}
