use std::sync::{Mutex, Arc};

use anyhow::Context;
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_hal::{peripherals::Peripherals};

mod utils;
mod setup;
mod lamp;

use utils::led;

#[derive(Debug)]
#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_pass: &'static str,
}

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    let peripherals = Peripherals::take().context("Failed to take peripherals")?;
    let led = led::Led::new(peripherals)?;
    let led = Arc::new(Mutex::new(led));

    setup::setup(led)
}