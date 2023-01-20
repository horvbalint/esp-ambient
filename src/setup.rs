use std::{thread, time::{Duration}, sync::{Arc, Mutex, RwLock}};

use embedded_svc::{
    http::{Method, Query}, io::Write,
};
use esp_idf_svc::{
    http::server::{self, EspHttpServer},
};
use palette::{rgb::Rgb};
use serde::{Deserialize, Serialize};

use crate::{CONFIG, utils::{wifi, storage}, led::Led};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct WifiCredentials {
    pub ssid: String,
    pub password: String,
}

type Credentials = Arc<RwLock<Option<WifiCredentials>>>;


pub const WIFI_NVS_NAME: &str = "wifi_creds";

pub fn setup(led: Arc<Mutex<Led>>) -> anyhow::Result<WifiCredentials> {
    let mut storage = storage::new("app", true)?;
        
    // First we check if wifi credentials are already saved from previous setup
    if let Ok(Some(ref credentials)) = storage.get::<WifiCredentials>(WIFI_NVS_NAME) {
        println!("Found credentials: {credentials:?}");
        return Ok(credentials.clone());
    }

    // If not, we will get one through wifi, for that we create a wifif access point
    let mut wifi = wifi::start_access_point(CONFIG.wifi_ssid, CONFIG.wifi_pass)?;
    let mac_address = wifi.sta_netif().get_mac()?;
    let mac_address_hex = mac_address.map(|byte| format!("{byte:02X}")).join(":");

    // and set up a http server
    let server_config = server::Configuration::default();
    let mut server = server::EspHttpServer::new(&server_config)?;
    let credentials: Credentials = Default::default();

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

            return Ok(credentials.clone());
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

        let mut response = request.into_ok_response()?;
        response.write_all(mac_address.as_bytes())?;
        response.flush()?;
        response.release();
        
        *credentials.write()? = Some(qs_credentials);

        Ok(())
    })?;

    Ok(())
}