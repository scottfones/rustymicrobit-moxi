//! Sense Task: SCD4X CO2, Humidity, and Temperature.

use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Sender, Watch};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use microbit_bsp::embassy_nrf::twim::Twim;
use rustymicrobit_moxi::measurement::{Co2Measurement, PressureMeasurement, fahrenheit};
use rustymicrobit_moxi::power::{POWER_MODE, PowerMode};

use crate::sense_pa;

/// Retries before panic.
const INIT_ATTEMPTS_MAX: u8 = 3;

/// Count of receiving tasks [`display`].
const CO2_CONSUMERS: usize = 1;

/// SPMC for pressure measurements.
static CO2_LENS: Watch<ThreadModeRawMutex, Co2Measurement, CO2_CONSUMERS> = Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, Co2Measurement>> {
    CO2_LENS.dyn_receiver()
}

/// SCD4X CO2, humidity, and temperature sensing task.
#[embassy_executor::task]
pub async fn sense_co2_task(i2c: I2cDevice<'static, NoopRawMutex, Twim<'static>>) {
    let mut scd = Scd4x::new(i2c, Delay);

    // Power on delay (30ms per datasheet, 50ms for margin)
    Timer::after_millis(50).await;

    for init_attempt in 1..=INIT_ATTEMPTS_MAX {
        match scd.stop_periodic_measurement().await {
            Ok(()) => break,
            Err(e) => {
                defmt::error!(
                    "CO2 Sensor: Stop/Init attempt {=u8} failed ({:?})",
                    init_attempt,
                    e
                );

                if init_attempt == INIT_ATTEMPTS_MAX {
                    defmt::panic!(
                        "CO2 Sensor: Failed to stop/initialize after {=u8} attempts",
                        INIT_ATTEMPTS_MAX
                    );
                }
                Timer::after_millis(250).await;
            }
        }
    }

    get_device_info(&mut scd).await;
    set_temp_offset(&mut scd).await;
    set_polling(&mut scd).await;

    let co2_tx = CO2_LENS.sender();
    let mut pa_rx = sense_pa::get_sensor_receiver().or_else(|| {
        defmt::error!("CO2 Sensor: Request for pressure rx failed (tracking disabled)");
        None
    });

    loop {
        match scd.data_ready().await {
            Ok(true) => get_measurement(&co2_tx, &mut pa_rx, &mut scd).await,
            Ok(false) => defmt::trace!("CO2 Sensor: No unread data"),
            Err(e) => defmt::error!("CO2 Sensor: Failed device ready probe ({:?})", e),
        }
        Timer::after(POWER_MODE.interval()).await;
    }
}

/// Query SCD4X variant and serial number.
async fn get_device_info(scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static>>, Delay>) {
    if let Ok(Some(variant)) = scd.sensor_variant().await {
        match variant {
            libscd::SensorVariant::Scd40 => defmt::info!("CO2 Sensor: SCD-40"),
            libscd::SensorVariant::Scd41 => defmt::info!("CO2 Sensor: SCD-41"),
            libscd::SensorVariant::Scd43 => defmt::info!("CO2 Sensor: SCD-43"),
            _ => defmt::info!("CO2 Sensor: Unknown"),
        }
        match scd.serial_number().await {
            Ok(sn) => defmt::info!("CO2 Sensor SN: {:?}", sn),
            Err(e) => defmt::error!("CO2 Sensor: Failed to read SN ({:?})", e),
        }
    } else {
        defmt::error!("CO2 Sensor: Failed to read sensor");
    }
}

/// Read SCD4X sensor data.
async fn get_measurement(
    co2_tx: &Sender<'_, ThreadModeRawMutex, Co2Measurement, CO2_CONSUMERS>,
    pa_rx: &mut Option<DynReceiver<'_, PressureMeasurement>>,
    scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static>>, Delay>,
) {
    match scd.read_measurement().await {
        Ok(m) => {
            let m_co2 = Co2Measurement::new(m.co2, m.humidity, m.temperature);
            defmt::info!(
                "CO2: {=f32}, Humidity: {=f32}, Temperature: {=f32} C ({=f32} F)",
                m_co2.co2,
                m_co2.humidity,
                m_co2.temp_c,
                fahrenheit(m_co2.temp_c)
            );

            if let Some(m_pa) = pa_rx.as_mut().and_then(|rx| rx.try_get()) {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "hPa is [300, 1250]"
                )]
                let hpa_ambient = m_pa.hpa as u16;
                if let Err(e) = scd.set_ambient_pressure(hpa_ambient).await {
                    defmt::error!("CO2 Sensor: Failed to set pressure ({:?})", e);
                }
            }
            co2_tx.send(m_co2);
        }
        Err(e) => defmt::error!("CO2 Sensor: Read failed ({:?})", e),
    }
}

/// Set SCD4X polling according to `POWER_MODE`.
async fn set_polling(scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static>>, Delay>) {
    match POWER_MODE {
        PowerMode::High => match scd.start_periodic_measurement().await {
            Ok(()) => {
                defmt::info!("CO2 Sensor: Initiated periodic measurement mode");
            }
            Err(e) => defmt::panic!(
                "CO2 Sensor: Failed to start periodic measurement mode ({:?})",
                e
            ),
        },
        PowerMode::Low => match scd.start_low_power_periodic_measurement().await {
            Ok(()) => {
                defmt::info!("CO2 Sensor: Initiated low-power periodic measurement mode");
            }
            Err(e) => defmt::panic!(
                "CO2 Sensor: Failed to start low-power periodic measurement mode ({:?})",
                e
            ),
        },
    }
}

/// Set SCD4X temperature reading offset.
async fn set_temp_offset(scd: &mut Scd4x<I2cDevice<'static, NoopRawMutex, Twim<'static>>, Delay>) {
    /// Offset from BMP581 in High Power mode.
    const OFFSET_BMP581: f32 = 2.949;

    let offset = match POWER_MODE {
        PowerMode::High => OFFSET_BMP581,
        PowerMode::Low => 0.0,
    };

    match scd.get_temperature_offset().await {
        Ok(previous_offset) => defmt::debug!(
            "CO2 Sensor: Setting temperature offset (old: {=f32} C, new: {=f32} C)",
            previous_offset,
            offset
        ),
        Err(e) => defmt::panic!("CO2 Sensor: Failed to probe temperature offset ({:?})", e),
    }

    if let Err(e) = scd.set_temperature_offset(offset).await {
        defmt::panic!("CO2 Sensor: Failed to set temperature offset ({:?})", e);
    }
}
