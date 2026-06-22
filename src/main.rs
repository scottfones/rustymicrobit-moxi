#![no_std]
#![no_main]

mod buttons;
mod display;
mod sense_co2;
mod sense_mb;
mod sense_pa;

use defmt::info;
use defmt_rtt as _;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use microbit_bsp::Microbit;
use microbit_bsp::embassy_nrf::peripherals::{P0_26, P1_00, P1_04, TEMP, TWISPI0};
use microbit_bsp::embassy_nrf::twim::Twim;
use microbit_bsp::embassy_nrf::{Peri, bind_interrupts, twim};
use panic_probe as _;
use static_cell::{ConstStaticCell, StaticCell};

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, Twim<'static>>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Power ON!");
    let b = Microbit::default();

    spawner.spawn(display::display_task(b.display).unwrap());

    let pin_temp = unsafe { TEMP::steal() };
    spawner.spawn(sense_mb::sense_mb_task(pin_temp).unwrap());

    let btn_touch = unsafe { P1_04::steal() };
    spawner.spawn(buttons::buttons_task(b.btn_a, b.btn_b, btn_touch.into()).unwrap());

    // I2C Tasks
    let i2c_bus = i2c_init(b.p19, b.p20, b.twispi0);

    let i2c_co2 = I2cDevice::new(i2c_bus);
    spawner.spawn(sense_co2::sense_co2_task(i2c_co2).unwrap());

    let i2c_hpa = I2cDevice::new(i2c_bus);
    spawner.spawn(sense_pa::sense_pa_task(i2c_hpa).unwrap());
}

fn i2c_init(
    scl: Peri<'static, P0_26>,
    sda: Peri<'static, P1_00>,
    twi: Peri<'static, TWISPI0>,
) -> &'static mut Mutex<NoopRawMutex, Twim<'static>> {
    bind_interrupts!(struct Irqs{
        TWISPI0 => twim::InterruptHandler<TWISPI0>;
    });
    static RAM_I2C_BUFFER: ConstStaticCell<[u8; 4]> = ConstStaticCell::new([0; 4]);

    let i2c_config = twim::Config::default();
    let i2c = Twim::new(twi, Irqs, sda, scl, i2c_config, RAM_I2C_BUFFER.take());
    let i2c_mutex = Mutex::new(i2c);
    I2C_BUS.init(i2c_mutex)
}
