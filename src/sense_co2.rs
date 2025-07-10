use defmt::{error, info};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use microbit_bsp::embassy_nrf::peripherals::TWISPI0;
use microbit_bsp::embassy_nrf::twim::Twim;

use crate::{POWER_MODE, sense_pa};

const CO2_CONSUMERS: usize = 3;
static CO2_LENS: Watch<ThreadModeRawMutex, Co2Measurement, CO2_CONSUMERS> = Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, Co2Measurement>> {
    CO2_LENS.dyn_receiver()
}

#[derive(Clone, Copy, Debug)]
pub struct Co2Measurement {
    pub co2: u16,
    pub humidity: u8,
    pub humidity_dec: u8,
    pub temp_c: i8,
    pub temp_c_dec: u8,
    pub temp_f: i8,
    pub temp_f_dec: u8,
}

impl Co2Measurement {
    pub fn new(co2: u16, humidity: f32, temp_c: f32) -> Self {
        let humidity_dec = 100.0 * (humidity - libm::truncf(humidity));
        let temp_c_dec = 100.0 * (temp_c - libm::truncf(temp_c));
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;
        let temp_f_dec = 100.0 * (temp_f - libm::truncf(temp_f));
        Self {
            co2,
            humidity: humidity as u8,
            humidity_dec: humidity_dec as u8,
            temp_c: temp_c as i8,
            temp_c_dec: temp_c_dec as u8,
            temp_f: temp_f as i8,
            temp_f_dec: temp_f_dec as u8,
        }
    }
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

    get_device_info(&mut scd).await;
    set_temp_offset(&mut scd).await;
    set_polling(&mut scd).await;

    let tx = CO2_LENS.sender();
    if let Some(mut pa_rx) = sense_pa::get_sensor_receiver() {
        loop {
            if scd.data_ready().await.unwrap() {
                let m = scd.read_measurement().await.unwrap();
                let cm = Co2Measurement::new(m.co2, m.humidity, m.temperature);
                info!(
                    "CO2: {}, Humidity: {}.{}, Temperature: {}.{} ({}.{})",
                    cm.co2,
                    cm.humidity,
                    cm.humidity_dec,
                    cm.temp_c,
                    cm.temp_c_dec,
                    cm.temp_f,
                    cm.temp_f_dec
                );

                let m_pa = pa_rx.get().await;
                if let Err(e) = scd.set_ambient_pressure(m_pa.hpa as u16).await {
                    error!("CO2 Sensor: Failed to set ambient pressure ({:?})", e);
                }

                tx.send(cm);
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
        High => 2.9,
        Low => 0.0,
    };
    if let Err(e) = scd.set_temperature_offset(offset).await {
        panic!("CO2 Sensor: Failed to set temperature offset ({:?})", e);
    }
}
