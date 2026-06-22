#![no_std]
#![no_main]

use defmt_rtt as _;

#[cfg(test)]
#[embedded_test::tests]
mod tests {
    use microbit_bsp::Microbit;

    #[init]
    fn init() -> Microbit {
        Microbit::default()
    }

    #[test]
    fn ficr_serial_is_nonzero(_board: Microbit) {
        // SAFETY: FICR DEVICEID 0 is a read-only register, always mapped.
        let id0 = unsafe { core::ptr::read_volatile(0x1000_0060 as *const u32) };
        assert_ne!(id0, 0);
    }
}
