use bmp5::Measurement;
use bmp5::i2c::{BMP5_ADDRESS, Bmp5};
use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_sync::blocking_mutex::raw::{NoopRawMutex, ThreadModeRawMutex};
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use microbit_bsp::embassy_nrf::peripherals::TWISPI0;
use microbit_bsp::embassy_nrf::twim::Twim;

use crate::POWER_MODE;

const PRESSURE_CONSUMERS: usize = 1;
static PRESSURE_LENS: Watch<ThreadModeRawMutex, Measurement, PRESSURE_CONSUMERS> = Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, Measurement>> {
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
pub async fn sense_hpa_task(i2c: I2cDevice<'static, NoopRawMutex, Twim<'static, TWISPI0>>) {
    let mut bmp = Bmp5::new(i2c, Delay, BMP5_ADDRESS, BMP5_CONFIG);
    Timer::after_millis(50).await;

    info!("Pressure Sensor: BMP581");

    match bmp.init().await {
        Ok(()) => info!("Pressure Sensor: Initialized successfully"),
        Err(e) => {
            panic!("Pressure Sensor: Failed to initialize: {:?}", e);
        }
    }

    use crate::PowerMode::*;
    let loop_delay = match POWER_MODE {
        High => 5_000,
        Low => 30_000,
    };

    let tx = PRESSURE_LENS.sender();
    loop {
        if let Ok(m) = bmp.measure().await {
            let hpa = m.pressure / 100.0;
            let temp_f = m.temperature * 9.0 / 5.0 + 32.0;
            info!(
                "Pressure: {}, Temperature: {} ({})",
                hpa as u16, m.temperature as u16, temp_f as u16
            );
            tx.send(m);
        }
        Timer::after_millis(loop_delay).await;
    }
}
