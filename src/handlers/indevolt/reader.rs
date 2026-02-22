use log::{debug, error, warn};
use reqwest::Client;

use crate::models::indevolt_models::{BatteryConfig, BatterySnapshot, SensorReading};

// --------------------------------------------------------------------------------------------------------------
// Sensor key constants - adjust these to match your exact Indevolt firmware key names.

const KEY_SOC:                    &str = "BatterySOC";
const KEY_BATTERY_STATE:          &str = "BatteryState";
const KEY_WORKING_MODE:           &str = "WorkingMode";
const KEY_BATTERY_POWER:          &str = "BatteryPower";
const KEY_DC_INPUT1:              &str = "DCInputPower1";
const KEY_DC_INPUT2:              &str = "DCInputPower2";
const KEY_TOTAL_DC_OUTPUT:        &str = "TotalDCOutputPower";
const KEY_TOTAL_AC_OUTPUT:        &str = "TotalACOutputPower";
const KEY_TOTAL_AC_INPUT:         &str = "TotalACInputPower";
const KEY_METER_POWER:            &str = "MeterPower";
const KEY_DAILY_PRODUCTION:       &str = "DailyProduction";
const KEY_CUMULATIVE_PRODUCTION:  &str = "CumulativeProduction";
const KEY_DAILY_CHARGING:         &str = "DailyCharging";
const KEY_DAILY_DISCHARGING:      &str = "DailyDischarging";
const KEY_TOTAL_CHARGING:         &str = "TotalCharging";
const KEY_TOTAL_DISCHARGING:      &str = "TotalDischarging";
const KEY_TOTAL_AC_INPUT_ENERGY:  &str = "TotalACInputEnergy";

const KEY_RATED_CAPACITY:         &str = "RatedCapacity";
const KEY_MIN_SOC:                &str = "MinSOC";
const KEY_MAX_SOC:                &str = "MaxSOC";
const KEY_MAX_CHARGE_POWER:       &str = "MaxChargePower";
const KEY_MAX_DISCHARGE_POWER:    &str = "MaxDischargePower";

// All snapshot keys batched together for efficient polling.
const SNAPSHOT_KEYS: &[&str] = &[
    KEY_SOC, KEY_BATTERY_STATE, KEY_WORKING_MODE,
    KEY_BATTERY_POWER, KEY_DC_INPUT1, KEY_DC_INPUT2,
    KEY_TOTAL_DC_OUTPUT, KEY_TOTAL_AC_OUTPUT, KEY_TOTAL_AC_INPUT,
    KEY_METER_POWER, KEY_DAILY_PRODUCTION, KEY_CUMULATIVE_PRODUCTION,
    KEY_DAILY_CHARGING, KEY_DAILY_DISCHARGING,
    KEY_TOTAL_CHARGING, KEY_TOTAL_DISCHARGING, KEY_TOTAL_AC_INPUT_ENERGY,
];

const CONFIG_KEYS: &[&str] = &[
    KEY_RATED_CAPACITY, KEY_MIN_SOC, KEY_MAX_SOC,
    KEY_MAX_CHARGE_POWER, KEY_MAX_DISCHARGE_POWER,
];

// --------------------------------------------------------------------------------------------------------------

/// Fetch a single sensor reading from GET /device/sensor?key=<KEY>
async fn fetch_sensor(client: &Client, base_url: &str, key: &str) -> Option<SensorReading> {
    let url = format!("{}/device/sensor?key={}", base_url, key);
    let response = client.get(&url).send().await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<SensorReading>().await {
                Ok(reading) => {
                    debug!("[Indevolt] {} = {} {:?}", reading.key, reading.value, reading.unit);
                    Some(reading)
                }
                Err(e) => {
                    warn!("[Indevolt] Failed to parse sensor '{}': {}", key, e);
                    None
                }
            }
        }
        Ok(resp) => {
            warn!("[Indevolt] Sensor '{}' returned HTTP {}", key, resp.status());
            None
        }
        Err(e) => {
            error!("[Indevolt] HTTP error fetching sensor '{}': {}", key, e);
            None
        }
    }
}

// --------------------------------------------------------------------------------------------------------------

/// Fetch all snapshot sensor keys concurrently and assemble a BatterySnapshot.
/// Individual key failures result in the field keeping its Default value (0 / empty string).
pub async fn read_battery_snapshot(base_url: &str, device_model: &str) -> BatterySnapshot {
    let client = Client::new();

    // Fire all requests concurrently.
    let futures: Vec<_> = SNAPSHOT_KEYS
        .iter()
        .map(|key| fetch_sensor(&client, base_url, key))
        .collect();

    let results = futures::future::join_all(futures).await;

    // Build a lookup map from the results.
    let readings: std::collections::HashMap<String, String> = results
        .into_iter()
        .flatten()
        .map(|r| (r.key, r.value))
        .collect();

    let parse_f64 = |key: &str| -> f64 {
        readings.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.0)
    };
    let parse_i32 = |key: &str| -> i32 {
        readings.get(key)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    };
    let parse_str = |key: &str| -> String {
        readings.get(key).cloned().unwrap_or_default()
    };

    BatterySnapshot {
        device_model:              device_model.to_string(),
        battery_soc:               parse_f64(KEY_SOC),
        battery_state:             parse_str(KEY_BATTERY_STATE),
        working_mode:              parse_str(KEY_WORKING_MODE),
        battery_power_w:           parse_i32(KEY_BATTERY_POWER),
        dc_input_power1_w:         parse_i32(KEY_DC_INPUT1),
        dc_input_power2_w:         parse_i32(KEY_DC_INPUT2),
        total_dc_output_power_w:   parse_i32(KEY_TOTAL_DC_OUTPUT),
        total_ac_output_power_w:   parse_i32(KEY_TOTAL_AC_OUTPUT),
        total_ac_input_power_w:    parse_i32(KEY_TOTAL_AC_INPUT),
        meter_power_w:             parse_i32(KEY_METER_POWER),
        daily_production_kwh:      parse_f64(KEY_DAILY_PRODUCTION),
        cumulative_production_kwh: parse_f64(KEY_CUMULATIVE_PRODUCTION),
        daily_charging_kwh:        parse_f64(KEY_DAILY_CHARGING),
        daily_discharging_kwh:     parse_f64(KEY_DAILY_DISCHARGING),
        total_charging_kwh:        parse_f64(KEY_TOTAL_CHARGING),
        total_discharging_kwh:     parse_f64(KEY_TOTAL_DISCHARGING),
        total_ac_input_energy_kwh: parse_f64(KEY_TOTAL_AC_INPUT_ENERGY),
    }
}

// --------------------------------------------------------------------------------------------------------------

/// Fetch the static battery configuration from the device.
pub async fn read_battery_config(base_url: &str, device_model: &str) -> BatteryConfig {
    let client = Client::new();

    let futures: Vec<_> = CONFIG_KEYS
        .iter()
        .map(|key| fetch_sensor(&client, base_url, key))
        .collect();

    let results = futures::future::join_all(futures).await;

    let readings: std::collections::HashMap<String, String> = results
        .into_iter()
        .flatten()
        .map(|r| (r.key, r.value))
        .collect();

    let parse_f64 = |key: &str| -> f64 {
        readings.get(key).and_then(|v| v.parse().ok()).unwrap_or(0.0)
    };
    let parse_i32 = |key: &str| -> i32 {
        readings.get(key).and_then(|v| v.parse().ok()).unwrap_or(0)
    };

    BatteryConfig {
        device_model:          device_model.to_string(),
        rated_capacity_kwh:    parse_f64(KEY_RATED_CAPACITY),
        min_soc_percent:       parse_f64(KEY_MIN_SOC),
        max_soc_percent:       parse_f64(KEY_MAX_SOC),
        max_charge_power_w:    parse_i32(KEY_MAX_CHARGE_POWER),
        max_discharge_power_w: parse_i32(KEY_MAX_DISCHARGE_POWER),
    }
}
