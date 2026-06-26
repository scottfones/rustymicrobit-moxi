//! Sense Task: Microbit Temperature.

use embassy_time::Timer;
use microbit_bsp::embassy_nrf::peripherals::TEMP;
use microbit_bsp::embassy_nrf::temp::Temp;
use microbit_bsp::embassy_nrf::{Peri, bind_interrupts, temp};
use rustymicrobit_moxi::measurement::fahrenheit;
use rustymicrobit_moxi::power::POWER_MODE;

/// Lower 32 bits of FICR.
const FICR_DEVICEID_0: *const u32 = core::ptr::with_exposed_provenance(0x1000_0060);

/// Read microbit serial number (lower 32 bits of FICR).
fn get_serial_number() -> u32 {
    // SAFETY: FICR is read-only at a fixed address
    unsafe { core::ptr::read_volatile(FICR_DEVICEID_0) }
}

/// Microbit temperature sensing task.
#[embassy_executor::task]
pub async fn sense_mb_task(p_temp: Peri<'static, TEMP>) {
    let serial_num = get_serial_number();
    defmt::info!("Microbit SN: {:?}", serial_num);

    bind_interrupts!(struct IrqsTemp {
        TEMP => temp::InterruptHandler;
    });
    let mut mb_temp = Temp::new(p_temp, IrqsTemp);

    loop {
        let value = mb_temp.read().await;
        let temp_c = value.to_num::<f32>() - 1.8;

        defmt::info!("Microbit: {=f32} ({=f32})", temp_c, fahrenheit(temp_c));
        Timer::after(POWER_MODE.interval()).await;
    }
}
