#![no_std]
#![no_main]

mod buttons;
mod display;
mod sense_co2;
mod sense_mb;
mod sense_pa;

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use microbit_bsp::Microbit;
use microbit_bsp::embassy_nrf::peripherals::{P0_26, P1_00, P1_04, TEMP, TWISPI0};
use microbit_bsp::embassy_nrf::twim::Twim;
use microbit_bsp::embassy_nrf::{Peri, bind_interrupts, twim};
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, panic_probe as _};

#[expect(unused)]
enum PowerMode {
    High = 5_000,
    Low = 30_000,
}
const POWER_MODE: PowerMode = PowerMode::Low;

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, Twim<TWISPI0>>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Power!!!");
    let b = Microbit::default();

    spawner.must_spawn(display::display_task(b.display));

    let pin_temp = unsafe { TEMP::steal() };
    spawner.must_spawn(sense_mb::sense_mb_task(pin_temp));

    let btn_touch = unsafe { P1_04::steal() };
    spawner.must_spawn(buttons::buttons_task(b.btn_a, b.btn_b, btn_touch.into()));

    // I2C Tasks
    let i2c_bus = create_shared_i2c_bus(b.p19, b.p20, b.twispi0);

    let i2c_co2 = I2cDevice::new(i2c_bus);
    spawner.must_spawn(sense_co2::sense_co2_task(i2c_co2));

    let i2c_hpa = I2cDevice::new(i2c_bus);
    spawner.must_spawn(sense_pa::sense_hpa_task(i2c_hpa));
}

fn create_shared_i2c_bus(
    scl: Peri<'static, P0_26>,
    sda: Peri<'static, P1_00>,
    twi: Peri<'static, TWISPI0>,
) -> &'static mut Mutex<NoopRawMutex, Twim<'static, TWISPI0>> {
    bind_interrupts!(struct Irqs{
        TWISPI0 => twim::InterruptHandler<TWISPI0>;
    });
    let i2c_config = twim::Config::default();
    static RAM_I2C_BUFFER: ConstStaticCell<[u8; 4]> = ConstStaticCell::new([0; 4]);
    let i2c = Twim::new(twi, Irqs, sda, scl, i2c_config, RAM_I2C_BUFFER.take());
    let i2c_mutex = Mutex::new(i2c);
    I2C_BUS.init(i2c_mutex)
}
