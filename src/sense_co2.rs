use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use libscd::measurement::Measurement;
use microbit_bsp::embassy_nrf::peripherals::TWISPI0;
use microbit_bsp::embassy_nrf::twim::Twim;

use crate::{POWER_MODE, sense_hpa};

const CO2_CONSUMERS: usize = 1;
static CO2_LENS: Watch<ThreadModeRawMutex, Measurement, CO2_CONSUMERS> = Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, Measurement>> {
    CO2_LENS.dyn_receiver()
}

#[embassy_executor::task]
pub async fn sense_co2_task(i2c: I2cDevice<'static, NoopRawMutex, Twim<'static, TWISPI0>>) {
    let mut scd = Scd4x::new(i2c, Delay);
    Timer::after_millis(50).await;

    // When re-programming, the controller will be restarted,
    // but not the sensor. We try to stop it in order to
    // prevent the rest of the commands failing.
    _ = scd.stop_periodic_measurement().await;

    if let Ok(Some(variant)) = scd.sensor_variant().await {
        use libscd::SensorVariant::*;
        match variant {
            Scd40 => info!("CO2 Sensor: SCD-40"),
            Scd41 => info!("CO2 Sensor: SCD-41"),
            Scd43 => info!("CO2 Sensor: SCD-43"),
            _ => info!("CO2 Sensor: Unknown"),
        }
        info!("CO2 Sensor SN: {:?}", scd.serial_number().await.unwrap());
    } else {
        error!("CO2 Sensor: Failed to read sensor");
    }

    let loop_delay = set_polling(&mut scd).await;

    let tx = CO2_LENS.sender();
    if let Some(mut hpa_rx) = sense_hpa::get_sensor_receiver() {
        loop {
            if scd.data_ready().await.unwrap() {
                let m = scd.read_measurement().await.unwrap();
                let temp_f = m.temperature * 9.0 / 5.0 + 32.0;

                info!(
                    "CO2: {}, Humidity: {}, Temperature: {} ({})",
                    m.co2, m.humidity as u16, m.temperature as u16, temp_f as u16
                );
                tx.send(m);

                let hpa = (hpa_rx.get().await.pressure / 100.0) as u16; // read is in Pa
                if let Err(e) = scd.set_ambient_pressure(hpa).await {
                    error!("CO2 Sensor: Failed to set ambient pressure: {:?}", e);
                }
            }
            Timer::after_millis(loop_delay).await;
        }
    }
}

async fn set_polling(
    scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static, TWISPI0>>, Delay>,
) -> u64 {
    use crate::PowerMode::*;
    match POWER_MODE {
        High => {
            if let Err(e) = scd.start_periodic_measurement().await {
                defmt::panic!(
                    "CO2 Sensor: Failed to start periodic measurement mode: {:?}",
                    e
                );
            } else {
                info!("CO2 Sensor: Initiated periodic measurement mode");
                5_000
            }
        }
        Low => {
            if let Err(e) = scd.start_low_power_periodic_measurement().await {
                defmt::panic!(
                    "CO2 Sensor: Failed to start low-power periodic measurement mode: {:?}",
                    e
                );
            } else {
                info!("CO2 Sensor: Initiated low-power periodic measurement mode");
                30_000
            }
        }
    }
}
