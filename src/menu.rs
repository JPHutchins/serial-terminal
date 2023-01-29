use std::io::stdout;

use crossterm::{
    cursor,
    event::{Event as CrosstermEvent, KeyCode},
    queue,
    style::{
        Color::{DarkGrey, Reset, White},
        Print, SetBackgroundColor, SetForegroundColor,
    },
    terminal::{size, Clear, ClearType::CurrentLine},
};
use phf::phf_map;

static MENU_COMMANDS: phf::Map<&'static str, Action> = phf_map! {
    "quit" => Action::Quit,
    "q" => Action::Quit,
    "timestamp" => Action::Timestamp,
    "ts" => Action::Timestamp,
    "help" => Action::Help,
    "h" => Action::Help,
    "?" => Action::Help,
};

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Quit,
    Timestamp,
    Help,
}

pub struct MenuState {
    pub is_open: bool,
    pub command: String,
    pub cursor_position: (u16, u16),
    pub action: Option<Action>,
    error: Option<String>,
}

impl MenuState {
    pub fn new(position: (u16, u16)) -> MenuState {
        MenuState {
            is_open: false,
            command: String::from(""),
            cursor_position: position,
            action: None,
            error: None,
        }
    }
}

pub fn newline(menu_state: MenuState) -> MenuState {
    let (col, _) = size().unwrap();
    let blank_row: String = vec![" "; col.into()].into_iter().collect();

    queue!(
        stdout(),
        Print("\n"),
        cursor::MoveTo(0, menu_state.cursor_position.1),
        SetBackgroundColor(DarkGrey),
        SetForegroundColor(White),
        Print(blank_row),
        cursor::MoveTo(0, menu_state.cursor_position.1),
        Print(": "),
        Print(&menu_state.command),
        SetBackgroundColor(Reset),
        SetForegroundColor(Reset),
    )
    .unwrap();

    return MenuState {
        is_open: true,
        command: menu_state.command,
        cursor_position: menu_state.cursor_position,
        action: None,
        error: None,
    };
}

pub fn close(menu_state: MenuState) -> MenuState {
    queue!(
        stdout(),
        cursor::MoveTo(3, menu_state.cursor_position.1),
        Clear(CurrentLine),
    )
    .unwrap();

    return MenuState {
        is_open: false,
        command: menu_state.command,
        cursor_position: menu_state.cursor_position,
        action: None,
        error: None,
    };
}

pub fn handle_chars(menu_state: MenuState, event: CrosstermEvent) -> MenuState {
    let mut new_menu_state = MenuState {
        is_open: true,
        command: menu_state.command,
        cursor_position: menu_state.cursor_position,
        action: None,
        error: None,
    };

    match event {
        CrosstermEvent::Key(key) => {
            match key.code {
                KeyCode::Char(code) => {
                    new_menu_state.command.push(code);
                    new_menu_state.cursor_position = (
                        new_menu_state.cursor_position.0 + 1,
                        new_menu_state.cursor_position.1,
                    )
                }
                KeyCode::Enter => {
                    if menu_state.error != None {
                        // clear the previous error and reveal the bad command but don't try to
                        // execute it again
                        new_menu_state.error = None;
                    } else if MENU_COMMANDS.contains_key(&new_menu_state.command) {
                        new_menu_state.action = Some(MENU_COMMANDS[&new_menu_state.command]);
                    } else {
                        new_menu_state.error =
                            Some(format!("{} is an unknown command", new_menu_state.command));
                    }
                }
                KeyCode::Esc => {
                    // TODO: back key
                }
                KeyCode::Backspace => {
                    if new_menu_state.command.len() > 0 {
                        new_menu_state.command.pop();
                    }
                }
                _ => {}
            }
        }
        CrosstermEvent::FocusGained
        | CrosstermEvent::FocusLost
        | CrosstermEvent::Mouse(_)
        | CrosstermEvent::Resize(_, _)
        | CrosstermEvent::Paste(_) => {}
    };

    let text_displayed = match &new_menu_state.error {
        None => &new_menu_state.command,
        Some(error) => &error,
    };

    let (col, _) = size().unwrap();
    let blank_row: String = vec![" "; col.into()].into_iter().collect();

    queue!(
        stdout(),
        cursor::MoveTo(0, new_menu_state.cursor_position.1),
        Clear(CurrentLine),
        SetBackgroundColor(DarkGrey),
        SetForegroundColor(White),
        Print(blank_row),
        cursor::MoveTo(0, new_menu_state.cursor_position.1),
        Print(": "),
        Print(text_displayed),
        SetBackgroundColor(Reset),
        SetForegroundColor(Reset),
    )
    .unwrap();

    new_menu_state
}
