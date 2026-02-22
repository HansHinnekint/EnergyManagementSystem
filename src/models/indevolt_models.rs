use serde::Serialize;

// --------------------------------------------------------------------------------------------------------------
// Indevolt PowerFlex2000 local RPC API models
//
// Read:  GET  /rpc/Indevolt.GetData?config={"t":[id,...]}
//        Response: flat JSON object {"<id>": <numeric_value>, ...}
//
// Write: POST /rpc/Indevolt.SetData
//        Body: {"id": <numeric_id>, "value": <value>}
// --------------------------------------------------------------------------------------------------------------

/// Config parameter for GET /rpc/Indevolt.SetData?config=<json>
/// Example: {"f":16,"t":47015,"v":[1,2000,100]}
#[derive(Serialize, Debug)]
pub struct SetDataConfig {
    pub f: u32,       // Modbus function code (always 16)
    pub t: u32,       // Register address
    pub v: Vec<i64>,  // Values to write
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
// Working modes for register 47005

#[derive(Debug, Clone, PartialEq)]
pub enum WorkingMode {
    /// Default: use solar first, battery as buffer (value = 1)
    SelfConsumedPrioritized,
    /// EMS takes direct real-time control via register 47015 (value = 4)
    RealtimeControl,
    /// Time-based charge/discharge schedule (value = 5)
    Schedule,
}

impl WorkingMode {
    pub fn register_value(&self) -> i64 {
        match self {
            WorkingMode::SelfConsumedPrioritized => 1,
            WorkingMode::RealtimeControl         => 4,
            WorkingMode::Schedule                => 5,
        }
    }

    pub fn from_register_value(v: i64) -> Option<Self> {
        match v {
            1 => Some(WorkingMode::SelfConsumedPrioritized),
            4 => Some(WorkingMode::RealtimeControl),
            5 => Some(WorkingMode::Schedule),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            WorkingMode::SelfConsumedPrioritized => "Self-consumed Prioritized",
            WorkingMode::RealtimeControl         => "Real-time Control",
            WorkingMode::Schedule                => "Schedule",
        }
    }
}


