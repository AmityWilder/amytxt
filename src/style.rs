//! Text style applied to an editor

use crate::text::MeasureText;
use raylib::prelude::*;

/// Parameters for measuring and displaying text
#[derive(Debug)]
pub struct Style<T> {
    /// The [`RaylibFont`] font the text is rendered in
    pub font: T,
    /// The height of a character in pixels
    pub font_size: f32,
    /// The number of pixels horizontally separating characters
    pub spacing: f32,
    /// The number of pixels vertically separating lines of text
    pub line_space: f32,
    /// The color of the "paper"
    pub background_color: Color,
    /// The color of the text
    pub foreground_color: Color,
    /// The color of the cursor/selection highlight
    pub selection_color: Color,
    /// The width of the cursor when [`Selection::is_empty`] is true
    pub min_selection_width: f32,
}

impl<T> Style<T> {
    /// The distance between the top of the current line and the top of the next line
    pub const fn line_height(&self) -> f32 {
        self.font_size + self.line_space
    }

    /// The width, in pixels, of `text` using the `self` style
    pub fn text_width(&self, text: &str) -> f32
    where
        T: MeasureText,
    {
        self.font.measure_str(text, self.font_size, self.spacing)
    }
}
