use core::fmt::Write;

use defmt::info;
use embassy_time::Duration;
use heapless::String;
use microbit_bsp::display::{Bitmap, Frame, LedMatrix};
use microbit_bsp::embassy_nrf::gpio::Output;

use crate::buttons::{ButtonState, get_buttons_receiver};
use crate::{sense_co2, sense_hpa};

const COLS: usize = 5;
const ROWS: usize = 5;

fn set_dash(co2: usize, humidity: usize, temp_f: usize) -> [Bitmap; ROWS] {
    let mut dash = [Bitmap::empty(COLS); ROWS];

    // set bits from bottom -> top
    for (i, row) in dash.iter_mut().rev().enumerate() {
        // set tens digit for temp [50F-90F]
        if temp_f >= (50 + 10 * i) {
            row.set(0);
        }

        // set co2 bit in buckets of 200ppm starting at 400ppm
        if co2 >= (400 + 200 * i) {
            row.set(2);
        }

        // set humidity in buckets of 20%, {[20,80], 90}
        if humidity >= (20 + 20 * i) || (i == 4 && humidity >= 90) {
            row.set(4);
        }
    }

    // set bits from top -> bottom
    for (i, row) in dash.iter_mut().enumerate() {
        // set temperature secondary bit for every 2F
        if temp_f % 10 > (2 * i) || temp_f > 99 {
            row.set(1);
        }

        // set co2 secondary bit in buckets of 40ppm
        if co2 % 200 > (40 * i) || co2 > 1400 {
            row.set(3);
        }
    }
    dash
}

async fn display_dash(
    co2: usize,
    humidity: usize,
    temp_f: usize,
    matrix: &mut LedMatrix<Output<'static>, ROWS, COLS>,
) {
    let dash = set_dash(co2, humidity, temp_f);
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
    let Some(mut co2_rx) = sense_co2::get_sensor_receiver() else {
        panic!("unable to get co2 sensor receiver");
    };
    let Some(mut hpa_rx) = sense_hpa::get_sensor_receiver() else {
        panic!("unable to get hpa sensor receiver");
    };

    loop {
        let libscd::measurement::Measurement { co2, humidity, .. } = co2_rx.get().await;
        let bmp5::Measurement {
            temperature: temp_c,
            ..
        } = hpa_rx.get().await;
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;

        match btn_rx.try_receive() {
            Ok(ButtonState::A) => {
                info!("Button A: Display Temp F");
                let (display_ms, units) = (2750, "F");
                display_specific(temp_f as u16, display_ms, &mut matrix, units).await;
            }
            Ok(ButtonState::B) => {
                info!("Button B: Display CO2 PPM");
                let (display_ms, units) = (4500, "ppm");
                display_specific(co2, display_ms, &mut matrix, units).await;
            }
            Ok(ButtonState::C) => {
                info!("Button C: Display Humidity %");
                let (display_ms, units) = (2750, "%");
                display_specific(humidity as u16, display_ms, &mut matrix, units).await;
            }
            // Only possible error is TryReceiveError, indicating an empty buffer
            Err(_) => {
                display_dash(
                    co2 as usize,
                    humidity as usize,
                    temp_f as usize,
                    &mut matrix,
                )
                .await;
            }
        }
    }
}
