//! Sense Task: BMP581 Pressure and Temperature.

use bmp5::i2c::{BMP5_ADDRESS, Bmp5};
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use microbit_bsp::embassy_nrf::twim::Twim;
use rustymicrobit_moxi::measurement::{PressureMeasurement, fahrenheit};
use rustymicrobit_moxi::power::POWER_MODE;

/// Retries before panic.
const INIT_ATTEMPTS_MAX: u8 = 3;

/// Count of receiving tasks (`display` and `sense_co2`).
const PRESSURE_CONSUMERS: usize = 2;

/// SPMC for pressure measurements.
static PRESSURE_LENS: Watch<ThreadModeRawMutex, PressureMeasurement, PRESSURE_CONSUMERS> =
    Watch::new();

const BMP5_CONFIG: bmp5::Config = bmp5::Config {
    temperature_oversampling: bmp5::Oversampling::Oversampling8X,
    temperature_iir_filter: bmp5::IIRFilter::Bypass,
    pressure_oversampling: bmp5::Oversampling::Oversampling8X,
    pressure_iir_filter: bmp5::IIRFilter::Bypass,
    output_data_rate: bmp5::OutputDataRate::OutputDataRate0_250Hz,
};

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, PressureMeasurement>> {
    PRESSURE_LENS.dyn_receiver()
}

/// BMP581 pressure sensing task.
#[embassy_executor::task]
pub async fn sense_pa_task(i2c: I2cDevice<'static, NoopRawMutex, Twim<'static>>) {
    let mut bmp = Bmp5::new(i2c, Delay, BMP5_ADDRESS, BMP5_CONFIG);
    Timer::after_millis(50).await;

    defmt::info!("Pressure Sensor: BMP581");

    for attempt in 1..=INIT_ATTEMPTS_MAX {
        match bmp.init().await {
            Ok(()) => {
                defmt::info!("Pressure Sensor: Initialized successfully");
                break;
            }
            Err(e) => {
                defmt::error!(
                    "Pressure Sensor init attempt {=u8} failed: {:?}",
                    attempt,
                    e
                );
                if attempt == INIT_ATTEMPTS_MAX {
                    defmt::panic!(
                        "Pressure Sensor failed to initialize after {=u8} attempts.",
                        INIT_ATTEMPTS_MAX
                    );
                }
            }
        }
    }

    let tx = PRESSURE_LENS.sender();
    loop {
        match bmp.measure().await {
            Ok(m) => {
                let m_pa = PressureMeasurement::new(m.pressure, m.temperature);
                defmt::info!(
                    "Pressure: {=f32} hPa, Temperature: {=f32} C ({=f32} F)",
                    m_pa.hpa,
                    m_pa.temp_c,
                    fahrenheit(m_pa.temp_c)
                );
                tx.send(m_pa);
            }
            Err(e) => defmt::error!("Pressure Sensor measurement failed: {:?}", e),
        }
        Timer::after(POWER_MODE.interval()).await;
    }
}
