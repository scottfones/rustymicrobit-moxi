use defmt::info;
use embassy_nrf::peripherals::TEMP;
use embassy_nrf::temp::Temp;
use embassy_nrf::{Peri, bind_interrupts, temp};
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn sense_mb_task(p_temp: Peri<'static, TEMP>) {
    bind_interrupts!(struct IrqsTemp {
        TEMP => temp::InterruptHandler;
    });
    let mut temp = Temp::new(p_temp, IrqsTemp);

    loop {
        let value = temp.read().await;
        let temp_c = value.to_num::<f32>();
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;

        info!("Microbit: {} ({})", temp_c as u16, temp_f as u16);
        Timer::after_millis(10_000).await;
    }
}
