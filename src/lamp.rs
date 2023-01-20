use std::{thread, time::Duration, sync::{Arc, Mutex}};

use embedded_svc::{
    http::{Method, Query}, io::Write,
};
use esp_idf_svc::{
    http::server::{self, EspHttpServer}, mdns::EspMdns,
};
use palette::rgb::Rgb;
use serde::Deserialize;
use serde_json::json;

use crate::{setup::WifiCredentials, utils::storage};
use crate::{utils::wifi, led::Led};

const CYCLE_DURATION: u64 = 30;

#[derive(Deserialize, Debug, Default)]
pub struct Color {
    hue: Option<f32>,
    sat: Option<f32>,
    val: Option<f32>,
}

pub fn start(credentials: WifiCredentials, led: Arc<Mutex<Led>>) -> anyhow::Result<()> {
    println!("Changing into lamp mode");

    // Connecting to wifi
    let _wifi = wifi::connect(&credentials.ssid, &credentials.password)?;

    // Setting up http server
    let server_config = server::Configuration::default();
    let mut server = server::EspHttpServer::new(&server_config)?;

    register_routes(&mut server, &led)?;
    println!("Lamp server ready, awaiting connections...");

    let mut mdns = EspMdns::take()?;
    mdns.set_hostname("esp32-c3")?;
    mdns.set_instance_name("Ambient lamp")?;
    mdns.add_service(None, "_lamp", "_tcp", 80, &[])?;
    println!("Service advertised using mDNS");

    led.lock().unwrap().set_rgb(Rgb::new(1., 0., 0.))?;
    led.lock().unwrap().cycle_colors(Duration::from_secs(CYCLE_DURATION));

    loop {
        led.lock().unwrap().tick()?;

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
            led.is_cycling = false;
            led.stop_transition();
            led.set_hue(hue);
        }

        if let Some(sat) = color.sat {
            led.set_saturation(sat);
        }

        if let Some(val) = color.val {
            led.set_value(val);
        }

        request.into_response(200, None, &[
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Allow-Private-Network", "true")
        ])?;

        Ok(())
    })?;

    let led_clone = led.clone();
    server.fn_handler("/cycle", Method::Get, move |request| {
        led_clone.lock()?.cycle_colors(Duration::from_secs(CYCLE_DURATION));

        let mut response = request.into_response(200, None, &[
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Allow-Private-Network", "true")
        ])?;
        response.write_all(CYCLE_DURATION.to_string().as_bytes())?;

        Ok(())
    })?;

    let led_clone = led.clone();
    server.fn_handler("/status", Method::Get, move |request| {
        let mut response = request.into_response(200, None, &[
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Allow-Private-Network", "true")
        ])?;

        let led = led_clone.lock()?;
        if led.is_cycling {
            let json = json!({
                "cycling": true,
                "hue": led.color.hue.to_positive_degrees(),
                "sat": led.color.saturation,
                "val": led.color.value,
                "duration": CYCLE_DURATION, 
            });

            response.write_all(json.to_string().as_bytes())?;
        }
        else {
            let json = json!({
                "cycling": false,
                "hue": led.color.hue.to_positive_degrees(),
                "sat": led.color.saturation,
                "val": led.color.value,
            });

            response.write_all(json.to_string().as_bytes())?;
        }

        response.flush()?;

        Ok(())
    })?;

    server.fn_handler("/reset", Method::Get, move |request| {
        let mut storage = storage::new("app", true)?;
        storage.remove(crate::setup::WIFI_NVS_NAME)?;

        request.into_response(200, None, &[
            ("Access-Control-Allow-Origin", "*"),
            ("Access-Control-Allow-Private-Network", "true")
        ])?;

        // we create a thread that will restart the device after a delay,
        // so that we have a chance to send the answer back to the request
        thread::spawn(|| {
            thread::sleep(Duration::from_secs(1));
            esp_idf_hal::reset::restart();
        });

        Ok(())
    })?;

    Ok(())
}