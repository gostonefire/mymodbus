//! # Modbus Register Definitions
//!
//! This module defines the structure for Modbus registers and provides a static database
//! of known registers for a Modbus device, typically an inverter.
//!
//! Each register entry contains its name, address, data type, and optional unit/scale information.

use std::collections::HashMap;
use std::sync::OnceLock;

/// Metadata for a specific Modbus register.
#[derive(Debug, Clone)]
pub struct RegisterInfo {
    /// Friendly name for display.
    pub name: &'static str,
    /// Starting Modbus register address.
    pub address: u16,
    /// Data type representation (e.g., "uint16", "uint32", "string").
    pub data_type: &'static str,
    /// Whether it's an input register.
    pub input_type: Option<&'static str>,
    /// Number of registers for this entry (required for strings).
    pub count: Option<u16>,
    /// Device class (e.g., "voltage", "current").
    pub device_class: Option<&'static str>,
    /// Unit of measurement (e.g., "V", "A", "kWh").
    pub unit_of_measurement: Option<&'static str>,
    /// Scale factor for the raw register value.
    pub scale: Option<f64>,
    /// Number of decimals to show in the UI.
    pub precision: Option<u8>,
    /// Home Assistant style state class (e.g., "measurement", "total").
    pub state_class: Option<&'static str>,
}

/// A lazily initialized hash map of register definitions.
pub static REGISTERS: OnceLock<HashMap<&'static str, RegisterInfo>> = OnceLock::new();

/// Provides access to the static register database.
///
/// This function initializes the `REGISTERS` map on the first call.
pub fn register_db() -> &'static HashMap<&'static str, RegisterInfo> {
    REGISTERS.get_or_init(|| {
        let mut map = HashMap::new();
        map.insert("inverter_model", RegisterInfo { name: "Inverter Model", address: 30000, data_type: "string", input_type: None, count: Some(16), device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: None });
        map.insert("master_version_code", RegisterInfo { name: "Master Version Code", address: 30016, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("slave_version_code", RegisterInfo { name: "Slave Version Code", address: 30017, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("manager_version_code", RegisterInfo { name: "Manager Version Code", address: 30018, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("bms_version_code", RegisterInfo { name: "BMS Version code", address: 30019, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_1_version_code", RegisterInfo { name: "Battery 1 Version code", address: 30020, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_2_version_code", RegisterInfo { name: "Battery 2 Version code", address: 30021, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_3_version_code", RegisterInfo { name: "Battery 3 Version code", address: 30022, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_4_version_code", RegisterInfo { name: "Battery 4 Version code", address: 30023, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_5_version_code", RegisterInfo { name: "Battery 5 Version code", address: 30024, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_6_version_code", RegisterInfo { name: "Battery 6 Version code", address: 30025, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_7_version_code", RegisterInfo { name: "Battery 7 Version code", address: 30026, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_8_version_code", RegisterInfo { name: "Battery 8 Version code", address: 30027, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("battery_9_version_code", RegisterInfo { name: "Battery 9 Version code", address: 30028, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("protocol_version_code", RegisterInfo { name: "Protocol Version Code", address: 30100, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("pv1_voltage", RegisterInfo { name: "PV1 Voltage", address: 31000, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("pv1_current", RegisterInfo { name: "PV1 Current", address: 31001, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("pv1_power", RegisterInfo { name: "PV1 Power", address: 31002, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("pv2_voltage", RegisterInfo { name: "PV2 Voltage", address: 31003, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("pv2_current", RegisterInfo { name: "PV2 Current", address: 31004, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("pv2_power", RegisterInfo { name: "PV2 Power", address: 31005, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("grid_voltage_R", RegisterInfo { name: "RVolt", address: 31006, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("grid_voltage_S", RegisterInfo { name: "SVolt", address: 31007, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("grid_voltage_T", RegisterInfo { name: "TVolt", address: 31008, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("inv_current_R", RegisterInfo { name: "RCurrent", address: 31009, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("inv_current_S", RegisterInfo { name: "SCurrent", address: 31010, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("inv_current_T", RegisterInfo { name: "TCurrent", address: 31011, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("inv_power_R", RegisterInfo { name: "RPower", address: 31012, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("inv_power_S", RegisterInfo { name: "SPower", address: 31013, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("inv_power_T", RegisterInfo { name: "TPower", address: 31014, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("grid_frequency", RegisterInfo { name: "Grid Frequency", address: 31015, data_type: "uint16", input_type: None, count: None, device_class: Some("frequency"), unit_of_measurement: Some("Hz"), scale: Some(0.01), precision: Some(3), state_class: Some("measurement") });
        map.insert("eps_voltage_R", RegisterInfo { name: "EPS RVolt", address: 31016, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("eps_voltage_S", RegisterInfo { name: "EPS SVolt", address: 31017, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("eps_voltage_T", RegisterInfo { name: "EPS TVolt", address: 31018, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("eps_current_R", RegisterInfo { name: "EPS RCurrent", address: 31019, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("eps_current_S", RegisterInfo { name: "EPS SCurrent", address: 31020, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("eps_current_T", RegisterInfo { name: "EPS TCurrent", address: 31021, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("eps_power_R", RegisterInfo { name: "EPS RPower", address: 31022, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("eps_power_S", RegisterInfo { name: "EPS SPower", address: 31023, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("eps_power_T", RegisterInfo { name: "EPS TPower", address: 31024, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("eps_frequency", RegisterInfo { name: "EPS Frequency", address: 31025, data_type: "uint16", input_type: None, count: None, device_class: Some("frequency"), unit_of_measurement: Some("Hz"), scale: Some(0.01), precision: Some(1), state_class: Some("measurement") });
        map.insert("meter1_power_R", RegisterInfo { name: "Meter RPower", address: 31026, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("meter1_power_S", RegisterInfo { name: "Meter SPower", address: 31027, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("meter1_power_T", RegisterInfo { name: "Meter TPower", address: 31028, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("load_power_R", RegisterInfo { name: "Load RPower", address: 31029, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("load_power_S", RegisterInfo { name: "Load SPower", address: 31030, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("load_power_T", RegisterInfo { name: "Load TPower", address: 31031, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("inverter_temperature", RegisterInfo { name: "Inverter Temperature", address: 31032, data_type: "uint16", input_type: None, count: None, device_class: Some("temperature"), unit_of_measurement: Some("°C"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("ambient_temperature", RegisterInfo { name: "Inner Temperature", address: 31033, data_type: "uint16", input_type: None, count: None, device_class: Some("temperature"), unit_of_measurement: Some("°C"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("battery_voltage", RegisterInfo { name: "InvBatVolt", address: 31034, data_type: "uint16", input_type: None, count: None, device_class: Some("voltage"), unit_of_measurement: Some("V"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("battery_current", RegisterInfo { name: "InvBatCurrent", address: 31035, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("battery_power", RegisterInfo { name: "Battery Discharge Power", address: 31036, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("battery_temperature", RegisterInfo { name: "Battery Temperature", address: 31037, data_type: "uint16", input_type: None, count: None, device_class: Some("temperature"), unit_of_measurement: Some("°C"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("battery_soc", RegisterInfo { name: "Battery SoC", address: 31038, data_type: "uint16", input_type: None, count: None, device_class: Some("battery"), unit_of_measurement: Some("%"), scale: None, precision: None, state_class: Some("measurement") });
        map.insert("bms_charge_rate", RegisterInfo { name: "BMS Max Charge Current", address: 31039, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("bms_discharge_rate", RegisterInfo { name: "BMS Max Discharge Current", address: 31040, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("inverter_state_code", RegisterInfo { name: "Inverter State Code", address: 31041, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("bms_connection_code", RegisterInfo { name: "BMS Connection Code", address: 31042, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("meter_1_connection_code", RegisterInfo { name: "Meter 1 Connection Code", address: 31043, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_1_code", RegisterInfo { name: "Fault 1 Code", address: 31044, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_2_code", RegisterInfo { name: "Fault 2 Code", address: 31045, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_3_code", RegisterInfo { name: "Fault 3 Code", address: 31046, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_4_code", RegisterInfo { name: "Fault 4 Code", address: 31047, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_5_code", RegisterInfo { name: "Fault 5 Code", address: 31048, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_6_code", RegisterInfo { name: "Fault 6 Code", address: 31049, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_7_code", RegisterInfo { name: "Fault 7 Code", address: 31050, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("fault_8_code", RegisterInfo { name: "Fault 8 Code", address: 31051, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("pv_energy_total", RegisterInfo { name: "PV Energy Total", address: 32000, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("pv_energy_today", RegisterInfo { name: "PV Energy Today", address: 32002, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("charge_energy_total", RegisterInfo { name: "Charge Energy Total", address: 32003, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("charge_energy_today", RegisterInfo { name: "Charge Energy Today", address: 32005, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("discharge_energy_total", RegisterInfo { name: "Discharge Energy Total", address: 32006, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("discharge_energy_today", RegisterInfo { name: "Discharge Energy Today", address: 32008, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("feed_in_energy_total", RegisterInfo { name: "Feed In Energy Total", address: 32009, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("feed_in_energy_today", RegisterInfo { name: "Feed In Energy Today", address: 32011, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("grid_consumption_energy_total", RegisterInfo { name: "Grid Consumption Energy Total", address: 32012, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("grid_consumption_energy_today", RegisterInfo { name: "Grid Consumption Energy Today", address: 32014, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("output_energy_total", RegisterInfo { name: "Output Energy Total", address: 32015, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("output_energy_today", RegisterInfo { name: "Output Energy Today", address: 32017, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("input_energy_total", RegisterInfo { name: "Input Energy Total", address: 32018, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("input_energy_today", RegisterInfo { name: "Input Energy Today", address: 32020, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("load_energy_total", RegisterInfo { name: "Load Energy Total", address: 32021, data_type: "uint32", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("load_energy_today", RegisterInfo { name: "Load Energy Today", address: 32023, data_type: "uint16", input_type: None, count: None, device_class: Some("energy"), unit_of_measurement: Some("kWh"), scale: Some(0.1), precision: Some(1), state_class: Some("total") });
        map.insert("rtc_hour", RegisterInfo { name: "RTC Hour", address: 40003, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("work_mode_code", RegisterInfo { name: "Work Mode Code", address: 41000, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("time_period_1_enabled", RegisterInfo { name: "Time Period 1 Enabled", address: 41001, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("time_period_1_start", RegisterInfo { name: "Time Period 1 Start", address: 41002, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("time_period_1_end", RegisterInfo { name: "Time Period 1 End", address: 41003, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("time_period_2_enabled", RegisterInfo { name: "Time Period 2 Enabled", address: 41004, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("time_period_2_start", RegisterInfo { name: "Time Period 2 Start", address: 41005, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("time_period_2_end", RegisterInfo { name: "Time Period 2 End", address: 41006, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("max_charge_current", RegisterInfo { name: "Max Charge Current", address: 41007, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("max_discharge_current", RegisterInfo { name: "Max Discharge Current", address: 41008, data_type: "uint16", input_type: None, count: None, device_class: Some("current"), unit_of_measurement: Some("A"), scale: Some(0.1), precision: Some(1), state_class: Some("measurement") });
        map.insert("min_soc", RegisterInfo { name: "Min SoC", address: 41009, data_type: "uint16", input_type: None, count: None, device_class: Some("battery"), unit_of_measurement: Some("%"), scale: None, precision: None, state_class: Some("measurement") });
        map.insert("max_soc", RegisterInfo { name: "Max SoC", address: 41010, data_type: "uint16", input_type: None, count: None, device_class: Some("battery"), unit_of_measurement: Some("%"), scale: None, precision: None, state_class: Some("measurement") });
        map.insert("min_soc_on_grid", RegisterInfo { name: "Min SoC On Grid", address: 41011, data_type: "uint16", input_type: None, count: None, device_class: Some("battery"), unit_of_measurement: Some("%"), scale: None, precision: None, state_class: Some("measurement") });
        map.insert("export_limit", RegisterInfo { name: "Export Limit", address: 41012, data_type: "uint16", input_type: None, count: None, device_class: Some("power"), unit_of_measurement: Some("kW"), scale: Some(0.001), precision: Some(3), state_class: Some("measurement") });
        map.insert("system_enable", RegisterInfo { name: "System Enable", address: 41014, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("eps_enable_code", RegisterInfo { name: "EPS Enable Code", address: 41015, data_type: "uint16", input_type: None, count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("remote_control_code", RegisterInfo { name: "Remote Control Code", address: 44000, data_type: "uint16", input_type: Some("input"), count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("remote_timeout_set", RegisterInfo { name: "Remote Timeout Set", address: 44001, data_type: "uint16", input_type: Some("input"), count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("remote_control_active_power_command", RegisterInfo { name: "Remote Control Active Power Command", address: 44002, data_type: "int32", input_type: Some("input"), count: None, device_class: Some("power"), unit_of_measurement: Some("W"), scale: Some(1.0), precision: Some(0), state_class: Some("measurement") });
        map.insert("remote_control_reactive_power_command", RegisterInfo { name: "Remote Control Reactive Power Command", address: 44004, data_type: "int32", input_type: Some("input"), count: None, device_class: Some("power"), unit_of_measurement: Some("VAR"), scale: Some(1.0), precision: Some(0), state_class: Some("measurement") });
        map.insert("remote_timeout_countdown", RegisterInfo { name: "Remote Timeout Countdown", address: 44006, data_type: "uint16", input_type: Some("input"), count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("remote_take_effect", RegisterInfo { name: "Remote Take Effect", address: 44007, data_type: "uint16", input_type: Some("input"), count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("remote_not_active_reason_code", RegisterInfo { name: "Remote Not Active Reason Code", address: 44008, data_type: "uint16", input_type: Some("input"), count: None, device_class: None, unit_of_measurement: None, scale: None, precision: None, state_class: Some("measurement") });
        map.insert("remote_pwr_limit_bat_up", RegisterInfo { name: "Remote Pwr_Limit Bat_Up", address: 44012, data_type: "uint16", input_type: Some("input"), count: None, device_class: Some("power"), unit_of_measurement: Some("W"), scale: Some(1.0), precision: Some(0), state_class: Some("measurement") });
        map.insert("remote_pwr_limit_bat_dn", RegisterInfo { name: "Remote Pwr_Limit Bat_Dn", address: 44013, data_type: "uint16", input_type: Some("input"), count: None, device_class: Some("power"), unit_of_measurement: Some("W"), scale: Some(1.0), precision: Some(0), state_class: Some("measurement") });
        map
    })
}

/// Look up a register by its unique identifier.
///
/// # Arguments
///
/// * `unique_id` - The identifier of the register (e.g., "pv1_voltage").
pub fn get_register(unique_id: &str) -> Option<&'static RegisterInfo> {
    register_db().get(unique_id)
}
