use std::io;

use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyModifiers};

pub enum KeyboardInputAction {
    Chars(Vec<u8>),
    KeypressError,
    Menu,
    NoAction,
}

pub fn handle_keypress_event(
    event: &Option<Result<CrosstermEvent, io::Error>>,
) -> KeyboardInputAction {
    match event {
        Some(Ok(event)) => handle_event(&event),
        Some(Err(_)) => KeyboardInputAction::KeypressError,
        None => KeyboardInputAction::KeypressError,
    }
}

fn handle_event(event: &CrosstermEvent) -> KeyboardInputAction {
    match event {
        CrosstermEvent::Key(key) => {
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
        CrosstermEvent::FocusGained
        | CrosstermEvent::FocusLost
        | CrosstermEvent::Mouse(_)
        | CrosstermEvent::Resize(_, _)
        | CrosstermEvent::Paste(_) => KeyboardInputAction::NoAction,
    }
}
