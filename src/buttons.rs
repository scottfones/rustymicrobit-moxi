use embassy_futures::select::{Either3, select3};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_time::Timer;
use microbit_bsp::Button;
use microbit_bsp::embassy_nrf::Peri;
use microbit_bsp::embassy_nrf::gpio::{AnyPin, Input, Pull};

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
pub async fn buttons_task(
    mut btn_a: Button,
    mut btn_b: Button,
    btn_touch_any: Peri<'static, AnyPin>,
) {
    let mut btn_touch = Input::new(btn_touch_any, Pull::None);

    let tx = get_buttons_sender();
    loop {
        match select3(
            btn_a.wait_for_falling_edge(),
            btn_b.wait_for_falling_edge(),
            btn_touch.wait_for_falling_edge(),
        )
        .await
        {
            Either3::First(_) => {
                tx.send(ButtonState::A).await;
            }
            Either3::Second(_) => {
                tx.send(ButtonState::B).await;
            }
            Either3::Third(_) => {
                tx.send(ButtonState::C).await;
            }
        }
        Timer::after_millis(500).await;
    }
}
