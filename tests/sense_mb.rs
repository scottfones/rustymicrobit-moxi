#![no_std]
#![no_main]

use defmt_rtt as _;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use microbit_bsp::Microbit;
    use microbit_bsp::embassy_nrf::bind_interrupts;
    use microbit_bsp::embassy_nrf::peripherals::TEMP;
    use microbit_bsp::embassy_nrf::temp::{self, Temp};

    bind_interrupts!(struct Irqs { TEMP => temp::InterruptHandler; });
    #[init]
    fn init() -> Microbit {
        Microbit::default()
    }

    #[test]
    fn ficr_serial_is_nonzero(_board: Microbit) {
        // SAFETY: FICR DEVICEID 0 is a read-only register, always mapped.
        let id0 = unsafe {
            core::ptr::read_volatile(core::ptr::with_exposed_provenance::<u32>(0x1000_0060))
        };
        defmt::assert_ne!(id0, 0);
    }

    #[test]
    async fn on_chip_temp(_board: Microbit) {
        // SAFETY: the bsp doesn't expose TEMP and it is unused elsewhere
        let mb_temp = unsafe { TEMP::steal() };
        let mut sensor = Temp::new(mb_temp, Irqs);
        let temp_celsius: f32 = sensor.read().await.to_num();
        defmt::assert!(temp_celsius > 18.0 && temp_celsius < 26.0);
    }
}
