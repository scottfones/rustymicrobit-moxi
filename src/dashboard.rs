//! Dashboard encoding for the 5x5 Microbit LED matrix.

use microbit_bsp::display::Bitmap;

/// LED matrix column count.
pub const LED_COLS: usize = 5;

/// LED matrix row count.
pub const LED_ROWS: usize = 5;

/// Temperature minimum display value (F).
pub const TEMP_BASE_F: i16 = 50;

/// Temperature primary step value (F).
pub const TEMP_STEP_F: i16 = 10;

/// Temperature secondary step value (F).
pub const TEMP_SUBSTEP_F: i16 = 2;

/// Temperature display saturation value (F).
pub const TEMP_SATURATION_F: i16 = 1
    + TEMP_BASE_F
    + (LED_ROWS - 1).saturating_truncate::<u16>().cast_signed() * (TEMP_STEP_F + TEMP_SUBSTEP_F);

/// CO2 minimum display value (ppm).
pub const CO2_BASE_PPM: u16 = 400;

/// CO2 primary step value (ppm).
pub const CO2_STEP_PPM: u16 = 200;

/// CO2 secondary step value (ppm).
pub const CO2_SUBSTEP_PPM: u16 = 40;

/// CO2 display saturation value (ppm).
pub const CO2_SATURATION_PPM: u16 = 1
    + CO2_BASE_PPM
    + (LED_ROWS - 1).saturating_truncate::<u16>() * (CO2_STEP_PPM + CO2_SUBSTEP_PPM);

/// Humidity step value (%RH).
pub const HUMIDITY_STEP_PCT: u8 = 20;

/// Humidity display saturation value (%RH).
pub const HUMIDITY_SATURATION_PCT: u8 = 90;

/// Encode a dashboard LED matrix frame (top to bottom).
#[must_use]
pub fn construct_dashboard_rows(co2: u16, humidity: u8, temp_f: i16) -> [Bitmap; LED_ROWS] {
    let mut dash_rows = [Bitmap::empty(LED_COLS); LED_ROWS];

    // Primary columns fill bottom to top
    for (i, row) in dash_rows.iter_mut().rev().enumerate() {
        if temp_f >= TEMP_BASE_F + TEMP_STEP_F * i.saturating_truncate::<u16>().cast_signed() {
            row.set(0);
        }

        if co2 >= CO2_BASE_PPM + CO2_STEP_PPM * i.saturating_truncate::<u16>() {
            row.set(2);
        }

        if humidity >= HUMIDITY_STEP_PCT * (i.saturating_truncate::<u8>() + 1)
            || humidity >= HUMIDITY_SATURATION_PCT
        {
            row.set(4);
        }
    }

    // Secondary columns fill top to bottom
    for (i, row) in dash_rows.iter_mut().enumerate() {
        if temp_f % TEMP_STEP_F > TEMP_SUBSTEP_F * i.saturating_truncate::<u16>().cast_signed()
            || temp_f >= TEMP_SATURATION_F
        {
            row.set(1);
        }

        if co2 % CO2_STEP_PPM > CO2_SUBSTEP_PPM * i.saturating_truncate::<u16>()
            || co2 >= CO2_SATURATION_PPM
        {
            row.set(3);
        }
    }

    dash_rows
}
