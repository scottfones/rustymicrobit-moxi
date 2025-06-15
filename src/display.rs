use core::fmt::Write;

use defmt::info;
use embassy_time::Duration;
use heapless::String;
use libscd::measurement::Measurement;
use microbit_bsp::display::{Bitmap, Frame, LedMatrix};
use microbit_bsp::embassy_nrf::gpio::Output;

use crate::buttons::{ButtonState, get_buttons_receiver};
use crate::sense_i2c;

const COLS: usize = 5;
const ROWS: usize = 5;

fn c_to_f(c: f32) -> f32 {
    c * 9.0 / 5.0 + 32.0
}

fn set_dash(m: &Measurement) -> [Bitmap; ROWS] {
    let mut dash = [Bitmap::empty(COLS); ROWS];
    let temp_f = c_to_f(m.temperature);

    // set bits from bottom -> top
    for (i, row) in dash.iter_mut().rev().enumerate() {
        // set tens digit for temp [60F-90F]
        if (temp_f as usize) > (50 + 10 * i) {
            row.set(0);
        }

        // set co2 bit in buckets of 200ppm starting at 400ppm
        if (m.co2 as usize) > (400 + 200 * i) {
            row.set(2);
        }

        // set humidity in buckets of 20%, {[20,80], 90}
        if (m.humidity as usize) >= (20 + 20 * i) || (i == 4 && (m.humidity as usize) >= 90) {
            row.set(4);
        }
    }

    // set bits from top -> bottom
    for (i, row) in dash.iter_mut().enumerate() {
        // set temperature secondary bit for every 2F
        if (temp_f as usize) % 10 > (2 * i) {
            row.set(1);
        }

        // set co2 secondary bit in buckets of 40ppm
        if (m.co2 as usize) % 200 > (40 * i) {
            row.set(3);
        }
    }
    dash
}

async fn display_dash(matrix: &mut LedMatrix<Output<'static>, ROWS, COLS>, m: &Measurement) {
    let dash = set_dash(m);
    matrix.set_brightness(microbit_bsp::display::Brightness::MIN);
    matrix
        .display(Frame::new(dash), Duration::from_millis(1000))
        .await;
}

async fn display_specific(
    data: u16,
    display_ms: u64,
    matrix: &mut LedMatrix<Output<'static>, ROWS, COLS>,
    units: &str,
) {
    let mut disp_txt: String<9> = String::new();
    write!(&mut disp_txt, " {data} {units}").unwrap();
    matrix.set_brightness(microbit_bsp::display::Brightness::MAX);
    matrix
        .scroll_with_speed(disp_txt.as_str(), Duration::from_millis(display_ms))
        .await;
    disp_txt.clear();
}

#[embassy_executor::task]
pub async fn display_task(mut matrix: LedMatrix<Output<'static>, ROWS, COLS>) {
    matrix.set_brightness(microbit_bsp::display::Brightness::MAX);
    matrix.scroll(" Power!").await;

    let btn_rx = get_buttons_receiver();
    if let Some(mut sense_rx) = sense_i2c::get_sensor_receiver() {
        loop {
            let m = sense_rx.get().await;

            match btn_rx.try_receive() {
                Ok(ButtonState::A) => {
                    info!("Button A: Display Temp F");
                    let (display_ms, units) = (2750, "F");
                    let temp_f = c_to_f(m.temperature) as u16;
                    display_specific(temp_f, display_ms, &mut matrix, units).await;
                    continue;
                }
                Ok(ButtonState::B) => {
                    info!("Button B: Display CO2 PPM");
                    let (display_ms, units) = (4500, "ppm");
                    display_specific(m.co2, display_ms, &mut matrix, units).await;
                    continue;
                }
                // Only possible error is TryReceiveError, indicating an empty buffer
                Err(_) => {
                    display_dash(&mut matrix, &m).await;
                }
            }
        }
    }
}
