use bmp5::i2c::{BMP5_ADDRESS, Bmp5};
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use microbit_bsp::embassy_nrf::peripherals::TWISPI0;
use microbit_bsp::embassy_nrf::twim::Twim;

use crate::POWER_MODE;

const PRESSURE_CONSUMERS: usize = 3;
static PRESSURE_LENS: Watch<ThreadModeRawMutex, PressureMeasurement, PRESSURE_CONSUMERS> =
    Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, PressureMeasurement>> {
    PRESSURE_LENS.dyn_receiver()
}

#[derive(Clone, Copy, Debug)]
pub struct PressureMeasurement {
    pub hpa: i32,
    pub hpa_dec: u8,
    pub temp_c: i8,
    pub temp_c_dec: u8,
    pub temp_f: i8,
    pub temp_f_dec: u8,
}

impl PressureMeasurement {
    pub fn new(pa: f32, temp_c: f32) -> Self {
        let hpa = pa / 100.0;
        let hpa_dec = 100.0 * (hpa - libm::truncf(hpa));
        let temp_c_dec = 100.0 * (temp_c - libm::truncf(temp_c));
        let temp_f = temp_c * 9.0 / 5.0 + 32.0;
        let temp_f_dec = 100.0 * (temp_f - libm::truncf(temp_f));
        Self {
            hpa: hpa as i32,
            hpa_dec: hpa_dec as u8,
            temp_c: temp_c as i8,
            temp_c_dec: temp_c_dec as u8,
            temp_f: temp_f as i8,
            temp_f_dec: temp_f_dec as u8,
        }
    }
}

const BMP5_CONFIG: bmp5::Config = bmp5::Config {
    temperature_oversampling: bmp5::Oversampling::Oversampling8X,
    temperature_iir_filter: bmp5::IIRFilter::Bypass,
    pressure_oversampling: bmp5::Oversampling::Oversampling8X,
    pressure_iir_filter: bmp5::IIRFilter::Bypass,
    output_data_rate: bmp5::OutputDataRate::OutputDataRate0_250Hz,
};

#[embassy_executor::task]
pub async fn sense_pa_task(i2c: I2cDevice<'static, NoopRawMutex, Twim<'static, TWISPI0>>) {
    let mut bmp = Bmp5::new(i2c, Delay, BMP5_ADDRESS, BMP5_CONFIG);
    Timer::after_millis(50).await;

    info!("Pressure Sensor: BMP581");

    match bmp.init().await {
        Ok(()) => info!("Pressure Sensor: Initialized successfully"),
        Err(e) => {
            panic!("Pressure Sensor: Failed to initialize: {:?}", e);
        }
    }
    Timer::after_millis(POWER_MODE as u64).await;

    let tx = PRESSURE_LENS.sender();
    loop {
        if let Ok(m) = bmp.measure().await {
            let pm = PressureMeasurement::new(m.pressure, m.temperature);
            info!(
                "Pressure: {}.{}, Temperature: {}.{} ({}.{})",
                pm.hpa, pm.hpa_dec, pm.temp_c, pm.temp_c_dec, pm.temp_f, pm.temp_f_dec
            );
            tx.send(pm);
        }
        Timer::after_millis(POWER_MODE as u64).await;
    }
}
