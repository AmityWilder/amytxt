use raylib::prelude::{KeyboardKey::*, *};

pub mod rep;

/// A [`KeyboardKey`] that performs an action when pressed
/// (rather than modifying another while held)
///
/// All [`KeyInput`]s are repeatable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyInput {
    /// A character being typed (includes enter/return as `'\n'`)
    Char(char),
    /// Erase the character before the cursor (or all characters selected)
    Backspace,
    /// Erase the character after the cursor (or all characters selected)
    Delete,
    /// Move the cursor to the backward (by either a character or a word)
    Left,
    /// Move the cursor to the forward (by either a character or a word)
    Right,
    /// Move the cursor to the previous line
    Up,
    /// Move the cursor to the next line
    Down,
    /// Move the cursor to the beginning (of either the document or the line)
    Home,
    /// Move the cursor to the end (of either the document or the line)
    End,
}

impl KeyInput {
    /// Get an array of the statuses of every possible key input since the last tick
    pub fn check_pressed(rl: &mut RaylibHandle) -> [Option<Self>; 9] {
        [
            rl.get_char_pressed()
                .or_else(|| rl.is_key_pressed(KEY_ENTER).then_some('\n'))
                .map(KeyInput::Char),
            rl.is_key_pressed(KEY_LEFT).then_some(KeyInput::Left),
            rl.is_key_pressed(KEY_RIGHT).then_some(KeyInput::Right),
            rl.is_key_pressed(KEY_UP).then_some(KeyInput::Up),
            rl.is_key_pressed(KEY_DOWN).then_some(KeyInput::Down),
            rl.is_key_pressed(KEY_DELETE).then_some(KeyInput::Delete),
            rl.is_key_pressed(KEY_BACKSPACE)
                .then_some(KeyInput::Backspace),
            rl.is_key_pressed(KEY_HOME).then_some(KeyInput::Home),
            rl.is_key_pressed(KEY_END).then_some(KeyInput::End),
        ]
    }
}
