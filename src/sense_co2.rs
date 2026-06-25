use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use microbit_bsp::embassy_nrf::twim::Twim;
use rustymicrobit_moxi::measurement::{Co2Measurement, fahrenheit};
use rustymicrobit_moxi::power::{POWER_MODE, PowerMode};

use crate::sense_pa;

const CO2_CONSUMERS: usize = 3;
static CO2_LENS: Watch<ThreadModeRawMutex, Co2Measurement, CO2_CONSUMERS> = Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, Co2Measurement>> {
    CO2_LENS.dyn_receiver()
}

#[embassy_executor::task]
pub async fn sense_co2_task(i2c: I2cDevice<'static, NoopRawMutex, Twim<'static>>) {
    let mut scd = Scd4x::new(i2c, Delay);
    Timer::after_millis(50).await;

    // When re-programming, the controller will be restarted,
    // but not the sensor. We try to stop it in order to
    // prevent the rest of the commands failing.
    if let Err(e) = scd.stop_periodic_measurement().await {
        panic!("CO2 Sensor: Failed to stop periodic measurement ({e:?})");
    }

    get_device_info(&mut scd).await;
    set_temp_offset(&mut scd).await;
    set_polling(&mut scd).await;

    let tx = CO2_LENS.sender();
    if let Some(mut pa_rx) = sense_pa::get_sensor_receiver() {
        loop {
            if scd.data_ready().await.unwrap() {
                let m = scd.read_measurement().await.unwrap();
                let m_co2 = Co2Measurement::new(m.co2, m.humidity, m.temperature);
                info!(
                    "CO2: {=f32}, Humidity: {=f32}, Temperature: {=f32} C ({=f32} F)",
                    m_co2.co2,
                    m_co2.humidity,
                    m_co2.temp_c,
                    fahrenheit(m_co2.temp_c)
                );

                let m_pa = pa_rx.get().await;
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "hPa is 300-1250, well within u16; the cast saturates"
                )]
                if let Err(e) = scd.set_ambient_pressure(m_pa.hpa as u16).await {
                    error!("CO2 Sensor: Failed to set ambient pressure ({:?})", e);
                }

                tx.send(m_co2);
            }
            Timer::after(POWER_MODE.interval()).await;
        }
    }
}

async fn get_device_info(scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static>>, Delay>) {
    if let Ok(Some(variant)) = scd.sensor_variant().await {
        use libscd::SensorVariant::{Scd40, Scd41, Scd43};
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

async fn set_polling(scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static>>, Delay>) {
    match POWER_MODE {
        PowerMode::High => match scd.start_periodic_measurement().await {
            Ok(()) => {
                info!("CO2 Sensor: Initiated periodic measurement mode");
            }
            Err(e) => defmt::panic!(
                "CO2 Sensor: Failed to start periodic measurement mode ({:?})",
                e
            ),
        },
        PowerMode::Low => match scd.start_low_power_periodic_measurement().await {
            Ok(()) => {
                info!("CO2 Sensor: Initiated low-power periodic measurement mode");
            }
            Err(e) => defmt::panic!(
                "CO2 Sensor: Failed to start low-power periodic measurement mode ({:?})",
                e
            ),
        },
    }
}

async fn set_temp_offset(scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static>>, Delay>) {
    let offset = match POWER_MODE {
        PowerMode::High => 2.95,
        PowerMode::Low => 0.0,
    };

    let res = scd.set_temperature_offset(offset).await;
    if let Err(e) = res {
        panic!("CO2 Sensor: Failed to set temperature offset ({:?})", e);
    }
}
