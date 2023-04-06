#![feature(result_option_inspect)]

use std::sync::{Arc, Mutex};

use anyhow::Context;
use esp_idf_hal::peripherals::Peripherals;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported

use esp32c3_utils::rgb_led;

mod lamp;
mod setup;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let mut peripherals = Peripherals::take().context("Failed to take peripherals")?;
    let pins = peripherals.pins;

    let led = rgb_led::Led::new(peripherals.ledc, pins.gpio0, pins.gpio1, pins.gpio2)?;
    let led = Arc::new(Mutex::new(led));

    let credentials = setup::setup(&mut peripherals.modem, led.clone())?;
    lamp::start(&mut peripherals.modem, credentials, led)
}
