use const_format::concatcp;
use futures::{future::FutureExt, pin_mut, select, stream::StreamExt};
use std::{
    format,
    io::{
        self,
        ErrorKind::{PermissionDenied, TimedOut, WouldBlock},
        Write,
    },
};

use clap::{error::ContextKind::InvalidArg, error::ContextValue, Parser};
use crossterm::{
    cursor,
    event::EventStream,
    execute, queue,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType::CurrentLine},
};
use tokio::io::AsyncReadExt;
use tokio_serial::{DataBits, FlowControl, Parity, SerialStream, StopBits};

mod arg_helpers;
mod constants;
mod keyboard_input;
mod list_ports;
mod log_to_ui;
mod menu;
mod serial_connection;

use crate::arg_helpers::{
    valid_baud, valid_data_bits, valid_flow_control, valid_parity, valid_stop_bits, CLIDisplay,
};
use crate::constants::{ABOUT, HELP, LONG_VERSION};
use crate::keyboard_input::{handle_keypress_event, KeyboardInputAction};
use crate::list_ports::list_ports;
use crate::log_to_ui::{log_to_ui, print_log_to_stdout};
use crate::serial_connection::wait_for_serial_port;

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

enum EventType {
    Menu,
    SerialRX,
    Initial,
}

async fn io_tasks(args: Args) {
    let mut reader = EventStream::new();
    let mut rx_buf: [u8; 1] = [0; 1];
    let mut stdout = io::stdout();

    let connect_event_fut = wait_for_serial_port(&args, None).fuse();
    pin_mut!(connect_event_fut);

    'connection: loop {
        stdout.flush().unwrap();

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
                serial_conn = event;
            },
        }

        let mut menu_state = menu::MenuState::new(cursor::position().unwrap());
        let mut serial_rx_cursor_position: (u16, u16) = cursor::position().unwrap();
        let mut event_type = EventType::Initial;

        let mut ansi_sequence: Vec<u8> = Vec::with_capacity(10);

        'communication: loop {
            let keypress_event = reader.next().fuse();
            let serial_rx_event = serial_conn.read_exact(&mut rx_buf).fuse();
            pin_mut!(keypress_event, serial_rx_event);

            queue!(stdout, cursor::Hide).unwrap();

            select! {
                event = keypress_event => {
                    match handle_keypress_event(&event) {
                        KeyboardInputAction::Chars(bytes) => {
                            if menu_state.is_open {
                                let event = event.unwrap().unwrap();
                                menu_state = menu::handle_chars(menu_state, event);
                                event_type = EventType::Menu;
                                match menu_state.action {
                                    None => {},
                                    Some(menu::Action::Quit) => break 'connection,
                                    Some(menu::Action::Timestamp) => {
                                        // overwrite the menu line with the timestamp
                                        queue!(
                                            stdout,
                                            cursor::MoveUp(1),
                                        ).unwrap();
                                        log_to_ui!(""); // blank log is just a timestamp

                                        // then reprint the menu
                                        menu_state = menu::newline(menu_state);
                                        serial_rx_cursor_position = (
                                            serial_rx_cursor_position.0,
                                            menu_state.cursor_position.1 - 1);
                                    },
                                    Some(menu::Action::Help) => {
                                        //TODO
                                    }
                                }
                            } else {
                                match serial_conn.write(&bytes) {
                                    Ok(_) => {},
                                    Err(error) => match error.kind() {
                                        WouldBlock => {},
                                        _ => log_to_ui!("Serial TX Error: {:?}", error)
                                    }
                                }
                            }
                        }
                        KeyboardInputAction::KeypressError => {
                            log_to_ui!("Keypress error");
                            break 'connection
                        }
                        KeyboardInputAction::Menu => {
                            if menu_state.is_open {
                                menu_state = menu::close(menu_state);
                                event_type = EventType::Initial;
                            } else {
                                menu_state = menu::newline(menu_state);
                                serial_rx_cursor_position = (
                                    serial_rx_cursor_position.0,
                                    menu_state.cursor_position.1 - 1);
                                event_type = EventType::Menu;
                            }
                        },
                        KeyboardInputAction::NoAction => {},
                    };
                },
                event = serial_rx_event => {
                    match event {
                        Ok(_) => {
                            event_type = EventType::Initial;

                            if ansi_sequence.len() > 0 { // continue buffering an ANSI sequence
                                assert_ne!(rx_buf[0], 0x1b); // second escape received
                                ansi_sequence.push(rx_buf[0]);
                                if rx_buf[0] == 'm' as u8 { // ANSI color finished
                                    let ansi_string: String = ansi_sequence
                                        .iter()
                                        .map(|&c| c as char)
                                        .collect();

                                    let (col, row) = serial_rx_cursor_position;
                                    queue!(
                                        stdout,
                                        cursor::MoveTo(col, row),
                                        Print(ansi_string),
                                    ).unwrap();
                                    event_type = EventType::SerialRX;
                                    ansi_sequence.clear();
                                }
                            } else if rx_buf[0] == 0x1b { // begin buffering an ANSI sequence
                                ansi_sequence.push(0x1b);
                            } else if menu_state.is_open && rx_buf[0] == '\n' as u8 {
                                // move to the menu line and clear it
                                queue!(
                                    stdout,
                                    cursor::MoveTo(0, menu_state.cursor_position.1),
                                    Clear(CurrentLine),
                                ).unwrap();

                                // add the menu back
                                menu_state = menu::newline(menu_state);

                                // manually set the serial rx cursor up 1 row from the menu
                                serial_rx_cursor_position = (0, menu_state.cursor_position.1 - 1);
                            } else {
                                let (col, row) = serial_rx_cursor_position;
                                queue!(
                                    stdout,
                                    cursor::MoveTo(col, row),
                                    Print(rx_buf[0] as char),
                                ).unwrap();
                                event_type = EventType::SerialRX;
                            }
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

            // update the text and cursor positions if they've changed
            stdout.flush().unwrap();

            match &event_type {
                EventType::Menu => menu_state.cursor_position = cursor::position().unwrap(),
                EventType::SerialRX => serial_rx_cursor_position = cursor::position().unwrap(),
                EventType::Initial => {}
            };

            // update the position of the cursor
            if menu_state.is_open {
                execute!(
                    stdout,
                    cursor::MoveTo(menu_state.cursor_position.0, menu_state.cursor_position.1),
                )
                .unwrap();
            } else {
                execute!(
                    stdout,
                    cursor::MoveTo(serial_rx_cursor_position.0, serial_rx_cursor_position.1),
                )
                .unwrap();
            }
            execute!(stdout, cursor::Show).unwrap();
        }
    }
}
