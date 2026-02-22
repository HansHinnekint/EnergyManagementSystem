use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer};
use std::fmt;
use reqwest::Error;

// --------------------------------------------------------------------------------------------------------------
// HomeWizard P1 meter returns some fields as either a number or a string depending on firmware version.
// This custom deserialiser accepts both without breaking parsing.

fn deserialize_to_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrNumber;

    impl<'de> de::Visitor<'de> for StringOrNumber {
        type Value = String;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or a number")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
        fn visit_u64<E: de::Error>(self, v: u64) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
        fn visit_i64<E: de::Error>(self, v: i64) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
        fn visit_f64<E: de::Error>(self, v: f64) -> Result<Self::Value, E> {
            Ok(v.to_string())
        }
    }

    deserializer.deserialize_any(StringOrNumber)
}

// --------------------------------------------------------------------------------------------------------------

/// An external (slave) meter attached to the P1 port.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExternalMeasurement {
    pub unique_id: String,
    pub r#type:    String,
    #[serde(deserialize_with = "deserialize_to_string")]
    pub timestamp: String,
    pub value:     f64,
    pub unit:      String,
}

// --------------------------------------------------------------------------------------------------------------

/// Full response from GET /api/v1/data on a HomeWizard P1 dongle.
/// Field names match the HomeWizard local API spec exactly.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct P1Data {
    pub wifi_ssid:               String,
    pub wifi_strength:           u8,
    pub smr_version:             u8,
    pub meter_model:             String,
    pub unique_id:               String,
    pub active_tariff:           u8,
    pub total_power_import_kwh:  f64,
    pub total_power_import_t1_kwh: f64,
    pub total_power_import_t2_kwh: f64,
    pub total_power_export_kwh:  f64,
    pub total_power_export_t1_kwh: f64,
    pub total_power_export_t2_kwh: f64,
    pub active_power_w:          f64,
    pub active_power_l1_w:       f64,
    pub active_power_l2_w:       f64,
    pub active_power_l3_w:       f64,
    pub active_voltage_l1_v:     f64,
    pub active_voltage_l2_v:     f64,
    pub active_voltage_l3_v:     f64,
    pub active_current_a:        f64,
    pub active_current_l1_a:     f64,
    pub active_current_l2_a:     f64,
    pub active_current_l3_a:     f64,
    pub active_power_average_w:  f64,
    pub montly_power_peak_w:     f64,   // note: HomeWizard typo kept intentionally
    #[serde(deserialize_with = "deserialize_to_string")]
    pub montly_power_peak_timestamp: String,
    pub total_gas_m3:            f64,
    #[serde(deserialize_with = "deserialize_to_string")]
    pub gas_timestamp:           String,
    pub gas_unique_id:           String,
    pub external:                Vec<ExternalMeasurement>,
}

impl P1Data {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

// --------------------------------------------------------------------------------------------------------------

/// Fetch the raw JSON string from the P1 local API.
pub async fn fetch_p1_data(url: &str) -> Result<String, Error> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await?
        .text()
        .await?;
    Ok(response)
}
