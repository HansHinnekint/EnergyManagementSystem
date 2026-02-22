use serde::{Deserialize, Serialize};
use reqwest::Error;

// --------------------------------------------------------------------------------------------------------------
// Indevolt PowerFlex2000 local API models
//
// The Indevolt exposes a key-value sensor API. Your n8n flow polls individual keys
// via GET /device/sensor?key=<KEY>. Each response looks like:
//   { "key": "BatterySOC", "value": "85.5", "unit": "%" }
//
// For control the API accepts:
//   POST /device/control   body: { "key": "WorkingMode", "value": "ChargingFromGrid" }
//
// These models cover everything your BatteryData and BatteryConfig tables store
// plus the control surface needed by the optimiser.
// --------------------------------------------------------------------------------------------------------------

/// Single sensor reading returned by GET /device/sensor?key=<KEY>
#[derive(Deserialize, Debug, Clone)]
pub struct SensorReading {
    pub key:   String,
    pub value: String,
    pub unit:  Option<String>,
}

// --------------------------------------------------------------------------------------------------------------

/// A snapshot of all battery sensors polled in one cycle.
/// Field names mirror the BatteryData table columns exactly so mapping is trivial.
#[derive(Debug, Clone, Default)]
pub struct BatterySnapshot {
    pub device_model:              String,
    pub battery_soc:               f64,   // %
    pub battery_state:             String, // "Charging" | "Discharging" | "Static"
    pub working_mode:              String, // e.g. "Self-consumed Prioritized"
    pub battery_power_w:           i32,   // negative = discharging, positive = charging
    pub dc_input_power1_w:         i32,
    pub dc_input_power2_w:         i32,
    pub total_dc_output_power_w:   i32,
    pub total_ac_output_power_w:   i32,
    pub total_ac_input_power_w:    i32,
    pub meter_power_w:             i32,   // positive = import, negative = export
    pub daily_production_kwh:      f64,
    pub cumulative_production_kwh: f64,
    pub daily_charging_kwh:        f64,
    pub daily_discharging_kwh:     f64,
    pub total_charging_kwh:        f64,
    pub total_discharging_kwh:     f64,
    pub total_ac_input_energy_kwh: f64,
}

/// Battery static configuration read from the device (mirrors BatteryConfig table).
#[derive(Debug, Clone, Default)]
pub struct BatteryConfig {
    pub device_model:         String,
    pub rated_capacity_kwh:   f64,
    pub min_soc_percent:      f64,
    pub max_soc_percent:      f64,
    pub max_charge_power_w:   i32,
    pub max_discharge_power_w: i32,
}

// --------------------------------------------------------------------------------------------------------------
// Working modes accepted by POST /device/control  key="WorkingMode"

#[derive(Debug, Clone, PartialEq)]
pub enum WorkingMode {
    /// Default: use solar first, battery as buffer
    SelfConsumedPrioritized,
    /// Force charge from grid (arbitrage / negative pricing)
    ChargingFromGrid,
    /// Force discharge to loads / grid
    DischargingToGrid,
    /// Fully managed by the EMS; no automatic switching
    Manual,
}

impl WorkingMode {
    /// Convert to the string value the Indevolt API expects.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            WorkingMode::SelfConsumedPrioritized => "Self-consumed Prioritized",
            WorkingMode::ChargingFromGrid        => "Charging From Grid",
            WorkingMode::DischargingToGrid       => "Discharging To Grid",
            WorkingMode::Manual                  => "Manual",
        }
    }

    /// Parse from what the API returns (inverse of as_api_str).
    pub fn from_api_str(s: &str) -> Option<Self> {
        match s {
            "Self-consumed Prioritized" => Some(WorkingMode::SelfConsumedPrioritized),
            "Charging From Grid"        => Some(WorkingMode::ChargingFromGrid),
            "Discharging To Grid"       => Some(WorkingMode::DischargingToGrid),
            "Manual"                    => Some(WorkingMode::Manual),
            _                           => None,
        }
    }
}

// --------------------------------------------------------------------------------------------------------------
// Control command sent to POST /device/control

#[derive(Serialize, Debug)]
pub struct ControlCommand {
    pub key:   String,
    pub value: String,
}

impl ControlCommand {
    pub fn set_working_mode(mode: &WorkingMode) -> Self {
        Self {
            key:   "WorkingMode".to_string(),
            value: mode.as_api_str().to_string(),
        }
    }

    pub fn set_charge_power(watts: i32) -> Self {
        Self {
            key:   "MaxChargePower".to_string(),
            value: watts.to_string(),
        }
    }

    pub fn set_discharge_power(watts: i32) -> Self {
        Self {
            key:   "MaxDischargePower".to_string(),
            value: watts.to_string(),
        }
    }
}
