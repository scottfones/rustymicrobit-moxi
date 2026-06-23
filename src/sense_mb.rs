use embassy_time::Timer;
use microbit_bsp::embassy_nrf::peripherals::TEMP;
use microbit_bsp::embassy_nrf::temp::Temp;
use microbit_bsp::embassy_nrf::{Peri, bind_interrupts, temp};
use rustymicrobit_moxi::power::POWER_MODE;

/// Lower 32 bits of FICR.
const FICR_DEVICEID_0: *const u32 = core::ptr::with_exposed_provenance(0x1000_0060);

/// Read microbit serial number (lower 32 bits of FICR).
fn get_serial_number() -> u32 {
    // SAFETY: FICR is read-only at a fixed address
    unsafe { core::ptr::read_volatile(FICR_DEVICEID_0) }
}

/// Initialize the microbit and loop temperature reads.
#[embassy_executor::task]
pub async fn sense_mb_task(p_temp: Peri<'static, TEMP>) {
    let serial_num = get_serial_number();
    defmt::info!("Microbit SN: {:?}", serial_num);

    bind_interrupts!(struct IrqsTemp {
        TEMP => temp::InterruptHandler;
    });
    let mut temp = Temp::new(p_temp, IrqsTemp);

    Timer::after(POWER_MODE.interval()).await;
    loop {
        let value = temp.read().await;
        let temp_c = value.to_num::<f32>();
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;

        defmt::info!("Microbit: {=f32} ({=f32})", temp_c, temp_f);
        Timer::after(POWER_MODE.interval()).await;
    }
}
