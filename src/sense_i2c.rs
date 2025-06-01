use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_nrf::peripherals::{P0_26, P1_00, TWISPI0};
use embassy_nrf::twim::Twim;
use embassy_nrf::{Peri, bind_interrupts, twim};
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use static_cell::StaticCell;

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, Twim<TWISPI0>>> = StaticCell::new();

#[embassy_executor::task]
pub async fn sense_i2c_task(
    twi: Peri<'static, TWISPI0>,
    sda: Peri<'static, P1_00>,
    scl: Peri<'static, P0_26>,
) {
    bind_interrupts!(struct IrqsCO2 {
        TWISPI0 => twim::InterruptHandler<TWISPI0>;
    });
    let i2c_config = twim::Config::default();
    let i2c = Twim::new(twi, IrqsCO2, sda, scl, i2c_config, &mut []);
    let i2c_bus = I2C_BUS.init(Mutex::new(i2c));

    let i2c_co2 = I2cDevice::new(i2c_bus);
    let mut scd = Scd4x::new(i2c_co2, Delay);
    Timer::after_millis(50).await;

    // When re-programming, the controller will be restarted,
    // but not the sensor. We try to stop it in order to
    // prevent the rest of the commands failing.
    _ = scd.stop_periodic_measurement().await;

    info!("Sensor serial number: {:?}", scd.serial_number().await);
    if let Err(e) = scd.start_periodic_measurement().await {
        defmt::panic!("Failed to start periodic measurement: {:?}", e);
    }

    loop {
        if scd.data_ready().await.unwrap() {
            let m = scd.read_measurement().await.unwrap();
            let temp_f = m.temperature * 9.0 / 5.0 + 32.0;
            info!(
                "CO2: {}, Humidity: {}, Temperature: {} ({})",
                m.co2 as u16, m.humidity as u16, m.temperature as u16, temp_f as u16
            )
        }

        Timer::after_millis(10_000).await;
    }
}
