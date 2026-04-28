//! Mock Modbus serial port
//!
//! Provides an in-memory mock implementation of a serial port for testing Modbus communication.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::time::Duration;

use serialport::{
    ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits,
};

const MOCK_BAUD: u32 = 9600;
const MOCK_DATA_BITS: DataBits = DataBits::Eight;
const MOCK_PARITY: Parity = Parity::None;
const MOCK_STOP_BITS: StopBits = StopBits::One;
const MOCK_FLOW_CONTROL: FlowControl = FlowControl::None;
const MOCK_TIMEOUT: Duration = Duration::from_millis(300);

const MOCK_SLAVE_ID: u8 = 247;
const MOCK_FUNCTION_READ_HOLDING: u8 = 0x04;

/// Mock implementation of a Modbus RTU serial port
///
#[derive(Debug, Clone)]
pub struct MockSerialPort {
    registers: HashMap<u16, u16>,
    pending_response: Vec<u8>,
    timeout: Duration,
    baud_rate: u32,
    data_bits: DataBits,
    flow_control: FlowControl,
    parity: Parity,
    stop_bits: StopBits,
}

impl Default for MockSerialPort {
    fn default() -> Self {
        Self::new()
    }
}

impl MockSerialPort {
    /// Create a new mock serial port with default values
    ///
    pub fn new() -> Self {
        let mut registers = HashMap::new();

        // Example values. Replace/add addresses that are meaningful in your app.
        registers.insert(0, 1234);
        registers.insert(1, 0x1234);
        registers.insert(2, 0x5678);
        registers.insert(10, 42);
        registers.insert(100, 2300);
        registers.insert(101, 500);
        registers.insert(102, 1200);

        let mut msp = Self {
            registers,
            pending_response: Vec::new(),
            timeout: MOCK_TIMEOUT,
            baud_rate: MOCK_BAUD,
            data_bits: MOCK_DATA_BITS,
            flow_control: MOCK_FLOW_CONTROL,
            parity: MOCK_PARITY,
            stop_bits: MOCK_STOP_BITS,
        };

        msp.set_i32(32000, 20000); // pv_energy_total
        msp.set_i32(32009, 10000); // feed_in_energy_total
        msp.set_i32(32021, 30000); // load_energy_total
        msp
    }

    /// Create a mock serial port from explicit register values
    ///
    /// # Arguments
    ///
    /// * `registers` - an iterator of address/value pairs
    pub fn with_registers(registers: impl IntoIterator<Item = (u16, u16)>) -> Self {
        Self {
            registers: registers.into_iter().collect(),
            ..Self::new()
        }
    }

    /// Set or replace a single 16-bit register value
    ///
    /// # Arguments
    ///
    /// * `address` - the register address
    /// * `value` - the 16-bit value to store
    pub fn set_register(&mut self, address: u16, value: u16) {
        self.registers.insert(address, value);
    }

    /// Store a `u32` across two consecutive registers, high word first
    ///
    /// # Arguments
    ///
    /// * `address` - the starting register address
    /// * `value` - the 32-bit value to store
    pub fn set_u32(&mut self, address: u16, value: u32) {
        self.registers.insert(address, (value >> 16) as u16);
        self.registers.insert(address + 1, (value & 0xFFFF) as u16);
    }

    /// Store an `i32` across two consecutive registers, high word first
    ///
    /// # Arguments
    ///
    /// * `address` - the starting register address
    /// * `value` - the 32-bit value to store
    pub fn set_i32(&mut self, address: u16, value: i32) {
        self.set_u32(address, value as u32);
    }

    /// Store a string as big-endian `u16` Modbus registers
    ///
    /// # Arguments
    ///
    /// * `address` - the starting register address
    /// * `value` - the string value to store
    /// * `register_count` - the number of registers to use
    pub fn set_string(&mut self, address: u16, value: &str, register_count: u16) {
        let mut bytes = value.as_bytes().to_vec();
        bytes.resize(register_count as usize * 2, 0);

        for index in 0..register_count {
            let byte_index = index as usize * 2;
            let reg = u16::from_be_bytes([bytes[byte_index], bytes[byte_index + 1]]);
            self.registers.insert(address + index, reg);
        }
    }

    /// Handle a Modbus request and prepare a response
    ///
    /// # Arguments
    ///
    /// * `request` - the received request frame
    fn handle_request(&mut self, request: &[u8]) {
        self.pending_response.clear();

        if request.len() < 8 {
            self.pending_response = Self::exception_response(
                MOCK_SLAVE_ID,
                MOCK_FUNCTION_READ_HOLDING,
                0x03,
            );
            return;
        }

        let payload_len = request.len() - 2;
        let received_crc = u16::from_le_bytes([request[payload_len], request[payload_len + 1]]);
        let calculated_crc = modbus_crc16(&request[..payload_len]);

        if received_crc != calculated_crc {
            self.pending_response = Self::exception_response(
                MOCK_SLAVE_ID,
                MOCK_FUNCTION_READ_HOLDING,
                0x03,
            );
            return;
        }

        let slave = request[0];
        let function = request[1];
        let start = u16::from_be_bytes([request[2], request[3]]);
        let count = u16::from_be_bytes([request[4], request[5]]);

        if slave != MOCK_SLAVE_ID {
            // Real Modbus slaves usually stay silent when addressed with
            // a different slave ID.
            return;
        }

        if function != MOCK_FUNCTION_READ_HOLDING {
            self.pending_response = Self::exception_response(slave, function, 0x01);
            return;
        }

        if count == 0 || count > 125 {
            self.pending_response = Self::exception_response(slave, function, 0x03);
            return;
        }

        let mut response = Vec::with_capacity(3 + count as usize * 2 + 2);
        response.push(slave);
        response.push(function);
        response.push((count * 2) as u8);

        for offset in 0..count {
            let value = self.registers.get(&(start + offset)).copied().unwrap_or(0);
            response.extend_from_slice(&value.to_be_bytes());
        }

        let crc = modbus_crc16(&response);
        response.push((crc & 0xFF) as u8);
        response.push((crc >> 8) as u8);

        self.pending_response = response;
    }

    /// Create a Modbus exception response
    ///
    /// # Arguments
    ///
    /// * `slave` - the slave ID
    /// * `function` - the function code
    /// * `code` - the exception code
    fn exception_response(slave: u8, function: u8, code: u8) -> Vec<u8> {
        let mut response = vec![slave, function | 0x80, code];
        let crc = modbus_crc16(&response);
        response.push((crc & 0xFF) as u8);
        response.push((crc >> 8) as u8);
        response
    }
}

impl Read for MockSerialPort {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pending_response.is_empty() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "mock timeout"));
        }

        let len = buf.len().min(self.pending_response.len());
        buf[..len].copy_from_slice(&self.pending_response[..len]);
        self.pending_response.drain(..len);

        Ok(len)
    }
}

impl Write for MockSerialPort {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.handle_request(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SerialPort for MockSerialPort {
    fn name(&self) -> Option<String> {
        Some("mock-modbus-rtu".to_string())
    }

    fn baud_rate(&self) -> serialport::Result<u32> {
        Ok(self.baud_rate)
    }

    fn data_bits(&self) -> serialport::Result<DataBits> {
        Ok(self.data_bits)
    }

    fn flow_control(&self) -> serialport::Result<FlowControl> {
        Ok(self.flow_control)
    }

    fn parity(&self) -> serialport::Result<Parity> {
        Ok(self.parity)
    }

    fn stop_bits(&self) -> serialport::Result<StopBits> {
        Ok(self.stop_bits)
    }

    fn timeout(&self) -> Duration {
        self.timeout
    }

    fn set_baud_rate(&mut self, baud_rate: u32) -> serialport::Result<()> {
        self.baud_rate = baud_rate;
        Ok(())
    }

    fn set_data_bits(&mut self, data_bits: DataBits) -> serialport::Result<()> {
        self.data_bits = data_bits;
        Ok(())
    }

    fn set_flow_control(&mut self, flow_control: FlowControl) -> serialport::Result<()> {
        self.flow_control = flow_control;
        Ok(())
    }

    fn set_parity(&mut self, parity: Parity) -> serialport::Result<()> {
        self.parity = parity;
        Ok(())
    }

    fn set_stop_bits(&mut self, stop_bits: StopBits) -> serialport::Result<()> {
        self.stop_bits = stop_bits;
        Ok(())
    }

    fn set_timeout(&mut self, timeout: Duration) -> serialport::Result<()> {
        self.timeout = timeout;
        Ok(())
    }

    fn write_request_to_send(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }

    fn write_data_terminal_ready(&mut self, _level: bool) -> serialport::Result<()> {
        Ok(())
    }

    fn read_clear_to_send(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }

    fn read_data_set_ready(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }

    fn read_ring_indicator(&mut self) -> serialport::Result<bool> {
        Ok(false)
    }

    fn read_carrier_detect(&mut self) -> serialport::Result<bool> {
        Ok(true)
    }

    fn bytes_to_read(&self) -> serialport::Result<u32> {
        Ok(self.pending_response.len() as u32)
    }

    fn bytes_to_write(&self) -> serialport::Result<u32> {
        Ok(0)
    }

    fn clear(&self, _buffer_to_clear: ClearBuffer) -> serialport::Result<()> {
        Ok(())
    }

    fn try_clone(&self) -> serialport::Result<Box<dyn SerialPort>> {
        Ok(Box::new(self.clone()))
    }

    fn set_break(&self) -> serialport::Result<()> {
        Ok(())
    }

    fn clear_break(&self) -> serialport::Result<()> {
        Ok(())
    }
}

/// Create a boxed mock serial port with default registers
///
pub fn boxed_mock_port() -> Box<dyn SerialPort> {
    Box::new(MockSerialPort::new())
}

/// Create a boxed mock serial port with explicit registers
///
/// # Arguments
///
/// * `registers` - an iterator of address/value pairs
pub fn boxed_mock_port_with_registers(
    registers: impl IntoIterator<Item = (u16, u16)>,
) -> Box<dyn SerialPort> {
    Box::new(MockSerialPort::with_registers(registers))
}

/// Calculate the Modbus RTU CRC16 checksum
///
/// # Arguments
///
/// * `data` - the byte slice to calculate the CRC for
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
