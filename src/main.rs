use anyhow::{anyhow, Context, Result};
use serialport::{DataBits, FlowControl, Parity, SerialPort, StopBits};
use std::io::Write;
use std::time::{Duration, Instant};

const PORT: &str = "/dev/ttyACM0";

// Replace these with your actual inverter settings.
const BAUD: u32 = 9600;
const SLAVE_ID: u8 = 247;

// Example: read 2 holding registers starting at address 0x0000.
// You will need the correct FoxESS register addresses for your model.
const FUNCTION_READ_HOLDING: u8 = 0x04;
const START_ADDR: u16 = 30016;
const REG_COUNT: u16 = 2;

fn main() -> Result<()> {
    let mut port = serialport::new(PORT, BAUD)
        .data_bits(DataBits::Eight)
        .parity(Parity::None) // or Even, depending on your inverter config
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .timeout(Duration::from_millis(300))
        .open()
        .with_context(|| format!("failed to open {PORT}"))?;

    let request = build_read_holding_request(SLAVE_ID, START_ADDR, REG_COUNT);
    println!("Sending request: {:?}", request);

    // Clear stale bytes, then observe a quiet period before sending.
    let _ = port.clear(serialport::ClearBuffer::All);
    std::thread::sleep(Duration::from_millis(5));

    port.write_all(&request)?;
    port.flush()?;

    let response = read_modbus_rtu_response(&mut *port, Duration::from_millis(1000))?;
    let values = parse_read_holding_response(&response, SLAVE_ID, REG_COUNT)?;

    println!("Registers: {values:?}");
    Ok(())
}

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

fn read_modbus_rtu_response(
    port: &mut dyn SerialPort,
    overall_timeout: Duration,
) -> Result<Vec<u8>> {
    let start = Instant::now();
    let mut buf = Vec::with_capacity(256);
    let mut temp = [0u8; 64];

    loop {
        match port.read(&mut temp) {
            Ok(n) if n > 0 => {
                buf.extend_from_slice(&temp[..n]);

                // Simple heuristic: once we have at least 5 bytes and the line goes quiet,
                // treat that as end-of-frame. For production, tighten this up.
                std::thread::sleep(Duration::from_millis(5));
                match port.read(&mut temp) {
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

        if start.elapsed() > overall_timeout {
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

