use embassy_futures::select::{Either, select};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_time::Timer;
use microbit_bsp::Button;
use microbit_bsp::embassy_nrf::Peri;
use microbit_bsp::embassy_nrf::gpio::{AnyPin, Input, Pull};
use microbit_bsp::embassy_nrf::peripherals::P1_04;

static BUTTONS_LENS: Channel<ThreadModeRawMutex, ButtonState, 3> = Channel::new();

pub fn get_buttons_receiver() -> DynamicReceiver<'static, ButtonState> {
    BUTTONS_LENS.dyn_receiver()
}

fn get_buttons_sender() -> DynamicSender<'static, ButtonState> {
    BUTTONS_LENS.dyn_sender()
}

#[derive(Clone, Copy, Debug)]
pub enum ButtonState {
    A,
    B,
    C,
}

#[embassy_executor::task]
pub async fn buttons_task(mut btn_a: Button, mut btn_b: Button) {
    let tx = get_buttons_sender();
    loop {
        match select(btn_a.wait_for_falling_edge(), btn_b.wait_for_falling_edge()).await {
            Either::First(_) => {
                tx.send(ButtonState::A).await;
            }
            Either::Second(_) => {
                tx.send(ButtonState::B).await;
            }
        }
        Timer::after_millis(500).await;
    }
}

#[embassy_executor::task]
pub async fn touch_task() {
    let tx = get_buttons_sender();
    unsafe {
        let touch_peri = P1_04::steal();
        let touch_any: Peri<'static, AnyPin> = touch_peri.into();
        let mut touch_input = Input::new(touch_any, Pull::None);

        loop {
            touch_input.wait_for_falling_edge().await;
            tx.send(ButtonState::C).await;
            Timer::after_millis(500).await;
        }
    }
}
