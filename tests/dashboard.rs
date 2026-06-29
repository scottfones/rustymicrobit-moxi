#![no_std]
#![no_main]

use defmt_rtt as _;
use microbit_bsp as _;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use microbit_bsp::display::Bitmap;
    use rustymicrobit_moxi::dashboard::{
        CO2_SATURATION_PPM, HUMIDITY_SATURATION_PCT, LED_COLS, LED_ROWS, TEMP_SATURATION_F,
        construct_dashboard_rows,
    };

    #[test]
    fn dashboard_encoding_0_0_0() {
        let expected = [Bitmap::empty(LED_COLS); LED_ROWS];
        defmt::assert_eq!(construct_dashboard_rows(0, 0, 0), expected);
    }

    #[test]
    fn dashboard_encoding_saturated() {
        let expected = [Bitmap::new(0b11111, LED_COLS); LED_ROWS];
        defmt::assert_eq!(
            construct_dashboard_rows(
                CO2_SATURATION_PPM,
                HUMIDITY_SATURATION_PCT,
                TEMP_SATURATION_F
            ),
            expected
        );
    }

    #[test]
    fn dashboard_encoding_intent_twins() {
        #[rustfmt::skip]
        let expected = [
            Bitmap::new(0b01010, LED_COLS),
            Bitmap::new(0b00000, LED_COLS),
            Bitmap::new(0b10000, LED_COLS),
            Bitmap::new(0b10101, LED_COLS),
            Bitmap::new(0b10101, LED_COLS),
        ];
        defmt::assert_eq!(construct_dashboard_rows(601, 41, 72), expected);
        defmt::assert_eq!(construct_dashboard_rows(639, 59, 71), expected);
    }

    #[test]
    fn dashboard_encoding_intent_saturation_min() {
        let expected = [Bitmap::new(0b11111, LED_COLS); LED_ROWS];
        defmt::assert_eq!(construct_dashboard_rows(1361, 90, 99), expected);
    }

    #[test]
    fn dashboard_encoding_intent_saturation_pre() {
        #[rustfmt::skip]
        let expected= [
            Bitmap::new(0b11110, LED_COLS),
            Bitmap::new(0b11111, LED_COLS),
            Bitmap::new(0b11111, LED_COLS),
            Bitmap::new(0b11111, LED_COLS),
            Bitmap::new(0b10101, LED_COLS),
        ];
        defmt::assert_eq!(construct_dashboard_rows(1360, 89, 98), expected);
    }
}
