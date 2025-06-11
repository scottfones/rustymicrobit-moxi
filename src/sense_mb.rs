use defmt::info;
use embassy_time::Timer;
use microbit_bsp::embassy_nrf::peripherals::TEMP;
use microbit_bsp::embassy_nrf::temp::Temp;
use microbit_bsp::embassy_nrf::{bind_interrupts, temp};

#[embassy_executor::task]
pub async fn sense_mb_task() {
    bind_interrupts!(struct IrqsTemp {
        TEMP => temp::InterruptHandler;
    });
    unsafe {
        let p_temp = TEMP::steal();
        let mut temp = Temp::new(p_temp, IrqsTemp);

        loop {
            let value = temp.read().await;
            let temp_c = value.to_num::<f32>();
            let temp_f = temp_c * 9.0 / 5.0 + 32.0;

            info!("Microbit: {} ({})", temp_c as u16, temp_f as u16);
            Timer::after_millis(30_000).await;
        }
    }
}
