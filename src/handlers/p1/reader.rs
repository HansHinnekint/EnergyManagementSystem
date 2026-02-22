use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
use log::{debug, error, warn};

use crate::models::p1_models::{fetch_p1_data, P1Data};

// --------------------------------------------------------------------------------------------------------------

/// Parse the compact 12-character P1 timestamp (YYMMDDHHmmss, local time) into UTC.
/// The HomeWizard firmware encodes the timestamp in local Belgian time without a zone indicator.
fn parse_p1_timestamp(timestamp: &str) -> Result<DateTime<Utc>, String> {
    if timestamp.len() != 12 {
        return Err(format!("Invalid P1 timestamp length (expected 12): '{}'", timestamp));
    }

    let formatted = format!(
        "20{}-{}-{} {}:{}:{}",
        &timestamp[0..2],
        &timestamp[2..4],
        &timestamp[4..6],
        &timestamp[6..8],
        &timestamp[8..10],
        &timestamp[10..12],
    );

    match NaiveDateTime::parse_from_str(&formatted, "%Y-%m-%d %H:%M:%S") {
        Ok(naive) => {
            match chrono::Local.from_local_datetime(&naive).single() {
                Some(local_dt) => {
                    debug!("Parsed P1 timestamp '{}' → {}", timestamp, local_dt.with_timezone(&Utc));
                    Ok(local_dt.with_timezone(&Utc))
                }
                None => Err(format!("Ambiguous local→UTC conversion for '{}'", timestamp)),
            }
        }
        Err(e) => Err(format!("Cannot parse '{}': {}", timestamp, e)),
    }
}

// --------------------------------------------------------------------------------------------------------------

/// A fully resolved P1 reading with timestamps already converted to UTC.
#[derive(Debug, Clone)]
pub struct P1Reading {
    pub raw: P1Data,
    pub monthly_power_peak_timestamp_utc: DateTime<Utc>,
    pub gas_timestamp_utc:                DateTime<Utc>,
}

// --------------------------------------------------------------------------------------------------------------

/// Fetch and parse one P1 reading from the HomeWizard API.
/// Returns `None` on any HTTP or parse error so the caller can skip and retry next cycle.
pub async fn read_p1(url: &str) -> Option<P1Reading> {
    let json = match fetch_p1_data(url).await {
        Ok(j)  => j,
        Err(e) => {
            error!("[P1] HTTP error fetching {}: {}", url, e);
            return None;
        }
    };

    let raw = match P1Data::from_json(&json) {
        Ok(d)  => d,
        Err(e) => {
            error!("[P1] JSON parse error: {}", e);
            return None;
        }
    };

    let monthly_power_peak_timestamp_utc = match parse_p1_timestamp(&raw.montly_power_peak_timestamp) {
        Ok(ts) => ts,
        Err(e) => {
            warn!("[P1] monthly_power_peak_timestamp parse failed ({}); using now()", e);
            Utc::now()
        }
    };

    let gas_timestamp_utc = match parse_p1_timestamp(&raw.gas_timestamp) {
        Ok(ts) => ts,
        Err(e) => {
            warn!("[P1] gas_timestamp parse failed ({}); using now()", e);
            Utc::now()
        }
    };

    Some(P1Reading {
        raw,
        monthly_power_peak_timestamp_utc,
        gas_timestamp_utc,
    })
}
