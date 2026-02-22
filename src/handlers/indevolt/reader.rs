use log::{debug, error};
use reqwest::Client;
use std::collections::HashMap;

use crate::models::indevolt_models::BatterySnapshot;

// --------------------------------------------------------------------------------------------------------------
// Numeric sensor IDs for the Indevolt RPC bulk-read API.
//
// API:  GET /rpc/Indevolt.GetData?config={"t":[id,...]}
// Resp: flat JSON object  {"<id>": <numeric_value>, ...}
//
// Official Indevolt firmware sensor ID mapping:
//   7101  Working mode              1=Self-consumed, 5=Schedule
//   1664  DC Input Power 1 (PV1)   W
//   1665  DC Input Power 2 (PV2)   W
//   1501  Total DC Output Power     W
//   2108  Total AC Output Power     W
//   1502  Daily Production          kWh
//   1505  Cumulative Production     raw ×0.001 → kWh
//   2101  Total AC Input Power      W
//   2107  Total AC Input Energy     kWh
//   6000  Battery Power             W
//   6001  Battery State             1000=Static, 1001=Charging, 1002=Discharging
//   6002  Total Battery SOC         %
//   6004  Battery Daily Charging    kWh
//   6005  Battery Daily Discharging kWh
//   6006  Battery Total Charging    kWh
//   6007  Battery Total Discharging kWh
//   11016 Meter Power (grid)        W  positive=import, negative=export
// --------------------------------------------------------------------------------------------------------------

const ID_WORKING_MODE:              u32 = 7101;  // 1=Self-consumed, 5=Schedule
const ID_DC_INPUT1:                 u32 = 1664;  // W  PV string 1
const ID_DC_INPUT2:                 u32 = 1665;  // W  PV string 2
const ID_TOTAL_DC_OUTPUT:           u32 = 1501;  // W
const ID_TOTAL_AC_OUTPUT:           u32 = 2108;  // W
const ID_DAILY_PRODUCTION:          u32 = 1502;  // kWh
const ID_CUMULATIVE_PRODUCTION:     u32 = 1505;  // raw ×0.001 = kWh
const ID_TOTAL_AC_INPUT:            u32 = 2101;  // W
const ID_TOTAL_AC_INPUT_ENERGY:     u32 = 2107;  // kWh
const ID_BATTERY_POWER:             u32 = 6000;  // W
const ID_BATTERY_STATE:             u32 = 6001;  // 1000=Static, 1001=Charging, 1002=Discharging
const ID_BATTERY_SOC:               u32 = 6002;  // %
const ID_DAILY_CHARGING:            u32 = 6004;  // kWh
const ID_DAILY_DISCHARGING:         u32 = 6005;  // kWh
const ID_TOTAL_CHARGING:            u32 = 6006;  // kWh
const ID_TOTAL_DISCHARGING:         u32 = 6007;  // kWh
const ID_METER_POWER:               u32 = 11016; // W  grid (positive=import)

/// All IDs requested in one shot (order mirrors the firmware table).
const SNAPSHOT_IDS: &[u32] = &[
    ID_WORKING_MODE, ID_DC_INPUT1, ID_DC_INPUT2,
    ID_TOTAL_DC_OUTPUT, ID_TOTAL_AC_OUTPUT, ID_DAILY_PRODUCTION,
    ID_CUMULATIVE_PRODUCTION, ID_TOTAL_AC_INPUT, ID_TOTAL_AC_INPUT_ENERGY,
    ID_BATTERY_POWER, ID_BATTERY_STATE, ID_BATTERY_SOC,
    ID_DAILY_CHARGING, ID_DAILY_DISCHARGING,
    ID_TOTAL_CHARGING, ID_TOTAL_DISCHARGING, ID_METER_POWER,
];

// --------------------------------------------------------------------------------------------------------------

/// Fetch all snapshot values in a single GET /rpc/Indevolt.GetData call.
pub async fn read_battery_snapshot(base_url: &str, device_model: &str) -> BatterySnapshot {
    let client = Client::new();

    // Build the config query parameter: {"t":[id,...]}
    let ids_json = format!(
        "{{\"t\":[{}]}}",
        SNAPSHOT_IDS.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",")
    );

    let url = format!("{}/rpc/Indevolt.GetData", base_url);

    let result = client
        .get(&url)
        .query(&[("config", &ids_json)])
        .send()
        .await;

    let data: HashMap<String, serde_json::Value> = match result {
        Ok(resp) if resp.status().is_success() => {
            match resp.json().await {
                Ok(map) => map,
                Err(e) => {
                    error!("[Indevolt] Failed to parse GetData response: {}", e);
                    HashMap::new()
                }
            }
        }
        Ok(resp) => {
            error!("[Indevolt] GetData returned HTTP {}", resp.status());
            HashMap::new()
        }
        Err(e) => {
            error!("[Indevolt] GetData request failed: {}", e);
            HashMap::new()
        }
    };

    debug!("[Indevolt] GetData raw: {:?}", data);

    // Helpers to extract typed values by numeric ID.
    let f64_id = |id: u32| -> f64 {
        data.get(&id.to_string())
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0)
    };
    let i32_id = |id: u32| -> i32 {
        data.get(&id.to_string())
            .and_then(|v| v.as_f64())
            .map(|f| f as i32)
            .unwrap_or(0)
    };

    // Decode battery state integer to human-readable string.
    let battery_state = match i32_id(ID_BATTERY_STATE) {
        1000 => "Static".to_string(),
        1001 => "Charging".to_string(),
        1002 => "Discharging".to_string(),
        code => format!("Unknown({})", code),
    };

    // Decode working mode integer to human-readable string.
    let working_mode = match i32_id(ID_WORKING_MODE) {
        1 => "Self-consumed Prioritized".to_string(),
        4 => "Real-time Control".to_string(),
        5 => "Schedule".to_string(),
        code => format!("Mode({})", code),
    };

    BatterySnapshot {
        device_model:              device_model.to_string(),
        battery_soc:               f64_id(ID_BATTERY_SOC),
        battery_state,
        working_mode,
        battery_power_w:           i32_id(ID_BATTERY_POWER),
        dc_input_power1_w:         i32_id(ID_DC_INPUT1),
        dc_input_power2_w:         i32_id(ID_DC_INPUT2),
        total_dc_output_power_w:   i32_id(ID_TOTAL_DC_OUTPUT),
        total_ac_output_power_w:   i32_id(ID_TOTAL_AC_OUTPUT),
        total_ac_input_power_w:    i32_id(ID_TOTAL_AC_INPUT),
        meter_power_w:             i32_id(ID_METER_POWER),
        daily_production_kwh:      f64_id(ID_DAILY_PRODUCTION),
        cumulative_production_kwh: f64_id(ID_CUMULATIVE_PRODUCTION) * 0.001, // raw ×0.001 = kWh
        daily_charging_kwh:        f64_id(ID_DAILY_CHARGING),
        daily_discharging_kwh:     f64_id(ID_DAILY_DISCHARGING),
        total_charging_kwh:        f64_id(ID_TOTAL_CHARGING),
        total_discharging_kwh:     f64_id(ID_TOTAL_DISCHARGING),
        total_ac_input_energy_kwh: f64_id(ID_TOTAL_AC_INPUT_ENERGY),
    }
}
