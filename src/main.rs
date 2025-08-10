use std::thread;
use std::time::Duration;
mod http;
mod sensor;
mod wifi;

fn wait_forever<T, U, V>(_a: &T, _b: &U, _c: &V) -> ! {
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

// Main function
fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Starting HTTP Server example...");

    // Setup WiFi first
    let wifi = crate::wifi::setup_default_ap()?;
    log::info!("WiFi setup complete");

    // Create a shared sensor data that will be updated periodically
    let sensor_data = sensor::new_shared();

    // Using std::thread for periodic work to avoid starving FreeRTOS idle task

    // Start HTTP server with predefined routes
    let server = crate::http::start_http_server(sensor_data.clone())?;

    // Start a periodic updater on a separate std thread to avoid starving FreeRTOS idle task
    let updater = sensor::start_updater(sensor_data.clone(), Duration::from_secs(5));

    log::info!("HTTP Server started. Access the demo at http://<ESP32-IP>/");

    wait_forever(&wifi, &server, &updater)
}
