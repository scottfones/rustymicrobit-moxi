#![no_std]
#![no_main]

use defmt_rtt as _;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use embassy_time::Duration;
    use rustymicrobit_moxi::power::PowerMode;

    #[test]
    fn intervals() {
        assert_eq!(PowerMode::High.interval(), Duration::from_secs(10));
        assert_eq!(PowerMode::Low.interval(), Duration::from_secs(30));
    }
}
