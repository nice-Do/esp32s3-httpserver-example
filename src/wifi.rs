use anyhow::Result;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::prelude::Peripherals;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{
    AccessPointConfiguration, AuthMethod, BlockingWifi, Configuration, EspWifi,
};
use heapless::String as HString;

/// Wi-Fi helpers for Access Point (AP) setup and logging.
///
/// Typical usage:
/// let _wifi = crate::wifi::setup_default_ap()?;
/// or
/// let _wifi = crate::wifi::setup_ap_with("MY_AP", Some("secret123"), 6)?;
///
/// Note: ensure the ESP logger is initialized in main:
/// esp_idf_svc::log::EspLogger::initialize_default();

/// Setup an Access Point with a default SSID/password/channel.
/// SSID: "ESP32-S3-DEMO", password: "password123", channel: 1
pub fn setup_default_ap() -> Result<BlockingWifi<EspWifi<'static>>> {
    setup_ap_with("ESP32-S3-DEMO", Some("password123"), 1)
}

/// Setup an Access Point with provided parameters.
///
/// - ssid: up to 32 chars
/// - password: None for open network, Some for WPA2-Personal (8..=63 chars)
/// - channel: Wi-Fi channel (1..=13 typical)
pub fn setup_ap_with(
    ssid: &str,
    password: Option<&str>,
    channel: u8,
) -> Result<BlockingWifi<EspWifi<'static>>> {
    // Take required ESP-IDF services
    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    let modem = peripherals.modem;

    // Create Wi-Fi driver and wrap with blocking helper
    let wifi = EspWifi::new(modem, sys_loop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(wifi, sys_loop)?;

    // Build AP configuration
    let cfg = build_ap_config(ssid, password, channel)?;

    // Apply config and start AP
    wifi.set_configuration(&cfg)?;
    wifi.start()?;
    log::info!("WiFi started in Access Point mode");
    // Improve AP stability: disable WiFi power save and bump TX power
    unsafe {
        // Disable power save (prevents skipped beacons that make AP appear to “disappear”)
        esp_idf_svc::sys::esp_wifi_set_ps(esp_idf_svc::sys::wifi_ps_type_t_WIFI_PS_NONE);
        // Max TX power: units of 0.25 dBm; 84 ≈ 21 dBm
        let _ = esp_idf_svc::sys::esp_wifi_set_max_tx_power(84);
    }

    // Wait for netif up and log IP info
    wifi.wait_netif_up()?;
    if let Ok(ip_info) = wifi.wifi().ap_netif().get_ip_info() {
        log::info!("AP IP: {}", ip_info.ip);
    } else {
        log::warn!("Failed to fetch AP IP info");
    }

    Ok(wifi)
}

/// Internal helper to create AP configuration with heapless Strings.
fn build_ap_config(ssid: &str, password: Option<&str>, channel: u8) -> Result<Configuration> {
    // SSID (<=32)
    let mut ssid_h: HString<32> = HString::new();
    if ssid.len() > ssid_h.capacity() {
        anyhow::bail!("SSID too long (max {} chars)", ssid_h.capacity());
    }
    ssid_h.push_str(ssid).unwrap();

    // Password (<=64) and auth method
    let (password_h, auth_method) = match password {
        Some(pwd) if !pwd.is_empty() => {
            if pwd.len() < 8 || pwd.len() > 63 {
                anyhow::bail!("WPA2 password must be 8..=63 characters");
            }
            let mut pwd_h: HString<64> = HString::new();
            if pwd.len() > pwd_h.capacity() {
                anyhow::bail!("Password too long (max {} chars)", pwd_h.capacity());
            }
            pwd_h.push_str(pwd).unwrap();
            (pwd_h, AuthMethod::WPA2Personal)
        }
        _ => {
            // Open network (no password)
            (HString::<64>::new(), AuthMethod::None)
        }
    };

    let ap_cfg = AccessPointConfiguration {
        ssid: ssid_h,
        auth_method,
        password: password_h,
        channel,
        // Make AP configuration explicit for stability
        ssid_hidden: false,
        max_connections: 4,
        ..Default::default()
    };

    Ok(Configuration::AccessPoint(ap_cfg))
}
