use std::{thread, time::{Duration}, sync::{Arc, Mutex, RwLock}};

use embedded_svc::{
    http::{Method, Query}, io::Write,
};
use esp_idf_svc::{
    http::server::{self, EspHttpServer},
};
use palette::{rgb::Rgb};
use serde::{Deserialize, Serialize};

use crate::{CONFIG, utils::{wifi, storage}, lamp, led::Led};

#[derive(Deserialize, Serialize, Debug)]
pub struct WifiCredentials {
    pub ssid: String,
    pub password: String,
}

type Credentials = Arc<RwLock<Option<WifiCredentials>>>;


pub const WIFI_NVS_NAME: &str = "wifi_creds";

pub fn setup(led: Arc<Mutex<Led>>) -> anyhow::Result<()> {
    let mut storage = storage::new("app", true)?;
        
    // First we check if wifi credentials are already saved from previous setup
    if let Ok(Some(ref credentials)) = storage.get(WIFI_NVS_NAME) {
        println!("Found credentials: {credentials:?}");
        drop(storage);

        return lamp::start(credentials, led);
    }
    println!("No previous credentials found");

    // If not, we will get one from the app
    let credentials: Credentials = Default::default();

    // Creating wifi access point
    let mut wifi = wifi::start_access_point(CONFIG.wifi_ssid, CONFIG.wifi_pass)?;
    let mac_address = wifi.sta_netif().get_mac()?;
    let mac_address_hex = mac_address.map(|byte| format!("{byte:02X}")).join(":");
    dbg!(&mac_address_hex);

    // Setting up http server
    let server_config = server::Configuration::default();
    let mut server = server::EspHttpServer::new(&server_config)?;

    register_routes(&mut server, &credentials, mac_address_hex)?;
    println!("Setup server ready, awaiting connections");

    let mut locked_led = led.lock().unwrap();
    locked_led.set_rgb(Rgb::new(0., 0., 1.))?;
    locked_led.pulse(Duration::from_secs(2));

    loop {
        if let Some(credentials) = credentials.read().unwrap().as_ref() {
            wifi.stop()?;

            locked_led.stop_transition();
            locked_led.set_rgb(Rgb::new(0., 0., 0.))?;
            
            storage.set(WIFI_NVS_NAME, credentials)
                .map_err(|err| {dbg!(&err); err}).ok();

            drop(wifi);
            drop(server);
            drop(locked_led);
            drop(storage);

            return lamp::start(credentials, led);
        }

        locked_led.tick()?;
        thread::sleep(Duration::from_millis(20));
    }
}

fn register_routes(server: &mut EspHttpServer, credentials: &Credentials, mac_address: String) -> anyhow::Result<()> {
    let credentials = credentials.clone();
    server.fn_handler("/connect", Method::Get, move |request| {
        let params = request.uri().trim_start_matches("/connect?");
        let qs_credentials: WifiCredentials = serde_qs::from_str(params)?;

        *credentials.write()? = Some(qs_credentials);
        
        let mut response = request.into_ok_response()?;
        response.write_all(mac_address.as_bytes())?;
        response.flush()?;
        response.release();

        Ok(())
    })?;

    Ok(())
}