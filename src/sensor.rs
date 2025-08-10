use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Shared sensor data structure that can be serialized to JSON and cloned for responses.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SensorData {
    pub temperature: f32,
    pub humidity: f32,
    pub timestamp: u64,
}

/// Thread-safe shared handle to the sensor data.
pub type SharedSensor = Arc<Mutex<SensorData>>;

/// Create a new shared sensor object with default initial values.
pub fn new_shared() -> SharedSensor {
    Arc::new(Mutex::new(SensorData {
        temperature: 25.0,
        humidity: 60.0,
        timestamp: now_secs(),
    }))
}

/// Take a snapshot (clone) of the current sensor data.
pub fn snapshot(shared: &SharedSensor) -> SensorData {
    shared.lock().map(|d| d.clone()).unwrap_or_else(|e| {
        log::error!("Failed to lock sensor data: {}", e);
        // Return a safe default if poisoned
        SensorData {
            temperature: 25.0,
            humidity: 60.0,
            timestamp: now_secs(),
        }
    })
}

/// Update the shared sensor data once with simulated values.
#[allow(dead_code)]
pub fn update_once(shared: &SharedSensor) {
    if let Ok(mut data) = shared.lock() {
        simulate_update(&mut data);
    } else {
        log::warn!("Failed to lock sensor data for update");
    }
}

/// Start a background thread that periodically updates the sensor data.
/// Returns the JoinHandle so the caller can keep it if needed.
/// The thread runs forever; drop the handle to detach.
pub fn start_updater(shared: SharedSensor, period: Duration) -> thread::JoinHandle<()> {
    thread::spawn(move || loop {
        // Update values
        {
            if let Ok(mut data) = shared.lock() {
                simulate_update(&mut data);
                log::info!(
                    "Updated sensor data: {:.1}°C, {:.1}%, {}",
                    data.temperature,
                    data.humidity,
                    data.timestamp
                );
            } else {
                log::warn!("Failed to lock sensor data for periodic update");
            }
        }

        // Let the scheduler run other tasks (prevents starving IDLE task)
        thread::sleep(period);
    })
}

/// Internal helper to simulate sensor data change and timestamp refresh.
fn simulate_update(data: &mut SensorData) {
    // Random temperature between 20-30°C
    data.temperature = 20.0 + (rand::random::<f32>() * 10.0);
    // Random humidity between 50-70%
    data.humidity = 50.0 + (rand::random::<f32>() * 20.0);
    data.timestamp = now_secs();
}

/// Current UNIX timestamp (seconds).
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_secs(0))
        .as_secs()
}
