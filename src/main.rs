use const_format::concatcp;
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use std::{
    format,
    io::{
        self, stdout, Error, ErrorKind,
        ErrorKind::{PermissionDenied, TimedOut, WouldBlock},
        Write,
    },
    time::Duration,
};

use chrono;
use clap::{error::ContextKind::InvalidArg, error::ContextValue, Parser};
use crossterm::{
    cursor::{Hide, MoveLeft, MoveUp, Show},
    event::{Event, EventStream, KeyCode, KeyModifiers},
    execute, queue,
    style::{
        Color::{DarkGrey, Reset, White},
        Print, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{disable_raw_mode, enable_raw_mode},
};
use terminal_spinner_data::{SpinnerData, DOTS12};
use tokio::io::AsyncReadExt;
use tokio_serial::{DataBits, FlowControl, Parity, SerialStream, StopBits};

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

    if args.port == "?" {
        list_ports();
        return;
    }

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
    let mut rx_buf: [u8; 1] = [0; 1];

    let connect_event_fut = wait_for_serial_port(&args, None).fuse();
    pin_mut!(connect_event_fut);

    'connection: loop {
        let keypress_event = reader.next().fuse();
        pin_mut!(keypress_event);

        let mut serial_conn: SerialStream;

        select! {
            event = keypress_event => {
                match handle_keypress_event(&event) {
                    KeyboardInputAction::Menu => break,
                    KeyboardInputAction::NoAction | KeyboardInputAction::Chars(_) => continue 'connection,
                    KeyboardInputAction::KeypressError => {log_to_ui!("Keypress error"); break}
                };
            },
            event = connect_event_fut => {
                serial_conn = event
            },
        }

        'communication: loop {
            let keypress_event = reader.next().fuse();
            let serial_rx_event = serial_conn.read_exact(&mut rx_buf).fuse();
            pin_mut!(keypress_event, serial_rx_event);

            select! {
                event = keypress_event => {
                    match handle_keypress_event(&event) {
                        KeyboardInputAction::Chars(bytes) => match serial_conn.write(&bytes) {
                            Ok(_) => {},
                            Err(error) => match error.kind() {
                                WouldBlock => {},
                                _ => log_to_ui!("Serial TX Error: {:?}", error)
                            }
                        }
                        KeyboardInputAction::KeypressError => {log_to_ui!("Keypress error"); break 'connection}
                        KeyboardInputAction::Menu => break 'connection,
                        KeyboardInputAction::NoAction => continue 'communication,
                    };
                },
                event = serial_rx_event => {
                    match event {
                        Ok(_) => {
                            print!("{}", rx_buf[0] as char);
                            io::stdout().flush().unwrap();
                        }
                        Err(error) => {
                            match error.kind() {
                                PermissionDenied | TimedOut => {
                                    connect_event_fut.set(wait_for_serial_port(&args, Some(error.kind())).fuse());
                                    break 'communication
                                },
                                _ => log_to_ui!("Serial RX Error: {:?}", error)
                            }
                        }
                    }
                },
            };
        }
    }
}

async fn wait_for_serial_port(args: &Args, error_kind: Option<ErrorKind>) -> SerialStream {
    const ANIMATION: SpinnerData = DOTS12;

    let mut is_first_retry = true;
    let mut stdout = stdout();
    let mut frame_iter = ANIMATION.frames.into_iter();
    let mut frame = frame_iter.next().unwrap();
    let mut previous_frame_size = frame.chars().count();

    loop {
        match serial_connection::get_serial_connection(&args) {
            Some(serial_conn) => {
                execute!(
                    // clear the animation and move up so log is on next line
                    stdout,
                    MoveLeft(previous_frame_size.try_into().unwrap()),
                    Print("     "),
                    MoveUp(1),
                )
                .unwrap();
                log_to_ui!("Connected to {}", args.port);
                execute!(stdout, Show).unwrap();
                break serial_conn;
            }
            None => {
                if is_first_retry {
                    match error_kind {
                        Some(error_kind) => {
                            log_to_ui!("{} error '{}', waiting", args.port, error_kind)
                        }
                        None => log_to_ui!("Waiting for {}", args.port),
                    };
                    queue!(stdout, Print(frame), Hide).unwrap();
                    is_first_retry = false;
                } else {
                    // let frame = ANIMATION.frames[conn_retries % ANIMATION.frames.len()];
                    frame = match frame_iter.next() {
                        Some(frame) => frame,
                        None => {
                            // restart the iterator
                            frame_iter = ANIMATION.frames.into_iter();
                            frame_iter.next().unwrap()
                        }
                    };
                    queue!(
                        stdout,
                        MoveLeft(previous_frame_size.try_into().unwrap()),
                        Print(frame),
                    )
                    .unwrap();
                    previous_frame_size = frame.chars().count();
                }
                io::stdout().flush().unwrap();
            }
        };
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
}

#[macro_export]
macro_rules! log_to_ui {
    ( $ ( $arg:tt ) * ) => {{
        let msg = format!($($arg)*);
        execute!(
            stdout(),
            Hide,
            SetBackgroundColor(DarkGrey),
            SetForegroundColor(White),
            Print(format!(
                "\r\n[{}] ",
                chrono::offset::Local::now().format("%H:%M:%S%.3f")
            )),
            Print(msg),
            SetBackgroundColor(Reset),
            SetForegroundColor(Reset),
            Print("\r\n"),
            Show,
        )
        .unwrap();
    }}
}

enum KeyboardInputAction {
    Chars(Vec<u8>),
    KeypressError,
    Menu,
    NoAction,
}

fn handle_keypress_event(event: &Option<Result<Event, Error>>) -> KeyboardInputAction {
    match event {
        Some(Ok(event)) => handle_event(&event),
        Some(Err(_)) => KeyboardInputAction::KeypressError,
        None => KeyboardInputAction::KeypressError,
    }
}

fn handle_event(event: &Event) -> KeyboardInputAction {
    match event {
        Event::Key(key) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                if let KeyCode::Char(code) = key.code {
                    if code == 'c' {
                        return KeyboardInputAction::Chars(vec![0x03]);
                    }
                    if code == 't' {
                        return KeyboardInputAction::Menu;
                    }
                }
                return KeyboardInputAction::NoAction; // don't handle other Ctrl-* for now
            }
            match key.code {
                KeyCode::Char(code) => KeyboardInputAction::Chars(vec![code as u8]),
                KeyCode::Enter => KeyboardInputAction::Chars(vec![b'\r']),
                KeyCode::Esc => KeyboardInputAction::Chars(vec![0x1B]),
                KeyCode::Up => KeyboardInputAction::Chars(vec![0x1B, b'[', b'A']),
                KeyCode::Down => KeyboardInputAction::Chars(vec![0x1B, b'[', b'B']),
                KeyCode::Left => KeyboardInputAction::Chars(vec![0x1B, b'[', b'C']),
                KeyCode::Right => KeyboardInputAction::Chars(vec![0x1B, b'[', b'D']),
                KeyCode::Tab => KeyboardInputAction::Chars(vec![b'\t']),
                KeyCode::Backspace => KeyboardInputAction::Chars(vec![0x08]),
                KeyCode::Delete => KeyboardInputAction::Chars(vec![0x7F]),
                KeyCode::Null => KeyboardInputAction::Chars(vec![0x00]),
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
