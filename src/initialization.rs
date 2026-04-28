use std::{env, fs};
use log::LevelFilter;
use crate::logging::setup_logger;
use anyhow::{anyhow, Context, Result};
use crate::manager_modbus::ModbusPortMode;

/// Configuration parameters for the web server
///
pub struct WebServerParameters {
    /// the address for the web server to bind to
    pub bind_address: String,
    /// the port for the web server to bind to
    pub bind_port: u16,
}

/// Configuration parameters for the modbus client
/// 
pub struct ModbusParameters {
    /// the serial port to connect to
    pub serial_port: String,
    /// whether the client will connect to the modbus server or not
    pub mock: ModbusPortMode,
}

/// General configuration parameters for the application
///
pub struct General {
    /// the path to the log file
    pub log_path: String,
    /// the logging level (Off, Error, Warn, Info, Debug, Trace)
    pub log_level: LevelFilter,
    /// if true, logging is also written to stdout
    pub log_to_stdout: bool,
}

/// The overall configuration for the application
///
pub struct Config {
    pub web_server: WebServerParameters,
    pub modbus: ModbusParameters,
    pub general: General,
}

/// Struct used during the parsing of the configuration file
///
/// It holds optional values for all configuration items
struct PartialConfig {
    web_server_bind_address: Option<String>,
    web_server_bind_port: Option<u16>,
    modbus_serial_port: Option<String>,
    modbus_mock: Option<ModbusPortMode>,
    general_log_path: Option<String>,
    general_log_level: Option<LevelFilter>,
    general_log_to_stdout: Option<bool>,
}

impl PartialConfig {
    /// Creates a new PartialConfig instance with all values set to None
    ///
    fn new() -> Self {
        Self {
            web_server_bind_address: None,
            web_server_bind_port: None,
            modbus_serial_port: None,
            modbus_mock: None,
            general_log_path: None,
            general_log_level: None,
            general_log_to_stdout: None,
        }
    }

    /// Builds a Config struct from the PartialConfig instance
    ///
    /// Returns an error if any of the required configuration items are missing
    fn build(self) -> Result<Config> {
        Ok(Config {
            web_server: WebServerParameters {
                bind_address: Self::require(self.web_server_bind_address, "web_server.bind_address")?,
                bind_port: Self::require(self.web_server_bind_port, "web_server.bind_port")?,
            },
            modbus: ModbusParameters {
                serial_port: Self::require(self.modbus_serial_port, "modbus.serial_port")?,
                mock: Self::require(self.modbus_mock, "modbus.mock")?,
            },
            general: General {
                log_path: Self::require(self.general_log_path, "general.log_path")?,
                log_level: Self::require(self.general_log_level, "general.log_level")?,
                log_to_stdout: Self::require(self.general_log_to_stdout, "general.log_to_stdout")?,
            },
        })
    }

    /// Helper function to require an optional value and return an error if it's None
    ///
    /// # Arguments
    ///
    /// * 'value' - the optional value to check
    /// * 'key' - the configuration key associated with the value
    fn require<T>(value: Option<T>, key: &str) -> Result<T> {
        value.ok_or_else(|| anyhow!("missing config key: {}", key))
    }
}

/// Returns a configuration struct for the application and starts logging
///
pub fn config() -> Result<Config> {
    let args: Vec<String> = env::args().collect();
    let config_path = args.iter()
        .find(|p| p.starts_with("--config="))
        .ok_or(anyhow!("missing --config=<config_path>"))?;
    let config_path = config_path
        .split_once('=')
        .ok_or(anyhow!("invalid --config=<config_path>"))?
        .1;

    let config = load_config(config_path)?;

    setup_logger(
        &config.general.log_path,
        config.general.log_level,
        config.general.log_to_stdout,
    )?;

    Ok(config)
}

/// Loads the configuration file and returns a struct with all configuration items
///
/// # Arguments
///
/// * 'config_path' - path to the configuration file
fn load_config(config_path: &str) -> Result<Config> {
    let text = fs::read_to_string(config_path)
        .with_context(|| format!("failed to read config file: {}", config_path))?;
    parse_config(&text)
}

/// Parses the configuration text and returns a Config struct
///
/// # Arguments
///
/// * 'text' - the configuration text to parse
fn parse_config(text: &str) -> Result<Config> {
    let mut partial = PartialConfig::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| anyhow!("line {}: expected key=value", line_number))?;

        let key = key.trim();
        let value = value.trim();

        match key {
            "web_server.bind_address" => {
                partial.web_server_bind_address = Some(value.to_string());
            }
            "web_server.bind_port" => {
                partial.web_server_bind_port = Some(parse_value(value, key, line_number)?);
            }
            "modbus.serial_port" => {
                partial.modbus_serial_port = Some(value.to_string());
            }
            "modbus.mock" => {
                partial.modbus_mock = Some(port_mode(parse_value(value, key, line_number)?));
            }
            "general.log_path" => {
                partial.general_log_path = Some(value.to_string());
            }
            "general.log_level" => {
                partial.general_log_level = Some(parse_log_level(value, line_number)?);
            }
            "general.log_to_stdout" => {
                partial.general_log_to_stdout = Some(parse_value(value, key, line_number)?);
            }
            _ => {
                return Err(anyhow!(
                    "line {}: unknown config key: {}",
                    line_number, key
                ));
            }
        }
    }

    partial.build()
}

/// Helper function to parse a value from a string
///
/// # Arguments
///
/// * 'value' - the string value to parse
/// * 'key' - the configuration key associated with the value
/// * 'line_number' - the line number in the configuration file
fn parse_value<T>(value: &str, key: &str, line_number: usize) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value.parse::<T>().map_err(|e| {
        anyhow!(
            "line {}: invalid value for {}: {}",
            line_number, key, e
        )
    })
}

/// Helper function to parse a log level from a string
///
/// # Arguments
///
/// * 'value' - the string value to parse
/// * 'line_number' - the line number in the configuration file
fn parse_log_level(value: &str, line_number: usize) -> Result<LevelFilter> {
    match value {
        "Off" => Ok(LevelFilter::Off),
        "Error" => Ok(LevelFilter::Error),
        "Warn" => Ok(LevelFilter::Warn),
        "Info" => Ok(LevelFilter::Info),
        "Debug" => Ok(LevelFilter::Debug),
        "Trace" => Ok(LevelFilter::Trace),
        _ => Err(anyhow!(
            "line {}: invalid value for general.log_level: {}",
            line_number, value
        )),
    }
}

/// Translate a boolean value to ModbusPortMode
///
/// # Arguments
///
/// * 'mock' - the boolean value to translate
fn port_mode(mock: bool) -> ModbusPortMode {
    if mock {
        ModbusPortMode::Mock
    } else {
        ModbusPortMode::Real
    }
}