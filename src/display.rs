use core::fmt::Write;

use embassy_time::Duration;
use heapless::String;
use microbit_bsp::display::{Bitmap, Frame, LedMatrix};
use microbit_bsp::embassy_nrf::gpio::Output;

use crate::sense_i2c;

const COLS: usize = 5;
const ROWS: usize = 5;

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

            let mut level = [Bitmap::empty(COLS); ROWS];
            for (i, row) in level.iter_mut().rev().enumerate() {
                if (sense_data.get_co2() as usize) > (400 + 200 * i) {
                    *row = Bitmap::new(0b0001_1111, COLS);
                } else {
                    break;
                }
            }
            matrix
                .display(Frame::new(level), Duration::from_millis(4000))
                .await;
        }
    }
}
