#![no_std]
#![no_main]

use defmt_rtt as _;
use microbit_bsp as _;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use rustymicrobit_moxi::measurement::{Co2Measurement, PressureMeasurement, fahrenheit};

    #[test]
    #[expect(clippy::float_cmp, reason = "values are exact and representable")]
    fn co2_measurement_is_full_precision() {
        let m = Co2Measurement::new(842, 41.7, 22.5);
        assert_eq!(m.co2, 842.0);
        assert_eq!(m.humidity, 41.7);
        assert_eq!(m.temp_c, 22.5);
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "values are exact and representable")]
    fn pressure_pa_to_hpa() {
        let m = PressureMeasurement::new(101_325.0, 19.0);
        assert_eq!(m.hpa, 1013.25);
        assert_eq!(m.temp_c, 19.0);
    }

    #[test]
    #[expect(clippy::float_cmp, reason = "values are exact and representable")]
    fn fahrenheit_conversion() {
        assert_eq!(fahrenheit(0.0), 32.0);
        assert_eq!(fahrenheit(100.0), 212.0);
    }
}
