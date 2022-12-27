use embedded_svc::wifi::{Configuration, ClientConfiguration};
use esp_idf_hal::{modem::WifiModem};
use esp_idf_svc::{wifi::{EspWifi}, eventloop::EspEventLoop, nvs::EspDefaultNvsPartition};
use anyhow::Context;

pub fn connect(wifi_ssid: &str, wifi_pass: &str) -> anyhow::Result<EspWifi<'static>> {
    let sysloop = EspEventLoop::take()?;
    let modem = unsafe { WifiModem::new() };
    let nvs_defaults = EspDefaultNvsPartition::take()?;
    let mut wifi = EspWifi::new(modem, sysloop, Some(nvs_defaults))?;

    println!("Wifi created, scanning available networks...");

    let available_networks = wifi.scan()?;
    let target_network = available_networks
        .iter()
        .find(|network| network.ssid == wifi_ssid)
        .with_context(|| format!("Failed to detect the target network ({})", wifi_ssid))?;

    println!("Scan successfull, found '{}', with config: {target_network:#?}", wifi_ssid);

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: wifi_ssid.into(),
        password: wifi_pass.into(),
        auth_method: target_network.auth_method,
        bssid: Some(target_network.bssid),
        channel: Some(target_network.channel),
    }))?;

    wifi.start()?;
    wifi.connect()?;

    Ok(wifi)
}