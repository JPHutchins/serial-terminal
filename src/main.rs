use const_format::concatcp;
use futures::stream::StreamExt;
use futures::{future::FutureExt, select};
use std::io::{self, Write};

use clap::Parser;
use crossterm::{
    event::{Event, EventStream, KeyCode, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use tokio::io::AsyncReadExt;
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};

mod arg_helpers;
mod constants;
mod serial_connection;

use crate::arg_helpers::{
    valid_baud, valid_data_bits, valid_flow_control, valid_parity, valid_stop_bits, CLIDisplay,
};
use crate::constants::{ABOUT, HELP, LONG_VERSION};

#[derive(Parser, Debug)]
#[command(author, version, long_version = LONG_VERSION, about = ABOUT, long_about = concatcp!(ABOUT, "\n\n", HELP))]
pub struct Args {
    #[arg(help = "Path to the serial port, e.g. 'COM1' or '/dev/ttyUSB0'")]
    port: String,

    #[arg(short, long, default_value_t = 115_200, value_parser = valid_baud)]
    baud: u32,

    #[arg(
        short,
        long, default_value_t = CLIDisplay { name: String::from("8"), value: DataBits::Eight},
        value_parser = valid_data_bits,
        help = "5, 6, 7, or 8"
    )]
    data_bits: CLIDisplay<DataBits>,

    #[arg(
        short,
        long,
        default_value_t = CLIDisplay { name: String::from("none"), value: FlowControl::None},
        value_parser = valid_flow_control,
        help = "none, sw, or hw"
    )]
    flow_control: CLIDisplay<FlowControl>,

    #[arg(
        short,
        long,
        default_value_t = CLIDisplay { name: String::from("none"), value: Parity::None},
        value_parser = valid_parity,
        help = "none, odd, or even"
    )]
    parity: CLIDisplay<Parity>,

    #[arg(
        short,
        long, default_value_t = CLIDisplay { name: String::from("1"), value: StopBits::One},
        value_parser = valid_stop_bits,
        help = "1 or 2"
    )]
    stop_bits: CLIDisplay<StopBits>,
}

fn main() {
    let args = Args::parse();

    enable_raw_mode().unwrap();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(io_tasks(args));

    disable_raw_mode().unwrap();
}

async fn io_tasks(args: Args) {
    let mut reader = EventStream::new();

    let mut serial_connection = serial_connection::get_serial_connection(args);

    let mut rx_buf: [u8; 1] = [0; 1];
    let mut display_buf: [u8; 1] = [0; 1];

    loop {
        let mut keypress_event = reader.next().fuse();
        let mut serial_rx_event = Box::pin(serial_connection.read_exact(&mut rx_buf).fuse());

        select! {
            event = keypress_event => {
                match event {
                    Some(Ok(event)) => {
                        match event {
                            Event::Key(key) => {
                                if key.modifiers.contains(KeyModifiers::CONTROL) {
                                    if let KeyCode::Char(code) = key.code {
                                        if code == 'c' {
                                            break;
                                        }
                                    }
                                }

                                if let KeyCode::Char(code) = key.code {
                                    display_buf[0] = code as u8;
                                    serial_connection.write(&display_buf).unwrap();
                                }
                            }
                            _ => continue

                        }

                        if event == Event::Key(KeyCode::Esc.into()) {
                            break;
                        }
                    }
                    Some(Err(e)) => println!("Error: {:?}\r", e),
                    None => break,
                }
            }
            event = serial_rx_event => {
                match event {
                    Ok(_) => {
                        print!("{}", rx_buf[0] as char);
                        io::stdout().flush().unwrap();
                    }
                    Err(e) => println!("Error: {:?}\r", e)
                }
            }
        };
    }
}
