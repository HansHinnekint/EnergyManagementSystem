use serde::Deserialize;
use std::fs;

// --------------------------------------------------------------------------------------------------------------

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    // --- connectivity ---

    /// HomeWizard P1 meter local API endpoint, e.g. "http://192.168.1.x/api/v1/data"
    pub p1_url: String,
    /// Indevolt PowerFlex base URL, e.g. "http://192.168.1.y"
    pub indevolt_url: String,
    /// Single loop interval: P1 read -> battery read -> optimiser -> sleep.
    /// 30s matches the HomeWizard P1 update rate.
    pub poll_interval_seconds: u64,

    // --- battery physical parameters ---

    /// Rated (nameplate) capacity of the battery cluster in kWh.
    pub battery_rated_capacity_kwh: f64,
    /// Minimum SOC to always keep in reserve for battery health / BMS functioning (%).
    /// Below this the optimiser will never discharge, regardless of price signals.
    pub battery_min_soc_percent: f64,
    /// Maximum SOC target (%). Normally 100, lower it to extend cycle life if desired.
    pub battery_max_soc_percent: f64,

    // --- grid power limits ---

    /// Maximum power the inverter may draw from the grid to charge the battery (W).
    /// Current hardware limit: 2400 W. Update to 7200 W after the planned upgrade.
    pub battery_max_charge_power_w: i32,
    /// Maximum power the inverter may push back to the grid / feed loads from the battery (W).
    /// Current hardware limit: 2400 W. Update to 7200 W after the planned upgrade.
    pub battery_max_discharge_power_w: i32,

    // --- optimiser thresholds ---

    /// Belgian capacity tariff peak limit (W). The optimiser will not let total grid import
    /// exceed this during peak hours to avoid a higher monthly capacity bill.
    pub battery_max_desired_grid_peak_w: i32,
    /// Minimum price spread required to justify a grid charge/discharge cycle (%).
    /// Covers round-trip efficiency losses (~85%). Default 25% from your BatteryConfig table.
    pub battery_min_price_spread_percent: f64,
    /// Round-trip efficiency of the battery (0.0-1.0). Used by the optimiser when calculating
    /// whether a charge/discharge cycle is profitable at a given price spread.
    pub battery_round_trip_efficiency: f64,

    // --- logging ---

    /// Log level: "Trace", "Debug", "Info", "Warn", "Error"
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // connectivity
            p1_url:               "http://127.0.0.1/api/v1/data".to_string(),
            indevolt_url:         "http://127.0.0.1".to_string(),
            poll_interval_seconds: 30,
            // battery physical - values from your live BatteryConfig table
            battery_rated_capacity_kwh:    12.0,
            battery_min_soc_percent:       10.0,
            battery_max_soc_percent:       100.0,
            // grid power limits - current 2400 W hardware; raise to 7200 after upgrade
            battery_max_charge_power_w:    2400,
            battery_max_discharge_power_w: 2400,
            // optimiser thresholds - from your live BatteryConfig table
            battery_max_desired_grid_peak_w:  3381,
            battery_min_price_spread_percent: 25.0,
            battery_round_trip_efficiency:    0.80,
            // logging
            log_level: "Info".to_string(),
        }
    }
}

impl Config {
    /// Usable capacity after reserving the minimum SOC buffer (kWh).
    pub fn usable_capacity_kwh(&self) -> f64 {
        self.battery_rated_capacity_kwh
            * (self.battery_max_soc_percent - self.battery_min_soc_percent)
            / 100.0
    }
}

// --------------------------------------------------------------------------------------------------------------

pub fn load_config() -> Config {
    let config_file = "config.json";
    let config_data = fs::read_to_string(config_file)
        .expect("Failed to read configuration file");
    serde_json::from_str(&config_data)
        .expect("Failed to parse configuration file")
}
