//! # Modbus CLI Tool
//!
//! This application provides a command-line interface for reading data from a Modbus device
//! (e.g., an inverter) via a serial port using the RTU protocol.
//!
//! The tool resolves register identifiers into their corresponding Modbus addresses
//! and data types using a predefined register database, performs the serial communication,
//! and displays the results.

mod manager_modbus;
mod registers;

use crate::manager_modbus::{Modbus, RegisterValue};
use anyhow::{anyhow, Result};
use std::env;

/// The default serial port to connect to.
const PORT: &str = "/dev/ttyACM0";

/// Main entry point for the Modbus CLI tool.
///
/// This function:
/// 1. Parses the command-line arguments to get the register identifier.
/// 2. Initializes the Modbus client.
/// 3. Resolves the register metadata.
/// 4. Reads the register value and displays it.
///
/// # Arguments
///
/// * `args` - Command-line arguments from the environment.
fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <unique_id>", args[0]);
        println!("Example: {} battery_soc", args[0]);
        return Ok(());
    }

    let input = &args[1];
    let mut client = Modbus::new(PORT)?;


    let info = client
        .get_register_info(input)
        .ok_or_else(|| anyhow!("unknown register id: {input}"))?;

    println!("Resolved register:");
    println!("  name: {}", info.name);
    println!("  address: {}", info.address);
    println!("  data_type: {}", info.data_type);
    println!("  input_type: {:?}", info.input_type);
    println!("  count: {:?}", info.count);
    println!("  device_class: {:?}", info.device_class);
    println!("  unit_of_measurement: {:?}", info.unit_of_measurement);
    println!("  scale: {:?}", info.scale);
    println!("  precision: {:?}", info.precision);
    println!("  state_class: {:?}", info.state_class);

    match client.read_register_by_id_typed(input)? {
        RegisterValue::U16(v) => println!("Read value (u16): {v}"),
        RegisterValue::U32(v) => println!("Read value (u32): {v}"),
        RegisterValue::I32(v) => println!("Read value (i32): {v}"),
        RegisterValue::String(v) => println!("Read value (string): {v}"),
    }

    Ok(())
}

