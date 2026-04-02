mod manager_modbus;
mod registers;

use crate::manager_modbus::{Modbus, RegisterValue};
use anyhow::{anyhow, Result};
use std::env;

const PORT: &str = "/dev/ttyACM0"; // Changed to a more generic name for Windows example

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
    }

    Ok(())
}

