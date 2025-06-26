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
    if let Err(e) = scd.stop_periodic_measurement().await {
        panic!("CO2 Sensor: Failed to stop periodic measurement ({:?})", e);
    }

    set_temp_offset(&mut scd).await;
    get_device_info(&mut scd).await;
    set_polling(&mut scd).await;

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

                let m_hpa = hpa_rx.get().await;
                let hpa = (m_hpa.pressure / 100.0) as u16; // read is in Pa
                if let Err(e) = scd.set_ambient_pressure(hpa).await {
                    error!("CO2 Sensor: Failed to set ambient pressure ({:?})", e);
                }

                tx.send(m);
            }
            Timer::after_millis(POWER_MODE as u64).await;
        }
    }
}

async fn get_device_info(
    scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static, TWISPI0>>, Delay>,
) {
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
}

async fn set_polling(
    scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static, TWISPI0>>, Delay>,
) {
    use crate::PowerMode::*;
    match POWER_MODE {
        High => match scd.start_periodic_measurement().await {
            Ok(_) => {
                info!("CO2 Sensor: Initiated periodic measurement mode");
            }
            Err(e) => defmt::panic!(
                "CO2 Sensor: Failed to start periodic measurement mode ({:?})",
                e
            ),
        },
        Low => match scd.start_low_power_periodic_measurement().await {
            Ok(_) => {
                info!("CO2 Sensor: Initiated low-power periodic measurement mode");
            }
            Err(e) => defmt::panic!(
                "CO2 Sensor: Failed to start low-power periodic measurement mode ({:?})",
                e
            ),
        },
    }
}

async fn set_temp_offset(
    scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static, TWISPI0>>, Delay>,
) {
    use crate::PowerMode::*;
    let offset = match POWER_MODE {
        High => 3.5,
        Low => 0.0,
    };
    if let Err(e) = scd.set_temperature_offset(offset).await {
        panic!("CO2 Sensor: Failed to set temperature offset ({:?})", e);
    }
}
