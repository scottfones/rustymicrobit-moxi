#![no_std]
#![no_main]

use defmt::info;
use embassy_executor::Spawner;
use embassy_nrf::bind_interrupts;
use embassy_nrf::peripherals::TWISPI0;
use embassy_nrf::twim::{self, Twim};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting...");

    let p = embassy_nrf::init(Default::default());

    bind_interrupts!(struct Irqs {
        TWISPI0 => twim::InterruptHandler<TWISPI0>;
    });
    let i2c = Twim::new(p.TWISPI0, Irqs, p.P1_00, p.P0_26, Default::default());

    let mut scd = Scd4x::new(i2c, Delay);
    Timer::after_millis(50).await;

    // When re-programming, the controller will be restarted,
    // but not the sensor. We try to stop it in order to
    // prevent the rest of the commands failing.
    _ = scd.stop_periodic_measurement().await;

    info!("Sensor serial number: {:?}", scd.serial_number().await);
    if let Err(e) = scd.start_periodic_measurement().await {
        defmt::panic!("Failed to start periodic measurement: {:?}", e);
    }

    loop {
        if scd.data_ready().await.unwrap() {
            let m = scd.read_measurement().await.unwrap();
            let temp_f = m.temperature * 9.0 / 5.0 + 32.0;
            info!(
                "CO2: {}, Humidity: {}, Temperature: {} ({})",
                m.co2 as u16, m.humidity as u16, m.temperature as f32, temp_f as f32
            )
        }

        Timer::after_millis(10000).await;
    }
}
