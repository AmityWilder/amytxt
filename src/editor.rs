use crate::{
    Document, EditableDocument, KeyInput, KeyRepeater, Selection, style::Style, text::MeasureText,
};
use raylib::prelude::{KeyboardKey::*, MouseButton::*, *};
use std::ops::*;

/// The number of lines of text that can fit into the content region a rectangle
/// that is `clip_height` pixels tall with `pad_y` pixels of padding
///
/// Assumes the padding is the same on the bottom as it is on the top
const fn fittable_lines(clip_height: f32, pad_y: f32, font_size: f32, line_space: f32) -> usize {
    // because the line space is blank, we can fit one extra outside of the clipping region
    ((clip_height - 2.0 * pad_y + line_space) / (font_size + line_space)) as usize
}

/// A container for editable text
#[derive(Debug, Clone, PartialEq)]
pub struct TextEditor<Doc> {
    /// The text content of the document
    pub content: Doc,
    /// How many lines to offset
    pub scroll: usize,
    /// The current selection within the document
    pub selection: Selection,
    /// The rectangle that determines how much of the document is displayed
    ///
    /// This is also the region covered by the background color
    clip: Rectangle,
    /// How far inward from the edges of `clip` the content is squeezed
    pad: Vector2,
    /// Cache how many lines can be rendered within the `clip` rectangle
    /// (TODO: isn't this is dependent on [`Style`]?)
    fittable_lines: usize,
    /// How many pixels wide a line can get before wrapping
    /// (TODO: doesn't this affect how many lines there are?)
    pub wrap: u32,
    /// How the editor should repeat key inputs
    pub key_repeat: KeyRepeater,
}

impl<Doc: Default> Default for TextEditor<Doc> {
    fn default() -> Self {
        Self {
            content: Default::default(),
            scroll: Default::default(),
            selection: Default::default(),
            clip: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            pad: Vector2::zero(),
            fittable_lines: Default::default(),
            wrap: Default::default(),
            key_repeat: Default::default(),
        }
    }
}

impl<Doc> TextEditor<Doc> {
    /// Construct a new [`TextEditor`] with a blank document
    pub fn new(
        clip: Rectangle,
        pad: Vector2,
        wrap: u32,
        font_size: f32,
        line_space: f32,
        key_repeat: KeyRepeater,
    ) -> Self
    where
        Doc: Default,
    {
        Self {
            content: Doc::default(),
            scroll: 0,
            selection: Selection { head: 0, tail: 0 },
            clip,
            pad,
            fittable_lines: fittable_lines(clip.height, pad.y, font_size, line_space),
            wrap,
            key_repeat,
        }
    }

    /// Call [`fittable_lines`] given this editor's parameters
    const fn calculate_fittable_lines(&self, font_size: f32, line_space: f32) -> usize {
        fittable_lines(self.clip.height, self.pad.y, font_size, line_space)
    }

    /// Access the cached copy of the most recently calculated number of lines that can fit in the [`Self::clip`] rectangle
    pub const fn fittable_lines(&self) -> usize {
        self.fittable_lines
    }

    /// Access the clip region of the editor
    pub const fn clip(&self) -> &Rectangle {
        &self.clip
    }

    /// Access the padding of the editor
    pub const fn pad(&self) -> &Vector2 {
        &self.pad
    }

    /// Mutate the clip region of the editor and recalculate [`Self::fittable_lines`]
    pub fn update_clip<F>(&mut self, font_size: f32, line_space: f32, f: F)
    where
        F: FnOnce(&mut Rectangle),
    {
        f(&mut self.clip);
        self.fittable_lines = self.calculate_fittable_lines(font_size, line_space);
    }

    /// Mutate the padding of the editor and recalculate [`Self::fittable_lines`]
    pub fn update_pad<F>(&mut self, font_size: f32, line_space: f32, f: F)
    where
        F: FnOnce(&mut Vector2),
    {
        f(&mut self.pad);
        self.fittable_lines = self.calculate_fittable_lines(font_size, line_space);
    }

    /// Tick the editor
    pub fn update<I, T>(&mut self, rl: &mut RaylibHandle, inputs: I, style: &Style<T>)
    where
        Doc: for<'a> EditableDocument,
        I: IntoIterator<Item = KeyInput>,
        T: MeasureText,
    {
        let is_shifting = rl.is_key_down(KEY_LEFT_SHIFT) || rl.is_key_down(KEY_RIGHT_SHIFT);
        let is_ctrling = rl.is_key_down(KEY_LEFT_CONTROL) || rl.is_key_down(KEY_RIGHT_CONTROL);
        // let is_alting = rl.is_key_down(KEY_LEFT_ALT) || rl.is_key_down(KEY_RIGHT_ALT);

        if rl.is_mouse_button_down(MOUSE_BUTTON_LEFT) {
            let mouse_point = rl.get_mouse_position();
            let mouse_point_rel = mouse_point - Vector2::new(self.clip.x, self.clip.y) - self.pad;
            let line = self
                .content
                // something is wrong here and it's always going to 0
                .find_line_slice(style.line_height(), mouse_point_rel.y, self.scroll);
            let pos = self.content.subset_range(line).start
                + line.find_pos(mouse_point_rel.x, |s| style.text_width(s));
            self.selection.tail = pos;
            if rl.is_mouse_button_pressed(MOUSE_BUTTON_LEFT) && !is_shifting {
                self.selection.head = self.selection.tail;
            }
        }

        for input in inputs {
            debug_assert!(
                *self.selection.end() <= self.content.len(),
                "the selection should always be within bounds"
            );
            match input {
                KeyInput::Char(ch) => {
                    // TODO: implement letter-combo hotkeys like ctrl+c/ctrl+v here
                    self.content
                        .replace_range(self.selection, ch.encode_utf8(&mut [0; 4]));
                    self.selection.tail += ch.len_utf8();
                    self.selection.head = self.selection.tail;
                }

                KeyInput::Backspace => {
                    // select the previous character if none are selected
                    if self.selection.is_empty() {
                        self.selection.tail = self.content.prev_char(self.selection.tail);
                    }
                    debug_assert!(
                        *self.selection.start() < self.content.len(),
                        "even if the tail is is at the end, at least one character within bounds should exist"
                    );
                    // erase the selection
                    self.content.erase_range(self.selection);
                    // flatten the selection range onto the tail
                    self.selection.head = self.selection.tail;
                }

                KeyInput::Delete => {
                    // select the next character if none are selected
                    if self.selection.is_empty() {
                        self.selection.tail = self.content.next_char(self.selection.tail);
                    }
                    // replace the range as long as the cursor isn't at the end of the text
                    // (which is a valid position to be in, since that's where it would append characters)
                    if *self.selection.start() < self.content.len() {
                        self.content.erase_range(self.selection);
                    }
                    self.selection.head = self.selection.head.min(self.content.len());
                    // flatten the selection range onto the tail
                    self.selection.tail = self.selection.head;
                }

                KeyInput::Left => {
                    if is_shifting || self.selection.is_empty() {
                        self.selection.tail = self.content.prev_char(self.selection.tail);
                        if is_ctrling {
                            self.selection.tail = self.content.word_start(self.selection.tail);
                        }
                    } else {
                        self.selection.tail = *self.selection.start();
                    }
                }

                KeyInput::Right => {
                    if is_shifting || self.selection.is_empty() {
                        self.selection.tail = self.content.next_char(self.selection.tail);
                        if is_ctrling {
                            self.selection.tail = self.content.word_end(self.selection.tail);
                        }
                    } else {
                        self.selection.tail = *self.selection.end();
                    }
                }

                KeyInput::Up => {
                    self.selection.tail = self
                        .content
                        .prev_char(self.content.line_start(self.selection.tail));
                    // TODO: align with column
                }

                KeyInput::Down => {
                    self.selection.tail = self
                        .content
                        .next_char(self.content.line_end(self.selection.tail));
                    // TODO: align with column
                }

                KeyInput::Home => {
                    self.selection.tail = if is_ctrling {
                        0
                    } else {
                        self.content.line_start(self.selection.tail)
                    };
                }

                KeyInput::End => {
                    self.selection.tail = if is_ctrling {
                        self.content.len()
                    } else {
                        self.content.line_end(self.selection.tail)
                    };
                }
            }

            if !is_shifting {
                self.selection.head = self.selection.tail;
            }
        }

        self.scroll = self
            .scroll
            .saturating_sub_signed(rl.get_mouse_wheel_move() as isize)
            .min(self.content.line_count());
    }
}

impl TextEditor<String> {
    /// Get an iterator over lines and their ranges within `content`
    pub fn ranged_lines(&self) -> impl DoubleEndedIterator<Item = (Range<usize>, &str)> {
        self.content
            .lines()
            .map(|line| match self.content.substr_range(line) {
                Some(range) => (range, line),
                None => unreachable!("all lines of `document` should be in `document`"),
            })
    }

    /// Render the editor
    pub fn draw<D, T>(&self, d: &mut D, style: &Style<T>)
    where
        D: RaylibDraw + std::ops::DerefMut<Target = RaylibHandle>,
        T: RaylibFont,
    {
        let selection_range = Range::from(self.selection);
        let selection_lines = self.content.line_indices(selection_range.clone());
        for (screen_linenum, (document_linenum, (line_range, line))) in self
            .ranged_lines()
            .enumerate()
            .skip(self.scroll)
            .take(self.fittable_lines())
            .enumerate()
        {
            // only highlight the line if it's within the selection
            if selection_lines.contains(&document_linenum) {
                // start of the selection clamped to the start of the line
                // - if the start is within this line, it becomes its offset from the start of the line
                // - if the start is before this line, it clamps to 0
                // - if the start is after this line, we would not have reached this point
                let inline_start = selection_range.start.saturating_sub(line_range.start);

                // end of the selection clamped to the end of the line
                // - if the end is within this line, it becomes its offset from the start of the line
                // - if the end is before this line, we would not have reached this point
                // - if the end is after this line, it clamps to the length of the line
                let inline_end = selection_range
                    .end
                    .min(line_range.end)
                    .saturating_sub(line_range.start);

                // number of pixels in the line before the cursor
                let pre_width = style.text_width(&line[..inline_start]);

                // number of pixels in the line that overlap the cursor
                let selected_width = style.text_width(&line[inline_start..inline_end]);

                d.draw_rectangle_rec(
                    Rectangle::new(
                        self.pad.x
                            + pre_width
                            + if self.selection.is_empty() {
                                -0.5 * style.min_selection_width
                            } else {
                                0.0
                            },
                        self.pad.y
                            + (document_linenum - self.scroll) as f32 * (style.line_height()),
                        (selected_width + style.spacing).max(style.min_selection_width),
                        style.font_size,
                    ),
                    style.selection_color,
                );
            }

            d.draw_text_pro(
                &style.font,
                line,
                Vector2::new(self.clip.x, self.clip.y)
                    + self.pad
                    + Vector2::new(0.0, screen_linenum as f32 * (style.line_height())),
                Vector2::zero(),
                0.0,
                style.font_size,
                style.spacing,
                style.foreground_color,
            );
        }
    }
}
