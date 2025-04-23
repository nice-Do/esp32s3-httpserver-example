use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use esp_idf_svc::wifi::{Configuration, AuthMethod};
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use serde::{Serialize, Deserialize};
use tokio::time::sleep;

// Define a struct for sensor data that can be serialized to JSON
#[derive(Serialize, Deserialize, Debug, Clone)]
struct SensorData {
    temperature: f32,
    humidity: f32,
    timestamp: u64,
}

// Function to setup WiFi
fn setup_wifi() -> anyhow::Result<BlockingWifi<EspWifi<'static>>> {
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let mut wifi = BlockingWifi::wrap(
        EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?,
        sys_loop,
    )?;

    // Use Access Point mode (create a WiFi network)
    let mut ssid = heapless::String::<32>::new();
    ssid.push_str("ESP32-S3-DEMO").unwrap();

    let mut password = heapless::String::<64>::new();
    password.push_str("password123").unwrap();

    let ap_config = esp_idf_svc::wifi::AccessPointConfiguration {
        ssid,
        auth_method: AuthMethod::WPA2Personal,
        password,
        channel: 1,
        ..Default::default()
    };
    let wifi_configuration = Configuration::AccessPoint(ap_config);

    wifi.set_configuration(&wifi_configuration)?;
    wifi.start()?;
    log::info!("WiFi started in Access Point mode");

    wifi.wait_netif_up()?;
    let ip_info = wifi.wifi().ap_netif().get_ip_info()?;
    log::info!("AP IP: {}", ip_info.ip);

    Ok(wifi)
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
    let _wifi = setup_wifi()?;
    log::info!("WiFi setup complete");

    // Create a shared sensor data that will be updated periodically
    let sensor_data = Arc::new(Mutex::new(SensorData {
        temperature: 25.0,
        humidity: 60.0,
        timestamp: 0,
    }));

    // Create a Tokio runtime with a single-threaded scheduler
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build Tokio runtime");

    // Create an HTTP server with increased stack size
    let server_config = esp_idf_svc::http::server::Configuration {
        stack_size: 10240,
        ..Default::default()
    };

    let mut server = EspHttpServer::new(&server_config)?;

    // Clone for the API handler
    let sensor_data_clone = sensor_data.clone();

    // Serve the main HTML page
    server.fn_handler::<anyhow::Error, _>("/", esp_idf_svc::http::Method::Get, |req| {
        let html = r#"<!DOCTYPE html>
<html>
<head>
    <title>ESP32-S3 HTTP Demo</title>
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <style>
        body { font-family: Arial, sans-serif; margin: 20px; }
        #status { font-weight: bold; }
        #messages { border: 1px solid #ccc; padding: 10px; height: 200px; overflow-y: auto; margin-bottom: 10px; }
        input, button { padding: 5px; margin-right: 5px; }
    </style>
</head>
<body>
    <h1>ESP32-S3 HTTP Demo</h1>
    <div id="sensor-data">
        <h2>Sensor Data</h2>
        <p>Temperature: <span id="temperature">--</span>°C</p>
        <p>Humidity: <span id="humidity">--</span>%</p>
        <p>Last updated: <span id="timestamp">--</span></p>
    </div>
    <button id="refresh-btn">Refresh Data</button>

    <script>
        const temperatureSpan = document.getElementById('temperature');
        const humiditySpan = document.getElementById('humidity');
        const timestampSpan = document.getElementById('timestamp');
        const refreshBtn = document.getElementById('refresh-btn');

        // Function to fetch sensor data
        async function fetchSensorData() {
            try {
                const response = await fetch('/api/sensor');
                if (!response.ok) {
                    throw new Error(`HTTP error! Status: ${response.status}`);
                }

                const data = await response.json();

                // Update the UI
                temperatureSpan.textContent = data.temperature.toFixed(1);
                humiditySpan.textContent = data.humidity.toFixed(1);

                // Format timestamp
                const date = new Date(data.timestamp * 1000);
                timestampSpan.textContent = date.toLocaleTimeString();

                console.log('Data updated:', data);
            } catch (error) {
                console.error('Error fetching sensor data:', error);
            }
        }

        // Fetch data on page load
        fetchSensorData();

        // Set up refresh button
        refreshBtn.addEventListener('click', fetchSensorData);

        // Auto-refresh every 5 seconds
        setInterval(fetchSensorData, 5000);
    </script>
</body>
</html>
"#;

        // Create response with HTML content and write it
        let mut resp = req.into_ok_response()?;
        resp.write(html.as_bytes())?;

        Ok(())
    })?;

    // API endpoint to get sensor data
    server.fn_handler::<anyhow::Error, _>("/api/sensor", esp_idf_svc::http::Method::Get, move |req| {
        // Get the current sensor data
        let sensor_data = sensor_data_clone.lock().unwrap().clone();

        // Convert to JSON
        let json_data = serde_json::to_string(&sensor_data).unwrap_or_default();

        // Create response with JSON content and write it
        let mut resp = req.into_ok_response()?;
        resp.write(json_data.as_bytes())?;

        Ok(())
    })?;

    // Run the Tokio runtime with a task to update sensor data periodically
    runtime.block_on(async {
        log::info!("HTTP Server started. Access the demo at http://<ESP32-IP>/");

        // Periodically update sensor data
        loop {
            // Update sensor data with simulated values
            let mut data = sensor_data.lock().unwrap();
            data.temperature = 20.0 + (rand::random::<f32>() * 10.0); // Random temperature between 20-30°C
            data.humidity = 50.0 + (rand::random::<f32>() * 20.0);   // Random humidity between 50-70%
            data.timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or(Duration::from_secs(0))
                .as_secs();

            // Log the updated values
            log::info!("Updated sensor data: {:.1}°C, {:.1}%, {}",
                      data.temperature, data.humidity, data.timestamp);

            // Wait before next update
            drop(data); // Release the lock before sleeping
            sleep(Duration::from_secs(5)).await;
        }
    });

    Ok(())
}
