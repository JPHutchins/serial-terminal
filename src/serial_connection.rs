use std::{
    io::{stdout, ErrorKind, Write},
    time::Duration,
};

use crossterm::{
    cursor::{Hide, MoveLeft, MoveUp, Show},
    execute, queue,
    style::Print,
};
use terminal_spinner_data::{SpinnerData, DOTS12};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

use crate::log_to_ui::{log_to_ui, print_log_to_stdout};
use crate::Args;

pub async fn wait_for_serial_port(args: &Args, error_kind: Option<ErrorKind>) -> SerialStream {
    const ANIMATION: SpinnerData = DOTS12;

    let mut is_first_retry = true;
    let mut stdout = stdout();
    let mut frame_iter = ANIMATION.frames.into_iter();
    let mut frame = frame_iter.next().unwrap();
    let mut previous_frame_size = frame.chars().count();

    loop {
        match get_serial_connection(&args) {
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
                    frame = match frame_iter.next() {
                        Some(frame) => frame,
                        None => {
                            frame_iter = ANIMATION.frames.into_iter(); // restart the iterator
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
                stdout.flush().unwrap();
            }
        };
        tokio::time::sleep(Duration::from_millis(80)).await;
    }
}

fn get_serial_connection(args: &Args) -> Option<SerialStream> {
    let mut serial_connection_res = tokio_serial::new(args.port.clone(), args.baud)
        .data_bits(args.data_bits.value)
        .flow_control(args.flow_control.value)
        .parity(args.parity.value)
        .stop_bits(args.stop_bits.value)
        .open_native_async();

    match serial_connection_res {
        Ok(mut serial_connection) => {
            #[cfg(unix)]
            serial_connection
                .set_exclusive(false)
                .expect("Unable to set serial port exclusive to false");

            Some(serial_connection)
        }
        Err(_) => None,
    }
}
