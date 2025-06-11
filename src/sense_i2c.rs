use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use libscd::measurement::Measurement;
use microbit_bsp::embassy_nrf::peripherals::{P0_26, P1_00, TWISPI0};
use microbit_bsp::embassy_nrf::twim::Twim;
use microbit_bsp::embassy_nrf::{bind_interrupts, twim};

const SENSE_CONSUMERS: usize = 1;
static SENSOR_LENS: Watch<ThreadModeRawMutex, Measurement, SENSE_CONSUMERS> = Watch::new();

pub fn get_sensor_receiver() -> Option<DynReceiver<'static, Measurement>> {
    SENSOR_LENS.dyn_receiver()
}

#[embassy_executor::task]
pub async fn sense_i2c_task(twi: TWISPI0, sda: P1_00, scl: P0_26) {
    bind_interrupts!(struct IrqsCO2 {
        TWISPI0 => twim::InterruptHandler<TWISPI0>;
    });
    let i2c_config = twim::Config::default();
    let i2c = Twim::new(twi, IrqsCO2, sda, scl, i2c_config);

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
    }

    info!("CO2 Sensor SN: {:?}", scd.serial_number().await.unwrap());

    if let Err(e) = scd.start_low_power_periodic_measurement().await {
        defmt::panic!(
            "CO2 Sensor: Failed to start low-power periodic measurement mode: {:?}",
            e
        );
    } else {
        info!("CO2 Sensor: Initiated low-power periodic measurement mode");
    }

    let tx = SENSOR_LENS.sender();
    loop {
        if scd.data_ready().await.unwrap() {
            let m = scd.read_measurement().await.unwrap();
            let temp_f = m.temperature * 9.0 / 5.0 + 32.0;

            info!(
                "CO2: {}, Humidity: {}, Temperature: {} ({})",
                m.co2, m.humidity as u16, m.temperature as u16, temp_f as u16
            );
            tx.send(m);
        }
        Timer::after_millis(30_000).await;
    }
}
