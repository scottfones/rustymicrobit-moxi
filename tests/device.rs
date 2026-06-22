#![no_std]
#![no_main]

use defmt_rtt as _;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use embassy_time::Duration;
    use microbit_bsp::Microbit;
    use rustymicrobit_moxi::power::PowerMode;

    #[init]
    fn init() -> Microbit {
        Microbit::default()
    }

    #[test]
    fn ficr_serial_is_nonzero(_board: Microbit) {
        // SAFETY: FICR DEVICEID 0 is a read-only register, always mapped.
        let id0 = unsafe { core::ptr::read_volatile(0x1000_0060 as *const u32) };
        assert_ne!(id0, 0);
    }

    #[test]
    fn power_mode_intervals() {
        assert_eq!(PowerMode::High.interval(), Duration::from_secs(10));
        assert_eq!(PowerMode::Low.interval(), Duration::from_secs(30));
    }
}
