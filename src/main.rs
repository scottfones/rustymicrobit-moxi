#![no_std]
#![no_main]

mod display;
mod sense_i2c;
mod sense_mb;

use defmt::info;
use embassy_executor::Spawner;
use microbit_bsp::Microbit;
use {defmt_rtt as _, panic_probe as _};

use crate::display::display_task;
use crate::sense_i2c::sense_i2c_task;
use crate::sense_mb::sense_mb_task;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let b = Microbit::default();
    spawner.must_spawn(sense_i2c_task(b.twispi0, b.p20, b.p19));
    spawner.must_spawn(sense_mb_task());
    spawner.must_spawn(display_task(b.display));
}
