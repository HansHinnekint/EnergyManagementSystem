use log::info;
use reqwest::Client;

use crate::models::indevolt_models::{SetDataConfig, WorkingMode};

// --------------------------------------------------------------------------------------------------------------
// Register addresses
const REG_WORKING_MODE: u32 = 47005; // set working mode (1=Self-consumed, 4=Realtime, 5=Schedule)
const REG_CONTROL:      u32 = 47015; // real-time charge/discharge commands
const FUNC_WRITE:       u32 = 16;    // Modbus function 16 (write multiple registers)

// v[0] action codes for REG_CONTROL
const ACTION_STOP:      i64 = 0;
const ACTION_CHARGE:    i64 = 1;
const ACTION_DISCHARGE: i64 = 2;

// --------------------------------------------------------------------------------------------------------------

/// Send a SetData command via GET /rpc/Indevolt.SetData?config=<json>.
async fn send_command(client: &Client, base_url: &str, cfg: &SetDataConfig) -> Result<(), String> {
    let url        = format!("{}/rpc/Indevolt.SetData", base_url);
    let config_str = serde_json::to_string(cfg)
        .map_err(|e| format!("[Indevolt] Failed to serialise SetData config: {}", e))?;

    let response = client
        .get(&url)
        .query(&[("config", &config_str)])
        .send()
        .await
        .map_err(|e| format!("[Indevolt] HTTP error sending SetData {:?}: {}", cfg, e))?;

    if response.status().is_success() {
        info!("[Indevolt] SetData accepted: t={} v={:?}", cfg.t, cfg.v);
        Ok(())
    } else {
        let status = response.status();
        let body   = response.text().await.unwrap_or_default();
        Err(format!("[Indevolt] SetData rejected (HTTP {}): {}", status, body))
    }
}

// --------------------------------------------------------------------------------------------------------------

/// Set the working mode (register 47005).
/// Call with `RealtimeControl` before issuing charge/discharge commands.
/// Call with `SelfConsumedPrioritized` to hand back control to the device.
pub async fn set_working_mode(base_url: &str, mode: WorkingMode) -> Result<(), String> {
    let client = Client::new();
    let value  = mode.register_value();
    let cfg    = SetDataConfig { f: FUNC_WRITE, t: REG_WORKING_MODE, v: vec![value] };
    info!("[Indevolt] Set working mode → {} (reg={} v={})", mode.as_str(), REG_WORKING_MODE, value);
    send_command(&client, base_url, &cfg).await
}

/// Enable real-time control mode — convenience wrapper for
/// `set_working_mode(RealtimeControl)`. Must be called before charge/discharge.
pub async fn enable_realtime_mode(base_url: &str) -> Result<(), String> {
    set_working_mode(base_url, WorkingMode::RealtimeControl).await
}

/// Charge the battery at the given power up to max_soc_percent.
pub async fn charge(base_url: &str, watts: i32, max_soc_percent: u8) -> Result<(), String> {
    let client = Client::new();
    let cfg = SetDataConfig {
        f: FUNC_WRITE,
        t: REG_CONTROL,
        v: vec![ACTION_CHARGE, watts as i64, max_soc_percent as i64],
    };
    info!("[Indevolt] Charge {} W up to {}% SOC", watts, max_soc_percent);
    send_command(&client, base_url, &cfg).await
}

/// Discharge the battery at the given power down to min_soc_percent.
pub async fn discharge(base_url: &str, watts: i32, min_soc_percent: u8) -> Result<(), String> {
    let client = Client::new();
    let cfg = SetDataConfig {
        f: FUNC_WRITE,
        t: REG_CONTROL,
        v: vec![ACTION_DISCHARGE, watts as i64, min_soc_percent as i64],
    };
    info!("[Indevolt] Discharge {} W down to {}% SOC", watts, min_soc_percent);
    send_command(&client, base_url, &cfg).await
}

/// Stop real-time control (standby). The working mode stays at RealtimeControl;
/// call `set_working_mode(SelfConsumedPrioritized)` to fully hand back control.
pub async fn stop(base_url: &str) -> Result<(), String> {
    let client = Client::new();
    let cfg = SetDataConfig { f: FUNC_WRITE, t: REG_CONTROL, v: vec![ACTION_STOP, 0, 0] };
    info!("[Indevolt] Stop (standby)");
    send_command(&client, base_url, &cfg).await
}

/// Restore autonomous self-consumption mode and stop any active command.
pub async fn restore_auto_mode(base_url: &str) -> Result<(), String> {
    set_working_mode(base_url, WorkingMode::SelfConsumedPrioritized).await
}

// Legacy stubs retained so existing call-sites (optimiser placeholder) still compile.
#[allow(dead_code)]
pub async fn set_charge_power(base_url: &str, watts: i32) -> Result<(), String> {
    charge(base_url, watts, 100).await
}
#[allow(dead_code)]
pub async fn set_discharge_power(base_url: &str, watts: i32) -> Result<(), String> {
    discharge(base_url, watts, 10).await
}
