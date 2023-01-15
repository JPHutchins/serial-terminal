use std::io::stdout;

use chrono;
use crossterm::{
    cursor::{Hide, Show},
    execute,
    style::{
        Color::{DarkGrey, Reset, White},
        Print, SetBackgroundColor, SetForegroundColor,
    },
};

macro_rules! log_to_ui {
    ( $ ( $arg:tt ) * ) => {{
        print_log_to_stdout(format!($($arg)*));
    }}
}

pub(crate) use log_to_ui;

pub fn print_log_to_stdout(msg: String) {
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
}
