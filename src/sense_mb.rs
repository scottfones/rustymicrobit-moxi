use defmt::info;
use embassy_time::Timer;
use microbit_bsp::embassy_nrf::peripherals::TEMP;
use microbit_bsp::embassy_nrf::temp::Temp;
use microbit_bsp::embassy_nrf::{Peri, bind_interrupts, temp};

use crate::POWER_MODE;

// FICR (Factory Information Configuration Registers) base address
const FICR_BASE: u32 = 0x10000000;
const FICR_DEVICEID: [u32; 2] = [
    FICR_BASE + 0x060, // DEVICEID[0]
    FICR_BASE + 0x064, // DEVICEID[1]
];

fn get_serial_number() -> u32 {
    let device_id = unsafe {
        let deviceid0 = core::ptr::read_volatile(FICR_DEVICEID[0] as *const u32) as u64;
        let deviceid1 = core::ptr::read_volatile(FICR_DEVICEID[1] as *const u32) as u64;

        // Combine the two 32-bit values into a 64-bit device ID
        (deviceid1 << 32) | deviceid0
    };

    // The micro:bit serial number is typically the lower 32 bits formatted as
    // decimal
    (device_id & 0xFFFFFFFF) as u32
}

#[embassy_executor::task]
pub async fn sense_mb_task(p_temp: Peri<'static, TEMP>) {
    let serial_num = get_serial_number();
    info!("Microbit SN: {:?}", serial_num);

    bind_interrupts!(struct IrqsTemp {
        TEMP => temp::InterruptHandler;
    });
    let mut temp = Temp::new(p_temp, IrqsTemp);

    Timer::after_millis(POWER_MODE as u64).await;
    loop {
        let value = temp.read().await;
        let temp_c = value.to_num::<f32>();
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;

        info!("Microbit: {} ({})", temp_c as u16, temp_f as u16);
        Timer::after_millis(POWER_MODE as u64).await;
    }
}
