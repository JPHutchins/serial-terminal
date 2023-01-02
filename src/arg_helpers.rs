use std::fmt::{Display, Formatter, Result};
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};

#[derive(Debug, Clone)]
pub struct CLIDisplay<T> {
    pub name: String,
    pub value: T,
}

impl<T> Display for CLIDisplay<T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.name)
    }
}

pub fn valid_baud(val: &str) -> std::result::Result<u32, String> {
    match val.parse::<u32>() {
        Ok(baud) => Ok(baud),
        Err(_) => Err(format!(
            "Invalid baud rate '{}' specified, expected valid u32",
            val
        )),
    }
}

pub fn valid_data_bits(val: &str) -> std::result::Result<CLIDisplay<DataBits>, String> {
    match val {
        "5" => Ok(CLIDisplay {
            name: String::from(val),
            value: DataBits::Five,
        }),
        "6" => Ok(CLIDisplay {
            name: String::from(val),
            value: DataBits::Six,
        }),
        "7" => Ok(CLIDisplay {
            name: String::from(val),
            value: DataBits::Seven,
        }),
        "8" => Ok(CLIDisplay {
            name: String::from(val),
            value: DataBits::Eight,
        }),
        _ => Err(format!(
            "Invalid data bits '{}' specified, expected 5 <= bits <= 8",
            val
        )),
    }
}

pub fn valid_flow_control(val: &str) -> std::result::Result<CLIDisplay<FlowControl>, String> {
    match val.to_lowercase().as_str() {
        "none" => Ok(CLIDisplay {
            name: String::from(val),
            value: FlowControl::None,
        }),
        "hardware" | "hw" => Ok(CLIDisplay {
            name: String::from(val),
            value: FlowControl::Hardware,
        }),
        "software" | "sw" => Ok(CLIDisplay {
            name: String::from(val),
            value: FlowControl::Software,
        }),
        _ => Err(format!(
            "Invalid flow control '{}' specified, expected 'none', 'hw', or 'sw'",
            val
        )),
    }
}

pub fn valid_parity(val: &str) -> std::result::Result<CLIDisplay<Parity>, String> {
    match val.to_lowercase().as_str() {
        "none" => Ok(CLIDisplay {
            name: String::from(val),
            value: Parity::None,
        }),
        "odd" => Ok(CLIDisplay {
            name: String::from(val),
            value: Parity::Odd,
        }),
        "even" => Ok(CLIDisplay {
            name: String::from(val),
            value: Parity::Even,
        }),
        _ => Err(format!(
            "Invalid parity '{}' specified, expected 'none', 'odd', or 'even'",
            val
        )),
    }
}

pub fn valid_stop_bits(val: &str) -> std::result::Result<CLIDisplay<StopBits>, String> {
    match val.to_lowercase().as_str() {
        "1" | "one" => Ok(CLIDisplay {
            name: String::from(val),
            value: StopBits::One,
        }),
        "2" | "two" => Ok(CLIDisplay {
            name: String::from(val),
            value: StopBits::Two,
        }),
        _ => Err(format!(
            "Invalid baud rate '{}' specified, expected 1 or 2",
            val
        )),
    }
}
