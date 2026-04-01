mod manager_modbus;

use crate::manager_modbus::Modbus;
use anyhow::{anyhow, Result};
use std::env;

const PORT: &str = "/dev/ttyACM0"; // Changed to a more generic name for Windows example

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <address>u16|u32", args[0]);
        println!("Example: {} 31038u16", args[0]);
        return Ok(());
    }

    let input = &args[1];
    let mut client = Modbus::new(PORT)?;

    if let Some(pos) = input.find("u16") {
        let addr_str = &input[..pos];
        let addr: u16 = addr_str.parse().map_err(|_| anyhow!("Invalid address: {}", addr_str))?;
        println!("Reading address {addr} as u16...");
        let val: u16 = client.read_register(addr)?;
        println!("Value (u16): {val}");
    } else if let Some(pos) = input.find("u32") {
        let addr_str = &input[..pos];
        let addr: u16 = addr_str.parse().map_err(|_| anyhow!("Invalid address: {}", addr_str))?;
        println!("Reading address {addr} as u32...");
        let val: u32 = client.read_register(addr)?;
        println!("Value (u32): {val}");
    } else {
        return Err(anyhow!("Input should be in the format of e.g. '31038u16' or '31038u32'"));
    }

    Ok(())
}

