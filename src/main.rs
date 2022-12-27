use std::{thread, time::{Duration, Instant}};

use anyhow::Context;
use embedded_svc::{
    http::{Method, Query}, io::Write,
};
use esp_idf_sys as _; // If using the `binstart` feature of `esp-idf-sys`, always keep this module imported
use esp_idf_svc::{
    http::server::{self, EspHttpServer},
};
use esp_idf_hal::{peripherals::Peripherals};
use serde::Deserialize;
use smart_leds::hsv::{hsv2rgb, Hsv};

mod led;
mod wifi;

#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_pass: &'static str,
}

#[derive(Deserialize, Debug, Default)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

static mut LAST_COLOR: Option<Color> = None;

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_sys::link_patches();

    // Connecting to wifi
    let _wifi = wifi::connect(CONFIG.wifi_ssid, CONFIG.wifi_pass)?;

    // Setting up http server
    let server_config = server::Configuration::default();
    let mut server = server::EspHttpServer::new(&server_config)?;

    register_routes(&mut server)?;
    println!("Server ready, awaiting connections...");

    // Initing led and color
    let peripherals = Peripherals::take().context("Failed to take peripherals")?;
    let mut led = led::Led::new(peripherals)?;

    let mut color = Hsv {
        hue: 0,
        sat: 255,
        val: 255,
    };

    let start = Instant::now();

    // Main loop
    loop {
        if let Some(color) = unsafe {&LAST_COLOR} {
            led.set_rgb(color.r, color.g, color.b)?;

            thread::sleep(Duration::from_millis(1000));
        }
        else {
            let elapsed_secs = start.elapsed().as_millis() as f64 / 1000.;
            let progress = (elapsed_secs / 5.).fract();

            color.hue = (progress * 255.) as u8;

            let rgb = hsv2rgb(color);
            led.set_rgb(rgb.r, rgb.g, rgb.b)?;

            thread::sleep(Duration::from_millis(20));
        }
    }
}

fn register_routes(server: &mut EspHttpServer) -> anyhow::Result<()> {
    server.fn_handler("/", Method::Get, |request| {
        let params = request.uri().trim_start_matches("/?");

        let qs_color: Color = serde_qs::from_str(params)
            .unwrap_or_default();

        unsafe {
            LAST_COLOR = Some(qs_color);
        }

        let mut response = request.into_ok_response()?;
        let conn = response.connection();
        conn.write_all("Provide query string params r, g, b to set led color".as_bytes())?;

        Ok(())
    })?;

    server.fn_handler("/auto", Method::Get, |request| {
        unsafe {
            LAST_COLOR = None;
        }

        let mut response = request.into_ok_response()?;
        let conn = response.connection();
        conn.write_all("Color set to auto".as_bytes())?;

        Ok(())
    })?;

    Ok(())
}