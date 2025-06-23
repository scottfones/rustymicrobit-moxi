#![no_std]
#![no_main]

mod buttons;
mod display;
mod sense_co2;
mod sense_hpa;
mod sense_mb;

use defmt::info;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use microbit_bsp::Microbit;
use microbit_bsp::embassy_nrf::peripherals::TWISPI0;
use microbit_bsp::embassy_nrf::twim::Twim;
use microbit_bsp::embassy_nrf::{bind_interrupts, twim};
use static_cell::{ConstStaticCell, StaticCell};
use {defmt_rtt as _, panic_probe as _};

#[expect(unused)]
enum PowerMode {
    High,
    Low,
}
static POWER_MODE: PowerMode = PowerMode::Low;

static I2C_BUS: StaticCell<Mutex<NoopRawMutex, Twim<TWISPI0>>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let b = Microbit::default();
    spawner.must_spawn(sense_mb::sense_mb_task()); // steals TEMP
    spawner.must_spawn(display::display_task(b.display));
    spawner.must_spawn(buttons::buttons_task(b.btn_a, b.btn_b));
    spawner.must_spawn(buttons::touch_task()); // steals P1.04

    // I2C Tasks
    bind_interrupts!(struct Irqs{
        TWISPI0 => twim::InterruptHandler<TWISPI0>;
    });
    let i2c_config = twim::Config::default();
    static RAM_I2C_BUFFER: ConstStaticCell<[u8; 4]> = ConstStaticCell::new([0; 4]);
    let i2c = Twim::new(
        b.twispi0,
        Irqs,
        b.p20,
        b.p19,
        i2c_config,
        RAM_I2C_BUFFER.take(),
    );
    let i2c_bus = Mutex::new(i2c);
    let i2c_bus = I2C_BUS.init(i2c_bus);

    let i2c_co2 = I2cDevice::new(i2c_bus);
    spawner.must_spawn(sense_co2::sense_co2_task(i2c_co2));

    let i2c_hpa = I2cDevice::new(i2c_bus);
    spawner.must_spawn(sense_hpa::sense_hpa_task(i2c_hpa));
}
