use log::{error, info};
use reqwest::Client;

use crate::models::indevolt_models::{ControlCommand, WorkingMode};

// --------------------------------------------------------------------------------------------------------------

/// Send a control command to POST /device/control on the Indevolt.
/// Returns Ok(()) on HTTP 2xx, Err with a description otherwise.
async fn send_command(client: &Client, base_url: &str, cmd: &ControlCommand) -> Result<(), String> {
    let url = format!("{}/device/control", base_url);

    let response = client
        .post(&url)
        .json(cmd)
        .send()
        .await
        .map_err(|e| format!("[Indevolt] HTTP error sending command {:?}: {}", cmd, e))?;

    if response.status().is_success() {
        info!("[Indevolt] Command accepted: key='{}' value='{}'", cmd.key, cmd.value);
        Ok(())
    } else {
        let status = response.status();
        let body   = response.text().await.unwrap_or_default();
        Err(format!("[Indevolt] Command rejected (HTTP {}): {}", status, body))
    }
}

// --------------------------------------------------------------------------------------------------------------

/// Switch the Indevolt working mode.
pub async fn set_working_mode(base_url: &str, mode: WorkingMode) -> Result<(), String> {
    let client = Client::new();
    let cmd    = ControlCommand::set_working_mode(&mode);
    info!("[Indevolt] Setting working mode → '{}'", mode.as_api_str());
    send_command(&client, base_url, &cmd).await
}

/// Override the maximum charge power (watts).
/// Useful for throttling during capacity-tariff peak hours.
pub async fn set_charge_power(base_url: &str, watts: i32) -> Result<(), String> {
    let client = Client::new();
    let cmd    = ControlCommand::set_charge_power(watts);
    info!("[Indevolt] Setting max charge power → {} W", watts);
    send_command(&client, base_url, &cmd).await
}

/// Override the maximum discharge power (watts).
pub async fn set_discharge_power(base_url: &str, watts: i32) -> Result<(), String> {
    let client = Client::new();
    let cmd    = ControlCommand::set_discharge_power(watts);
    info!("[Indevolt] Setting max discharge power → {} W", watts);
    send_command(&client, base_url, &cmd).await
}

/// Convenience: restore the Indevolt to its default autonomous mode.
pub async fn restore_auto_mode(base_url: &str) -> Result<(), String> {
    set_working_mode(base_url, WorkingMode::SelfConsumedPrioritized).await
}
