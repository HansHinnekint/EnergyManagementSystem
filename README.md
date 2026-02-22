# Energy Management System

A Rust/Tokio daemon that reads a **HomeWizard P1 smart meter** and an **Indevolt PowerFlex2000 battery inverter** over their local HTTP APIs, reconciles the two grid-power readings, and will later drive autonomous charge/discharge scheduling.

---

## Hardware

| Device | Model | Local IP |
|--------|-------|----------|
| Smart meter | HomeWizard Wi-Fi P1 | `172.19.11.76` |
| Battery inverter | Indevolt PowerFlex2000 | `172.19.11.102` |

---

## Architecture

```
main loop (configurable interval, default 1 s)
  │
  ├─ Step 1: GET /api/v1/data          → P1 reading  (HomeWizard)
  ├─ Step 2: GET /rpc/Indevolt.GetData → battery snapshot (Indevolt RPC)
  ├─ Step 3: log [EMS] summary line
  └─ Step 4: optimiser  (← placeholder, not yet implemented)
```

All steps are sequential within a cycle so the battery decision always uses readings from the same polling epoch.

---

## Device APIs

### HomeWizard P1 — read

```
GET http://<ip>/api/v1/data
```

Returns JSON with active power, per-phase voltage/current, and energy totals.

### Indevolt PowerFlex2000 — read

```
GET http://<ip>:8080/rpc/Indevolt.GetData?config={"t":[<id>,...]}
```

Returns a flat JSON object `{"<id>": <numeric_value>, ...}`.

All 17 sensor IDs are fetched in a single call per cycle.

| Register ID | Description | Unit / Notes |
|-------------|-------------|--------------|
| 7101 | Working mode | 1=Self-consumed, 4=Realtime, 5=Schedule |
| 1664 | DC input power PV1 | W |
| 1665 | DC input power PV2 | W |
| 1501 | Total DC output power | W |
| 2108 | Total AC output power | W |
| 1502 | Daily production | kWh |
| 1505 | Cumulative production | raw × 0.001 → kWh |
| 2101 | Total AC input power | W |
| 2107 | Total AC input energy | kWh |
| 6000 | Battery power | W |
| 6001 | Battery state | 1000=Static, 1001=Charging, 1002=Discharging |
| 6002 | Battery SOC | % |
| 6004 | Battery daily charging | kWh |
| 6005 | Battery daily discharging | kWh |
| 6006 | Battery total charging | kWh |
| 6007 | Battery total discharging | kWh |
| 11016 | Meter power (grid CT) | W, positive = import; updates ~every 5 s |

> **Note:** Register 11016 is updated by the inverter firmware roughly every 5 seconds regardless of how fast the EMS polls. Polling faster than 5 s gives no benefit for this register.

### Indevolt PowerFlex2000 — control

```
GET http://<ip>:8080/rpc/Indevolt.SetData?config={"f":16,"t":<reg>,"v":[...]}
```

| Register | Purpose | Values |
|----------|---------|--------|
| 47005 | Working mode | 1=Self-consumed, 4=Realtime, 5=Schedule |
| 47015 | Real-time control | v=[action, watts, soc_limit] where action 0=Stop, 1=Charge, 2=Discharge |

**Control sequence:**
1. Switch to real-time mode: `set_working_mode(RealtimeControl)` (reg 47005 = 4)
2. Issue command: `charge(watts, max_soc_%)` or `discharge(watts, min_soc_%)` (reg 47015)
3. Restore auto: `restore_auto_mode()` (reg 47005 = 1)

---

## Configuration (`config.json`)

```json
{
    "p1_url":                        "http://172.19.11.76/api/v1/data",
    "indevolt_url":                  "http://172.19.11.102:8080",
    "poll_interval_seconds":         1,

    "battery_rated_capacity_kwh":    12.0,
    "battery_min_soc_percent":       10.0,
    "battery_max_soc_percent":       100.0,

    "battery_max_charge_power_w":    2400,
    "battery_max_discharge_power_w": 2400,

    "battery_max_desired_grid_peak_w":  4000,
    "battery_min_price_spread_percent": 25.0,
    "battery_round_trip_efficiency":    0.80,

    "log_level": "Info"
}
```

Set `log_level` to `"Debug"` to see per-phase P1 data and full battery sensor detail each cycle.

---

## Build & Run

### Prerequisites

```bash
sudo apt-get install -y libssl-dev pkg-config
```

### Run

```bash
cargo run
```

### Example output (Info level)

```
[EMS] P1=+2090W  Indevolt=+2092W  diff=-2W | SOC=10.0% Static Self-consumed Prioritized bat=+0W
[EMS] Cycle done in 251ms. Sleeping 749ms.
```

---

## Project Structure

```
src/
├── main.rs                          # Control loop
├── configuration/
│   └── config.rs                    # Config loader (config.json)
├── models/
│   ├── p1_models.rs                 # HomeWizard P1 API response types
│   └── indevolt_models.rs           # BatterySnapshot, SetDataConfig, WorkingMode
└── handlers/
    ├── p1/
    │   └── reader.rs                # GET /api/v1/data → P1Reading
    └── indevolt/
        ├── reader.rs                # GET /rpc/Indevolt.GetData → BatterySnapshot
        └── controller.rs           # GET /rpc/Indevolt.SetData (charge/discharge/mode)
```

---

## Roadmap

- [x] P1 meter reading
- [x] Indevolt bulk sensor read (17 IDs, single HTTP call)
- [x] Indevolt control API (charge / discharge / working mode)
- [ ] Optimiser: peak-shaving / price-spread arbitrage
- [ ] Day-ahead price feed integration
- [ ] Schedule-mode support (register 47005 = 5)
