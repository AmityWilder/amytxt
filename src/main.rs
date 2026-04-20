//! Notepad without AI

#![feature(
    substr_range,
    cmp_minmax,
    min_specialization,
    const_trait_impl,
    const_default,
    never_type
)]
#![warn(
    clippy::missing_const_for_fn,
    missing_docs,
    clippy::multiple_unsafe_ops_per_block,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    clippy::missing_safety_doc,
    clippy::missing_panics_doc,
    clippy::missing_docs_in_private_items
)]
#![deny(clippy::undocumented_unsafe_blocks)]

use oil::*;
use raylib::prelude::*;
use std::time::Duration;

mod oil;

impl RaylibFont for Oil<Font, WeakFont> {}

impl Unload<&mut RaylibHandle> for WeakFont {
    type Error = !;

    fn unload(self, rl: &mut RaylibHandle) -> Result<(), Self::Error> {
        rl.unload_font(self);
        Ok(())
    }
}

mod text;
use text::{Document, edit::EditableDocument};

mod select;
use select::Selection;

mod style;
use style::Style;

mod input;
use input::{KeyInput, rep::KeyRepeater};

mod editor;
use editor::TextEditor;

fn main() {
    let (mut rl, thread) = init().title("Amitxt").resizable().build();

    let style = Style {
        font: Weak(rl.get_font_default()),
        font_size: 20.0,
        spacing: 2.0,
        line_space: 2.0,
        background_color: Color::new(27, 27, 27, 255),
        foreground_color: Color::WHITE,
        selection_color: Color::BLUEVIOLET,
        min_selection_width: 2.0,
    };

    let mut document = TextEditor::new(
        Rectangle::new(
            0.0,
            0.0,
            rl.get_screen_width() as f32,
            rl.get_screen_height() as f32,
        ),
        Vector2::new(5.0, 5.0),
        600,
        style.font_size,
        style.line_space,
        KeyRepeater::new(Duration::from_millis(500), Duration::from_millis(200)),
    );

    while !rl.window_should_close() {
        if rl.is_window_resized() {
            document.update_clip(style.font_size, style.line_space, |clip| {
                clip.height = rl.get_screen_height() as f32
            });
        }

        let input_buffer: Vec<KeyInput> = KeyInput::check_pressed(&mut rl)
            .into_iter()
            .flatten()
            .collect();

        document.update(&mut rl, input_buffer, &style);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(style.background_color);

        document.draw(&mut d, &style);
    }
}
