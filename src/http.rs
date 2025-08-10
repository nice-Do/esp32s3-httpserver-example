use anyhow::Result;
use esp_idf_svc::http::server::EspHttpServer;
use esp_idf_svc::http::Method;

use crate::sensor::{snapshot, SharedSensor};

/// Static index page embedded at compile-time from the assets directory.
/// Adjust the path if you move the file.
const INDEX_HTML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/index.html"));

/// Build a default HTTP server configuration.
/// Tweak values here if you need different stack size or timeouts.
pub fn default_config() -> esp_idf_svc::http::server::Configuration {
    esp_idf_svc::http::server::Configuration {
        // Increase stack size for handlers if you extend them
        stack_size: 10_240,
        ..Default::default()
    }
}

/// Create and configure an HTTP server with routes and handlers.
/// The returned server must be kept alive (held in a variable) for the routes to remain active.
pub fn start_http_server(sensor: SharedSensor) -> Result<EspHttpServer<'static>> {
    let config = default_config();
    start_http_server_with_config(sensor, &config)
}

/// Same as `start_http_server` but allows passing a custom configuration.
pub fn start_http_server_with_config(
    sensor: SharedSensor,
    config: &esp_idf_svc::http::server::Configuration,
) -> Result<EspHttpServer<'static>> {
    let mut server = EspHttpServer::new(config)?;

    // Serve the static index page at "/" and "/index.html"
    server.fn_handler::<anyhow::Error, _>("/", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write(INDEX_HTML.as_bytes())?;
        Ok(())
    })?;

    server.fn_handler::<anyhow::Error, _>("/index.html", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write(INDEX_HTML.as_bytes())?;
        Ok(())
    })?;

    // Basic health endpoint (optional)
    server.fn_handler::<anyhow::Error, _>("/health", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        resp.write(b"OK")?;
        Ok(())
    })?;

    // API endpoint for sensor data
    server.fn_handler::<anyhow::Error, _>("/api/sensor", Method::Get, move |req| {
        let data = snapshot(&sensor);
        let json = serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string());

        let mut resp = req.into_ok_response()?;
        // If you want to be explicit about the content type (optional)
        // let _ = resp.set_content_type("application/json");
        resp.write(json.as_bytes())?;
        Ok(())
    })?;

    // Optional: handle favicon to avoid 404 noise in logs
    server.fn_handler::<anyhow::Error, _>("/favicon.ico", Method::Get, |req| {
        let mut resp = req.into_ok_response()?;
        // Transparent 1x1 GIF (43 bytes) if you want a tiny inline icon:
        // const FAVICON_GIF: &[u8] = b"GIF89a\x01\x00\x01\x00\xf0\x01\x00\xff\xff\xff\x00\x00\x00!\xf9\x04\x01\n\x00\x01\x00,\x00\x00\x00\x00\x01\x00\x01\x00\x00\x02\x02D\x01\x00;";
        // let _ = resp.set_content_type("image/gif");
        // resp.write(FAVICON_GIF)?;
        // Or just return empty
        resp.write(b"")?;
        Ok(())
    })?;

    Ok(server)
}
