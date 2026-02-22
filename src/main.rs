use std::time::Instant;
use log::LevelFilter;
use tokio::time::{sleep, Duration};

// --------------------------------------------------------------------------------------------------------------

mod configuration;
use configuration::config::load_config;

mod models;

mod handlers;
use handlers::p1::reader::read_p1;
use handlers::indevolt::reader::read_battery_snapshot;

// --------------------------------------------------------------------------------------------------------------
// Device model string - adjust if yours differs from the n8n logging.
const DEVICE_MODEL: &str = "PowerFlex2000";

// --------------------------------------------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let config = load_config();

    // Initialise logger.
    if let Err(e) = env_logger::Builder::new()
        .filter_level(config.log_level.parse::<LevelFilter>().unwrap_or(LevelFilter::Info))
        .try_init()
    {
        eprintln!("Failed to initialise logger: {}", e);
        panic!("Cannot start without logging");
    }

    log::info!("=== Energy Management System starting ===");
    log::info!("P1 URL:       {}", config.p1_url);
    log::info!("Indevolt URL: {}", config.indevolt_url);
    log::info!("Poll interval: {}s", config.poll_interval_seconds);

    let interval = Duration::from_secs(config.poll_interval_seconds);

    // ----------------------------------------------------------------------------------------------------------
    // Single control loop: read P1 → read battery → decide → act → sleep.
    // Keeping this sequential means every battery decision is based on the
    // freshest possible P1 reading from the same cycle.
    loop {
        let cycle_start = Instant::now();

        // Step 1: read the smart meter.
        let p1 = read_p1(&config.p1_url).await;

        // Step 2: read the battery state.
        let battery = read_battery_snapshot(&config.indevolt_url, DEVICE_MODEL).await;

        // Step 3: log what we have.
        match &p1 {
            Some(reading) => {
                let r = &reading.raw;
                log::debug!(
                    "[P1] tariff={} power={:+.0}W import={:.3}kWh export={:.3}kWh",
                    r.active_tariff,
                    r.active_power_w,
                    r.total_power_import_kwh,
                    r.total_power_export_kwh,
                );
                log::debug!(
                    "[P1] L1={:+.0}W L2={:+.0}W L3={:+.0}W | {:.1}V {:.1}V {:.1}V",
                    r.active_power_l1_w, r.active_power_l2_w, r.active_power_l3_w,
                    r.active_voltage_l1_v, r.active_voltage_l2_v, r.active_voltage_l3_v,
                );
            }
            None => log::warn!("[P1] No reading this cycle - skipping optimiser."),
        }

        log::debug!(
            "[Battery] SOC={:.1}% state={} mode={} power={:+}W meter={:+}W",
            battery.battery_soc,
            battery.battery_state,
            battery.working_mode,
            battery.battery_power_w,
            battery.meter_power_w,
        );
        log::debug!(
            "[Battery] DC1={:+}W DC2={:+}W | AC_out={:+}W AC_in={:+}W",
            battery.dc_input_power1_w,
            battery.dc_input_power2_w,
            battery.total_ac_output_power_w,
            battery.total_ac_input_power_w,
        );
        log::debug!(
            "[Battery] daily prod={:.3}kWh chrg={:.3}kWh dischrg={:.3}kWh",
            battery.daily_production_kwh,
            battery.daily_charging_kwh,
            battery.daily_discharging_kwh,
        );

        // Step 3b: reconciliation line — P1 vs Indevolt meter vs difference.
        if let Some(ref reading) = p1 {
            let p1_w      = reading.raw.active_power_w as i32;
            let inv_w     = battery.meter_power_w;
            let diff_w    = p1_w - inv_w;
            log::info!(
                "[EMS] P1={:+}W  Indevolt={:+}W  diff={:+}W | SOC={:.1}% {} {} bat={:+}W",
                p1_w, inv_w, diff_w,
                battery.battery_soc,
                battery.battery_state,
                battery.working_mode,
                battery.battery_power_w,
            );
        } else {
            log::warn!("[EMS] No P1 reading this cycle.");
        }

        // Step 4: optimiser (placeholder - receives both readings together).
        // if let Some(p1_reading) = p1 {
        //     optimiser::run(&p1_reading, &battery, &config).await;
        // }

        // Sleep for whatever time remains in the interval.
        let elapsed = cycle_start.elapsed();
        if elapsed < interval {
            let remaining = interval - elapsed;
            log::info!("[EMS] Cycle done in {:?}. Sleeping {:?}.", elapsed, remaining);
            sleep(remaining).await;
        } else {
            log::warn!(
                "[EMS] Cycle took {:?}, overran interval {:?} - skipping sleep.",
                elapsed, interval
            );
        }
    }
}
