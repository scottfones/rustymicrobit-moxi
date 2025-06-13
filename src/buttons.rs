use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, DynamicReceiver, DynamicSender};
use embassy_time::Timer;
use microbit_bsp::Button;

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
}

#[embassy_executor::task]
pub async fn button_a_task(mut btn_a: Button) {
    let tx = get_buttons_sender();
    loop {
        btn_a.wait_for_low().await;
        tx.send(ButtonState::A).await;
        Timer::after_millis(250).await;
    }
}

#[embassy_executor::task]
pub async fn button_b_task(mut btn_b: Button) {
    let tx = get_buttons_sender();
    loop {
        btn_b.wait_for_low().await;
        tx.send(ButtonState::B).await;
        Timer::after_millis(250).await;
    }
}
