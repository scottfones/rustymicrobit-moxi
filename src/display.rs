use core::fmt::Write;

use embassy_time::Duration;
use heapless::String;
use microbit_bsp::display::{Bitmap, Frame, LedMatrix};
use microbit_bsp::embassy_nrf::gpio::Output;

use crate::sense_i2c::{self, SenseData};

const COLS: usize = 5;
const ROWS: usize = 5;

fn set_dash(sense_data: &SenseData) -> [Bitmap; ROWS] {
    let mut dash = [Bitmap::empty(COLS); ROWS];

    // set bits from bottom -> top
    for (i, row) in dash.iter_mut().rev().enumerate() {
        // set tens digit for temp [60F-90F]
        if (sense_data.get_temp_f() as usize) > (50 + 10 * i) {
            row.set(0);
        }

        // set co2 bit in buckets of 200ppm starting at 400ppm
        if (sense_data.get_co2() as usize) > (400 + 200 * i) {
            row.set(2);
        }

        // set humidity in bucket of 20%
        if (sense_data.get_humid() as usize) > (20 * i) {
            row.set(4);
        }
    }

    // set bits from top -> bottom
    for (i, row) in dash.iter_mut().enumerate() {
        // set temperature secondary bit for every 2F
        if (sense_data.get_temp_f() as usize) % 10 > (2 * i) {
            row.set(1);
        }

        // set co2 secondary bit in buckets of 40ppm
        if (sense_data.get_co2() as usize) % 200 > (40 * i) {
            row.set(3);
        }
    }

    dash
}

#[embassy_executor::task]
pub async fn display_task(mut matrix: LedMatrix<Output<'static>, ROWS, COLS>) {
    if let Some(mut rx) = sense_i2c::get_receiver() {
        let mut disp_txt: String<6> = String::new();
        loop {
            let sense_data = rx.get().await;
            write!(&mut disp_txt, " {}", sense_data.get_co2()).unwrap();

            matrix
                .scroll_with_speed(disp_txt.as_str(), Duration::from_millis(1750))
                .await;
            disp_txt.clear();

            let dash = set_dash(&sense_data);

            matrix
                .display(Frame::new(dash), Duration::from_millis(10_000))
                .await;
        }
    }
}
