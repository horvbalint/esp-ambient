use std::{thread, time::Duration, sync::{Arc, Mutex}};

use embedded_svc::{
    http::{Method, Query},
};
use esp_idf_svc::{
    http::server::{self, EspHttpServer},
};
use serde::Deserialize;

use crate::{setup::WifiCredentials, utils::storage};
use crate::{utils::wifi, led::Led};

#[derive(Deserialize, Debug, Default)]
pub struct Color {
    hue: Option<f32>,
    saturation: Option<f32>,
    value: Option<f32>,
}

pub fn start(credentials: &WifiCredentials, led: Arc<Mutex<Led>>) -> anyhow::Result<()> {
    println!("Changing into lamp mode");

    // Connecting to wifi
    let _wifi = wifi::connect(&credentials.ssid, &credentials.password)?;

    // Setting up http server
    let server_config = server::Configuration::default();
    let mut server = server::EspHttpServer::new(&server_config)?;

    register_routes(&mut server, &led)?;
    println!("Lamp server ready, awaiting connections...");
    led.lock().unwrap().cycle_colors(Duration::from_secs(10));

    loop {
        if let Ok(mut led) = led.lock() {
            led.tick()?;
        }

        thread::sleep(Duration::from_millis(20));
    }
}

fn register_routes(server: &mut EspHttpServer, led: &Arc<Mutex<Led>>) -> anyhow::Result<()> {
    let led_clone = led.clone();
    server.fn_handler("/set", Method::Get, move |request| {
        let params = request.uri().trim_start_matches("/set?");
        let color: Color = serde_qs::from_str(params).unwrap_or_default();

        let mut led = led_clone.lock()?;
        
        if let Some(hue) = color.hue {
            led.stop_transition();
            led.set_hue(hue);
        }

        if let Some(saturation) = color.saturation {
            led.set_saturation(saturation);
        }

        if let Some(value) = color.value {
            led.set_value(value);
        }

        request.into_response(200, None, &[("Access-Control-Allow-Origin", "*")])?;

        Ok(())
    })?;

    let led_clone = led.clone();
    server.fn_handler("/cycle", Method::Get, move |request| {
        led_clone.lock()?.cycle_colors(Duration::from_secs(5));

        request.into_response(200, None, &[("Access-Control-Allow-Origin", "*")])?;

        Ok(())
    })?;

    server.fn_handler("/reset", Method::Get, move |request| {
        let mut storage = storage::new("app", true)?;

        storage.remove(crate::setup::WIFI_NVS_NAME)?;

        request.into_response(200, None, &[("Access-Control-Allow-Origin", "*")])?;
        esp_idf_hal::reset::restart();

        Ok(())
    })?;

    Ok(())
}