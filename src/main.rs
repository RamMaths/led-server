use std::{
    thread::sleep,
    time::Duration,
    sync::{Arc, Mutex}
};
use embedded_svc::{http::Method, io::Write};
use anyhow::Result;
use esp_idf_svc::hal::{
    gpio::PinDriver,
    peripherals::Peripherals
};
use esp_idf_svc::{
    wifi::EspWifi,
    nvs::EspDefaultNvsPartition,
    eventloop::EspSystemEventLoop,
    http::server::{Configuration, EspHttpServer}
};
use embedded_svc::wifi::{ClientConfiguration, Configuration as wifiConfiguration};

//Add your wifi credentials in the cfg.toml file
#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    wifi_ssid: &'static str,
    #[default("")]
    wifi_pass: &'static str
}

fn main() -> Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    let peripherals = Peripherals::take().unwrap();
    let sys_loop = EspSystemEventLoop::take().unwrap();
    let nvs = EspDefaultNvsPartition::take().unwrap();

    let app_config: Config = CONFIG;

    //Wifi configuration
    let mut wifi_driver = EspWifi::new(
        peripherals.modem,
        sys_loop,
        Some(nvs)
    ).unwrap();

    wifi_driver.set_configuration(&wifiConfiguration::Client (
        ClientConfiguration {
            ssid: app_config.wifi_ssid.try_into().unwrap(),
            password: app_config.wifi_pass.try_into().unwrap(),
            ..Default::default()
        }
    )).expect("Failed to set the client");

    wifi_driver.start().unwrap();
    wifi_driver.connect().unwrap();

    while !wifi_driver.is_connected().unwrap() {
        let config = wifi_driver.get_configuration().unwrap();
        log::info!("Waiting for station: {:?}", config);
    }

    log::info!("Should be connected now with credentials: ");

    //Setting up the led pin
    let pin = PinDriver::output(peripherals.pins.gpio48);
    let pin = Arc::new(Mutex::new(pin));

    let pin_ref = pin.clone();

    //Setting the http server

    let mut server = EspHttpServer::new(&Configuration::default())?;

    server.fn_handler("/on", Method::Get, move |request| {
        pin_ref
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .set_high()
            .unwrap();

        request.into_ok_response()?
            .write_all("led on".as_bytes())
            .map(|_|())
    })?;

    let pin_ref = pin.clone(); 

    server.fn_handler("/off", Method::Get, move |request| {
        pin_ref
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .set_low()
            .unwrap();

        request.into_ok_response()?
            .write_all("led off".as_bytes())
            .map(|_|())
    })?;

    let mut print_once: bool = false;
    let pin_ref = pin.clone();

    loop {

        if !print_once {
            println!("IP info: {:?}", wifi_driver.sta_netif().get_ip_info().unwrap());
            print_once = true;
        }

        if pin_ref
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .is_set_high() {

            println!("Led on");
        } else {
            println!("Led off");
        }
        
        sleep(Duration::from_millis(1000));
    }
}
