use defmt::info;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::{DynReceiver, Watch};
use embassy_time::{Delay, Timer};
use libscd::asynchronous::scd4x::Scd4x;
use microbit_bsp::embassy_nrf::peripherals::{P0_26, P1_00, TWISPI0};
use microbit_bsp::embassy_nrf::twim::Twim;
use microbit_bsp::embassy_nrf::{bind_interrupts, twim};

const SENSE_CONSUMERS: usize = 1;
static SENSOR_LENS: Watch<ThreadModeRawMutex, SenseData, SENSE_CONSUMERS> = Watch::new();

pub fn get_receiver() -> Option<DynReceiver<'static, SenseData>> {
    SENSOR_LENS.dyn_receiver()
}

#[derive(Clone, Copy)]
pub struct SenseData {
    co2: u16,
    humid: u16,
    temp_c: u16,
    temp_f: u16,
}

impl SenseData {
    fn new(co2: u16, humid: f32, temp_c: f32, temp_f: f32) -> Self {
        SenseData {
            co2,
            humid: (humid as u16),
            temp_c: (temp_c as u16),
            temp_f: (temp_f as u16),
        }
    }

    pub fn get_co2(&self) -> u16 {
        self.co2
    }

    pub fn get_humid(&self) -> u16 {
        self.humid
    }

    pub fn get_temp_c(&self) -> u16 {
        self.temp_c
    }

    pub fn get_temp_f(&self) -> u16 {
        self.temp_f
    }
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

    info!("Sensor serial number: {:?}", scd.serial_number().await);
    if let Err(e) = scd.start_periodic_measurement().await {
        defmt::panic!("Failed to start periodic measurement: {:?}", e);
    }

    let tx = SENSOR_LENS.sender();
    loop {
        if scd.data_ready().await.unwrap() {
            let m = scd.read_measurement().await.unwrap();
            let temp_f = m.temperature * 9.0 / 5.0 + 32.0;

            let sense_data = SenseData::new(m.co2, m.humidity, m.temperature, temp_f);

            info!(
                "CO2: {}, Humidity: {}, Temperature: {} ({})",
                sense_data.get_co2(),
                sense_data.get_humid(),
                sense_data.get_temp_c(),
                sense_data.get_temp_f()
            );
            tx.send(sense_data);
        }

        Timer::after_millis(10_000).await;
    }
}
