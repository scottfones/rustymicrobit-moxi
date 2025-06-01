#![no_std]
#![no_main]

mod sense_i2c;
mod sense_mb;

use defmt::info;
use embassy_executor::Spawner;
use {defmt_rtt as _, panic_probe as _};

use crate::sense_i2c::sense_i2c_task;
use crate::sense_mb::sense_mb_task;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting...");
    let p = embassy_nrf::init(Default::default());
    spawner.must_spawn(sense_i2c_task(p.TWISPI0, p.P1_00, p.P0_26));
    spawner.must_spawn(sense_mb_task(p.TEMP));
}
