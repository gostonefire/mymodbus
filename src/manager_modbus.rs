//! # Modbus RTU Manager
//!
//! This module provides the core logic for communicating with Modbus RTU devices
//! over a serial port. It includes traits for reading different data types from
//! registers, a `Modbus` client for managing the connection, and helper functions
//! for frame building and parsing.

use std::fmt::Debug;
use std::time::{Duration, Instant};
use anyhow::{anyhow, Context, Result};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use crate::registers::RegisterInfo;

const BAUD: u32 = 9600;
const DATA_BITS: DataBits = DataBits::Eight;
const PARITY: Parity = Parity::None;
const STOP_BITS: StopBits = StopBits::One;
const FLOW_CONTROL: FlowControl = FlowControl::None;
const TIMEOUT: Duration = Duration::from_millis(300);
const OVERALL_TIMEOUT: Duration = Duration::from_millis(500);
const SLAVE_ID: u8 = 247;
const FUNCTION_READ_HOLDING: u8 = 0x04;

/// Trait for types that can be read from Modbus registers.
pub trait ModbusRead: Sized {
    /// The number of registers to read for this type.
    const REG_COUNT: u16;

    /// Combine the read registers into the target type.
    ///
    /// # Arguments
    ///
    /// * `regs` - A slice of 16-bit registers read from the device.
    fn from_registers(regs: &[u16]) -> Result<Self>;
}

impl ModbusRead for u16 {
    const REG_COUNT: u16 = 1;

    fn from_registers(regs: &[u16]) -> Result<Self> {
        regs.get(0)
            .copied()
            .ok_or_else(|| anyhow!("not enough registers for u16"))
    }
}

impl ModbusRead for u32 {
    const REG_COUNT: u16 = 2;

    fn from_registers(regs: &[u16]) -> Result<Self> {
        if regs.len() < 2 {
            return Err(anyhow!("not enough registers for u32"));
        }
        // Most common: high word first (BE-style across registers)
        Ok(((regs[0] as u32) << 16) | (regs[1] as u32))
    }
}

impl ModbusRead for i32 {
    const REG_COUNT: u16 = 2;

    fn from_registers(regs: &[u16]) -> Result<Self> {
        if regs.len() < 2 {
            return Err(anyhow!("not enough registers for i32"));
        }
        // Most common: high word first (BE-style across registers)
        let value = ((regs[0] as u32) << 16) | (regs[1] as u32);
        Ok(value as i32)
    }
}

pub struct Modbus {
    port: Box<dyn SerialPort>,
}

impl Modbus {
    /// Create a new Modbus client.
    /// 
    /// # Arguments
    /// 
    /// * `serial_port` - The serial port to connect to.
    pub fn new(serial_port: &str) -> Result<Self> {
        let port = serialport::new(serial_port, BAUD)
            .data_bits(DATA_BITS)
            .parity(PARITY) // or Even, depending on your inverter config
            .stop_bits(STOP_BITS)
            .flow_control(FLOW_CONTROL)
            .timeout(TIMEOUT)
            .open()
            .with_context(|| format!("failed to open {serial_port}"))?;

        Ok(Modbus { port })    
    }

    /// Get metadata for a register by its unique identifier.
    ///
    /// # Arguments
    ///
    /// * `unique_id` - The identifier of the register.
    pub fn get_register_info(&self, unique_id: &str) -> Option<&'static RegisterInfo> {
        crate::registers::get_register(unique_id)
    }

    /// Read a register by its unique identifier and return a typed value.
    ///
    /// # Arguments
    ///
    /// * `unique_id` - The identifier of the register.
    pub fn read_register_by_id_typed(&mut self, unique_id: &str) -> Result<RegisterValue> {
        let info = self
            .get_register_info(unique_id)
            .ok_or_else(|| anyhow!("unknown register id: {unique_id}"))?;

        match info.data_type {
            "u16" | "uint16" => Ok(RegisterValue::U16(self.read_register::<u16>(info.address)?)),
            "u32" | "uint32" => Ok(RegisterValue::U32(self.read_register::<u32>(info.address)?)),
            "i32" | "int32" => Ok(RegisterValue::I32(self.read_register::<i32>(info.address)?)),
            "string" => {
                let count = info.count.ok_or_else(|| anyhow!("string type requires count field"))?;
                Ok(RegisterValue::String(self.read_register_string(info.address, count)?))
            },
            other => Err(anyhow!("unsupported data_type: {other}")),
        }
    }

    /// Read a sequence of registers and interpret them as a UTF-8 string.
    ///
    /// # Arguments
    ///
    /// * `address` - The starting register address.
    /// * `count` - The number of registers to read.
    pub fn read_register_string(&mut self, address: u16, count: u16) -> Result<String> {
        let request = build_read_holding_request(SLAVE_ID, address, count);
        println!("Sending string request: address={}, count={}", address, count);

        // Clear stale bytes, then observe a quiet period before sending.
        let _ = &self.port.clear(serialport::ClearBuffer::All);
        std::thread::sleep(Duration::from_millis(5));

        self.port.write_all(&request)?;
        self.port.flush()?;

        let response = &self.read_modbus_rtu_response()?;
        let regs = parse_read_holding_response(&response, SLAVE_ID, count)?;
        
        // Convert registers to bytes and then to a string.
        // Each register (u16) is two characters.
        let mut bytes = Vec::with_capacity(regs.len() * 2);
        for reg in regs {
            let b = reg.to_be_bytes();
            bytes.push(b[0]);
            bytes.push(b[1]);
        }
        
        // Strings in Modbus are often null-terminated or padded with nulls/spaces.
        // We'll trim them.
        let s = String::from_utf8_lossy(&bytes).trim_matches('\0').trim().to_string();
        println!("String result: {}", s);
        Ok(s)
    }

    
    /// Read one or more registers from the Modbus device and combine them into type T.
    ///
    /// # Arguments
    ///
    /// * `address` - The starting register address.
    pub fn read_register<T: ModbusRead + Debug>(&mut self, address: u16) -> Result<T> {
        let count = T::REG_COUNT;
        let request = build_read_holding_request(SLAVE_ID, address, count);
        println!("Sending request: {:?}", request);

        // Clear stale bytes, then observe a quiet period before sending.
        let _ = &self.port.clear(serialport::ClearBuffer::All);
        std::thread::sleep(Duration::from_millis(5));

        self.port.write_all(&request)?;
        self.port.flush()?;

        let response = &self.read_modbus_rtu_response()?;
        let regs = parse_read_holding_response(&response, SLAVE_ID, count)?;
        let value = T::from_registers(&regs)?;

        println!("Result: {value:?}");
        
        Ok(value)
    }

    /// Read an RTU response from the serial port.
    ///
    /// This method uses a heuristic to detect the end of the frame.
    fn read_modbus_rtu_response(&mut self) -> Result<Vec<u8>> {
        let start = Instant::now();
        let mut buf = Vec::with_capacity(256);
        let mut temp = [0u8; 64];

        loop {
            match self.port.read(&mut temp) {
                Ok(n) if n > 0 => {
                    buf.extend_from_slice(&temp[..n]);

                    // Simple heuristic: once we have at least 5 bytes and the line goes quiet,
                    // treat that as end-of-frame. For production, tighten this up.
                    std::thread::sleep(Duration::from_millis(5));
                    match self.port.read(&mut temp) {
                        Ok(m) if m > 0 => buf.extend_from_slice(&temp[..m]),
                        _ => break,
                    }
                }
                Ok(_) => {}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
                    if !buf.is_empty() {
                        break;
                    }
                }
                Err(e) => return Err(e.into()),
            }

            if start.elapsed() > OVERALL_TIMEOUT {
                if buf.is_empty() {
                    return Err(anyhow!("timed out waiting for response"));
                }
                break;
            }
        }

        if buf.len() < 5 {
            return Err(anyhow!("response too short: {:02X?}", buf));
        }

        Ok(buf)
    }
}

/// Represents a value read from a Modbus register.
#[derive(Debug, Clone)]
pub enum RegisterValue {
    /// A 16-bit unsigned integer.
    U16(u16),
    /// A 32-bit unsigned integer (occupies two registers).
    U32(u32),
    /// A 32-bit signed integer (occupies two registers).
    I32(i32),
    /// A UTF-8 string.
    String(String),
}

/// Build a Modbus RTU Read Holding Registers request frame.
///
/// # Arguments
///
/// * `slave` - The slave ID.
/// * `start` - The starting register address.
/// * `count` - The number of registers to read.
fn build_read_holding_request(slave: u8, start: u16, count: u16) -> Vec<u8> {
    let mut frame = vec![
        slave,
        FUNCTION_READ_HOLDING,
        (start >> 8) as u8,
        (start & 0xFF) as u8,
        (count >> 8) as u8,
        (count & 0xFF) as u8,
    ];
    let crc = modbus_crc16(&frame);
    frame.push((crc & 0xFF) as u8);       // CRC low byte first
    frame.push((crc >> 8) as u8);         // CRC high byte
    frame
}



/// Parse a Modbus RTU Read Holding Registers response frame.
///
/// # Arguments
///
/// * `frame` - The received RTU frame.
/// * `expected_slave` - The expected slave ID.
/// * `expected_regs` - The expected number of registers.
fn parse_read_holding_response(frame: &[u8], expected_slave: u8, expected_regs: u16) -> Result<Vec<u16>> {
    if frame.len() < 5 {
        return Err(anyhow!("frame too short"));
    }

    let payload_len = frame.len() - 2;
    let crc_recv = u16::from_le_bytes([frame[payload_len], frame[payload_len + 1]]);
    let crc_calc = modbus_crc16(&frame[..payload_len]);
    if crc_recv != crc_calc {
        return Err(anyhow!(
            "CRC mismatch: recv=0x{crc_recv:04X}, calc=0x{crc_calc:04X}"
        ));
    }

    let slave = frame[0];
    let func = frame[1];

    if slave != expected_slave {
        return Err(anyhow!("unexpected slave id {slave}, expected {expected_slave}"));
    }

    // Exception response
    if func & 0x80 != 0 {
        let code = frame.get(2).copied().unwrap_or(0);
        return Err(anyhow!("modbus exception: function=0x{func:02X}, code=0x{code:02X}"));
    }

    if func != FUNCTION_READ_HOLDING {
        return Err(anyhow!("unexpected function 0x{func:02X}"));
    }

    let byte_count = frame[2] as usize;
    let expected_bytes = expected_regs as usize * 2;
    if byte_count != expected_bytes {
        return Err(anyhow!(
            "unexpected byte count {byte_count}, expected {expected_bytes}"
        ));
    }

    if frame.len() != 3 + byte_count + 2 {
        return Err(anyhow!("unexpected frame length {}", frame.len()));
    }

    let mut regs = Vec::with_capacity(expected_regs as usize);
    for chunk in frame[3..3 + byte_count].chunks_exact(2) {
        regs.push(u16::from_be_bytes([chunk[0], chunk[1]]));
    }

    Ok(regs)
}

/// Calculate the Modbus RTU CRC16 checksum.
///
/// # Arguments
///
/// * `data` - The byte slice to calculate the CRC for.
fn modbus_crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;

    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            let lsb = crc & 1;
            crc >>= 1;
            if lsb != 0 {
                crc ^= 0xA001;
            }
        }
    }

    crc
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_u16() {
        // Slave 247 (0xF7), Func 4, 2 bytes (1 reg), Value 0x1234
        let mut frame = vec![0xF7, 0x04, 0x02, 0x12, 0x34];
        let crc = modbus_crc16(&frame);
        frame.push((crc & 0xFF) as u8);
        frame.push((crc >> 8) as u8);

        let regs = parse_read_holding_response(&frame, 247, 1).unwrap();
        assert_eq!(regs, vec![0x1234]);
        let val = u16::from_registers(&regs).unwrap();
        assert_eq!(val, 0x1234);
    }

    #[test]
    fn test_parse_response_u32() {
        // Slave 247, Func 4, 4 bytes (2 regs), Value 0x12345678
        let mut frame = vec![0xF7, 0x04, 0x04, 0x12, 0x34, 0x56, 0x78];
        let crc = modbus_crc16(&frame);
        frame.push((crc & 0xFF) as u8);
        frame.push((crc >> 8) as u8);

        let regs = parse_read_holding_response(&frame, 247, 2).unwrap();
        assert_eq!(regs, vec![0x1234, 0x5678]);
        let val = u32::from_registers(&regs).unwrap();
        assert_eq!(val, 0x12345678);
    }

    #[test]
    fn test_parse_response_i32() {
        // Slave 247, Func 4, 4 bytes (2 regs), Value -1 (0xFFFFFFFF)
        let mut frame = vec![0xF7, 0x04, 0x04, 0xFF, 0xFF, 0xFF, 0xFF];
        let crc = modbus_crc16(&frame);
        frame.push((crc & 0xFF) as u8);
        frame.push((crc >> 8) as u8);

        let regs = parse_read_holding_response(&frame, 247, 2).unwrap();
        assert_eq!(regs, vec![0xFFFF, 0xFFFF]);
        let val = i32::from_registers(&regs).unwrap();
        assert_eq!(val, -1);
    }

    #[test]
    fn test_string_conversion() {
        let regs = vec![
            u16::from_be_bytes([b'H', b'e']),
            u16::from_be_bytes([b'l', b'l']),
            u16::from_be_bytes([b'o', 0]),
        ];
        
        let mut bytes = Vec::new();
        for reg in regs {
            let b = reg.to_be_bytes();
            bytes.push(b[0]);
            bytes.push(b[1]);
        }
        let s = String::from_utf8_lossy(&bytes).trim_matches('\0').trim().to_string();
        assert_eq!(s, "Hello");
    }
}
