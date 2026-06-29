use core::fmt::Write;

use defmt::info;
use embassy_time::Duration;
use heapless::String;
use microbit_bsp::display::{Frame, LedMatrix};
use microbit_bsp::embassy_nrf::gpio::Output;
use rustymicrobit_moxi::dashboard::{LED_COLS, LED_ROWS, construct_dashboard_rows};
use rustymicrobit_moxi::measurement::{Co2Measurement, PressureMeasurement, fahrenheit};

use crate::buttons::{ButtonState, get_buttons_receiver};
use crate::{sense_co2, sense_pa};

async fn display_dash(
    co2: u16,
    humidity: u8,
    temp_f: i16,
    matrix: &mut LedMatrix<Output<'static>, LED_ROWS, LED_COLS>,
) {
    let dash = construct_dashboard_rows(co2, humidity, temp_f);
    matrix.set_brightness(microbit_bsp::display::Brightness::MIN);
    matrix
        .display(Frame::new(dash), Duration::from_millis(1000))
        .await;
}

async fn display_specific(
    data: u16,
    display_ms: u64,
    matrix: &mut LedMatrix<Output<'static>, LED_ROWS, LED_COLS>,
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
pub async fn display_task(mut matrix: LedMatrix<Output<'static>, LED_ROWS, LED_COLS>) {
    matrix.set_brightness(microbit_bsp::display::Brightness::MAX);
    matrix.scroll(" Power ON!").await;

    let btn_rx = get_buttons_receiver();
    let mut co2_rx = defmt::unwrap!(
        sense_co2::get_sensor_receiver(),
        "unable to get co2 sensor receiver"
    );
    let mut pa_rx = defmt::unwrap!(
        sense_pa::get_sensor_receiver(),
        "unable to get hpa sensor receiver"
    );

    loop {
        let Co2Measurement { co2, humidity, .. } = co2_rx.get().await;
        let PressureMeasurement { temp_c, .. } = pa_rx.get().await;

        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "values within bounds"
        )]
        let (co2_u16, humidity_u8, temp_f_i16) =
            (co2 as u16, humidity as u8, fahrenheit(temp_c) as i16);

        match btn_rx.try_receive() {
            Ok(ButtonState::A) => {
                info!("Button A: Display Temp F");
                let (display_ms, units) = (2750, "F");
                display_specific(temp_f_i16.cast_unsigned(), display_ms, &mut matrix, units).await;
            }
            Ok(ButtonState::B) => {
                info!("Button B: Display CO2 PPM");
                let (display_ms, units) = (4500, "ppm");
                display_specific(co2_u16, display_ms, &mut matrix, units).await;
            }
            Ok(ButtonState::C) => {
                info!("Button C: Display Humidity %");
                let (display_ms, units) = (2750, "%");
                display_specific(humidity_u8.into(), display_ms, &mut matrix, units).await;
            }
            // Only possible error is TryReceiveError, indicating an empty buffer
            Err(_) => {
                display_dash(co2_u16, humidity_u8, temp_f_i16, &mut matrix).await;
            }
        }
    }
}
