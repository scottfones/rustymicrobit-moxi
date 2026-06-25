use bmp5::i2c::{BMP5_ADDRESS, Bmp5};
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use microbit_bsp::embassy_nrf::twim::Twim;
use rustymicrobit_moxi::measurement::{PressureMeasurement, fahrenheit};
use rustymicrobit_moxi::power::POWER_MODE;

const PRESSURE_CONSUMERS: usize = 3;
static PRESSURE_LENS: Watch<ThreadModeRawMutex, PressureMeasurement, PRESSURE_CONSUMERS> =
    Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, PressureMeasurement>> {
    PRESSURE_LENS.dyn_receiver()
}

const BMP5_CONFIG: bmp5::Config = bmp5::Config {
    temperature_oversampling: bmp5::Oversampling::Oversampling8X,
    temperature_iir_filter: bmp5::IIRFilter::Bypass,
    pressure_oversampling: bmp5::Oversampling::Oversampling8X,
    pressure_iir_filter: bmp5::IIRFilter::Bypass,
    output_data_rate: bmp5::OutputDataRate::OutputDataRate0_250Hz,
};

#[embassy_executor::task]
pub async fn sense_pa_task(i2c: I2cDevice<'static, NoopRawMutex, Twim<'static>>) {
    let mut bmp = Bmp5::new(i2c, Delay, BMP5_ADDRESS, BMP5_CONFIG);
    Timer::after_millis(50).await;

    info!("Pressure Sensor: BMP581");

    match bmp.init().await {
        Ok(()) => info!("Pressure Sensor: Initialized successfully"),
        Err(e) => {
            panic!("Pressure Sensor: Failed to initialize: {e:?}");
        }
    }
    Timer::after(POWER_MODE.interval()).await;

    let tx = PRESSURE_LENS.sender();
    loop {
        if let Ok(m) = bmp.measure().await {
            let m_pa = PressureMeasurement::new(m.pressure, m.temperature);
            info!(
                "Pressure: {=f32} hPa, Temperature: {=f32} C ({=f32} F)",
                m_pa.hpa,
                m_pa.temp_c,
                fahrenheit(m_pa.temp_c)
            );
            tx.send(m_pa);
        }
        Timer::after(POWER_MODE.interval()).await;
    }
}
