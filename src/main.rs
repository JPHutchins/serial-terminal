use const_format::concatcp;
use crossterm::style::Print;
use futures::stream::StreamExt;
use futures::{future::FutureExt, select};
use std::format;
use std::io::{
    self, stdout,
    ErrorKind::{PermissionDenied, TimedOut},
    Write,
};
use std::time::SystemTime;

use clap::{error::ContextKind::InvalidArg, error::ContextValue, Parser};
use crossterm::{
    cursor::{Hide, MoveLeft, Show},
    event::{Event, EventStream, KeyCode, KeyModifiers},
    execute, queue,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use tokio::io::AsyncReadExt;
use tokio_serial::{DataBits, FlowControl, Parity, StopBits};

use terminal_spinner_data::{SpinnerData, DOTS12};

mod arg_helpers;
mod constants;
mod list_ports;
mod serial_connection;

use crate::arg_helpers::{
    valid_baud, valid_data_bits, valid_flow_control, valid_parity, valid_stop_bits, CLIDisplay,
};
use crate::constants::{ABOUT, HELP, LONG_VERSION};
use crate::list_ports::list_ports;

#[derive(Parser, Debug)]
#[command(author, version, long_version = LONG_VERSION, about = ABOUT, long_about = concatcp!(ABOUT, "\n\n", HELP))]
pub struct Args {
    #[arg(help = "Serial port, e.g. 'COM1' or '/dev/ttyUSB0'. Use '?' to list")]
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
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            // if the <PORT> argument is omitted, list ports but still exit with error
            let invalid_arg = error.get(InvalidArg);
            if let Some(ContextValue::Strings(invalid_arg)) = invalid_arg {
                for v in invalid_arg {
                    if v == "<PORT>" {
                        list_ports();
                    }
                }
            }
            Args::parse()
        }
    };

    enable_raw_mode().unwrap();

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(io_tasks(args));

    disable_raw_mode().unwrap();
}

async fn io_tasks(args: Args) {
    let mut serial_conn = match serial_connection::get_serial_connection(&args) {
        Some(serial_conn) => serial_conn,
        None => {
            if args.port != "?" {
                println!(
                    "Could not open port '{}', searching for serial ports...",
                    args.port
                );
            }
            list_ports();
            return;
        }
    };

    let mut reader = EventStream::new();

    let mut rx_buf: [u8; 1] = [0; 1];

    let mut conn_retries = 0;

    let mut stdout = stdout();

    const ANIMATION: SpinnerData = DOTS12;
    let mut last_animation_time = SystemTime::now();
    let mut previous_frame_size = ANIMATION.frames[0].chars().count();

    loop {
        let mut keypress_event = reader.next().fuse();
        let mut serial_rx_event = Box::pin(serial_conn.read_exact(&mut rx_buf).fuse());

        select! {
            event = keypress_event => {
                match event {
                    Some(Ok(event)) => {
                        match handle_keypress_event(event) {
                            KeyboardInputAction::Menu => {println!("TODO: menu"); break},
                            KeyboardInputAction::OneByteCode(byte) => {
                                serial_conn.write(&byte).unwrap();
                            }
                            KeyboardInputAction::TwoByteCode(bytes) => {
                                serial_conn.write(&bytes).unwrap();
                            }
                            KeyboardInputAction::NoAction => continue,
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
                    Err(error) => {
                        match error.kind() {
                            PermissionDenied | TimedOut => {
                                serial_conn = match serial_connection::get_serial_connection(&args) {
                                    Some(serial_conn) => {
                                        conn_retries = 0;
                                        execute!(
                                            stdout,
                                            MoveLeft(previous_frame_size.try_into().unwrap()),
                                            Print("  "),
                                            Print(format!("\r\nReconnected to {}\r\n", args.port)),
                                            Show
                                        ).unwrap();
                                        serial_conn
                                    },
                                    None => {
                                        if conn_retries == 0 {
                                            queue!(
                                                stdout,
                                                Hide,
                                                Print(format!("Lost connection to {}  ", args.port)),
                                                Print(ANIMATION.frames[0]),
                                            ).unwrap();
                                            conn_retries += 1;
                                        } else if last_animation_time.elapsed().unwrap().as_millis() >= ANIMATION.interval.into() {
                                            let frame = ANIMATION.frames[conn_retries % ANIMATION.frames.len()];
                                            queue!(
                                                stdout,
                                                MoveLeft(previous_frame_size.try_into().unwrap()),
                                                Print(frame),
                                            ).unwrap();
                                            previous_frame_size = frame.chars().count();
                                            conn_retries += 1;
                                            last_animation_time = SystemTime::now();
                                        }
                                        io::stdout().flush().unwrap();
                                        serial_conn
                                    }
                                };
                            },
                            _ => println!("Serial RX Error: {:?}\r", error)
                        }
                    }
                }
            }
        };
    }
}

enum KeyboardInputAction {
    Menu,
    NoAction,
    OneByteCode([u8; 1]),
    TwoByteCode([u8; 2]),
}

fn handle_keypress_event(event: Event) -> KeyboardInputAction {
    match event {
        Event::Key(key) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if let KeyCode::Char(code) = key.code {
                    if code == 'c' {
                        return KeyboardInputAction::OneByteCode([0x03]);
                    }
                    if code == 't' {
                        return KeyboardInputAction::Menu;
                    }
                }
                return KeyboardInputAction::NoAction; // don't handle other Ctrl-* for now
            }
            match key.code {
                KeyCode::Char(code) => KeyboardInputAction::OneByteCode([code as u8]),
                KeyCode::Enter => KeyboardInputAction::OneByteCode([b'\r']),
                KeyCode::Esc => KeyboardInputAction::OneByteCode([0x1B]),
                KeyCode::Up => KeyboardInputAction::TwoByteCode([0x1B, b'A']),
                KeyCode::Down => KeyboardInputAction::TwoByteCode([0x1B, b'B']),
                KeyCode::Left => KeyboardInputAction::TwoByteCode([0x1B, b'C']),
                KeyCode::Right => KeyboardInputAction::TwoByteCode([0x1B, b'D']),
                KeyCode::Tab => KeyboardInputAction::OneByteCode([b'\t']),
                KeyCode::Backspace => KeyboardInputAction::OneByteCode([0x08]),
                KeyCode::Delete => KeyboardInputAction::OneByteCode([0x7F]),
                KeyCode::Null => KeyboardInputAction::OneByteCode([0x00]),
                _ => KeyboardInputAction::NoAction,
            }
        }
        Event::FocusGained
        | Event::FocusLost
        | Event::Mouse(_)
        | Event::Resize(_, _)
        | Event::Paste(_) => KeyboardInputAction::NoAction,
    }
}
