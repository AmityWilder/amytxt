#![feature(substr_range, cmp_minmax)]

use raylib::prelude::{KeyboardKey::*, MouseButton::*, *};
use std::ops::{Bound, Index, IndexMut, Range, RangeBounds, RangeInclusive};

trait Document: Index<Range<usize>> + IndexMut<Range<usize>> {
    /// The range of values that are safe to index into
    fn full_range(&self) -> Range<usize>;

    /// Gives the position following the last instance of `delim` at or before `pos`.
    fn start_of(&self, pos: usize, delim: char) -> usize;

    /// Gives the position of the first instance of `delim` at or after `pos`.
    fn end_of(&self, pos: usize, delim: char) -> usize;

    /// Counts the number of newlines before `pos`.
    fn line_index(&self, pos: usize) -> usize;

    /// Gives the position following the last space at or before `pos`.
    fn word_start(&self, pos: usize) -> usize {
        self.start_of(pos, ' ')
    }

    /// Gives the position of the first space at or after `pos`.
    fn word_end(&self, pos: usize) -> usize {
        self.end_of(pos, ' ')
    }

    /// Gives the position following the last newline at or before `pos`.
    fn line_start(&self, pos: usize) -> usize {
        self.start_of(pos, '\n')
    }

    /// Gives the position of the first newline at or after `pos`.
    fn line_end(&self, pos: usize) -> usize {
        self.end_of(pos, '\n')
    }

    /// Snaps `range` to the tightest line boundaries that fully contain it.
    fn line_range(&self, range: Range<usize>) -> Range<usize> {
        self.line_start(range.start)..self.line_end(range.end)
    }

    /// Gives the range of every [`Self::line_index`] that overlaps `range`.
    fn line_indices(&self, range: Range<usize>) -> RangeInclusive<usize> {
        self.line_index(range.start)..=self.line_index(range.end)
    }
}

impl Document for str {
    fn full_range(&self) -> Range<usize> {
        0..self.len()
    }

    fn start_of(&self, pos: usize, delim: char) -> usize {
        self[..pos.min(self.len())]
            .rfind(delim)
            .map(|i| i + delim.len_utf8())
            .unwrap_or(0)
    }

    fn end_of(&self, pos: usize, delim: char) -> usize {
        self[pos.min(self.len())..]
            .find(delim)
            .map(|i| i + pos)
            .unwrap_or(self.len())
    }

    fn line_index(&self, pos: usize) -> usize {
        self[..pos.min(self.len())].matches('\n').count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Selection {
    pub head: usize,
    pub tail: usize,
}

impl Selection {
    pub fn start(&self) -> &usize {
        (&self.head).min(&self.tail)
    }
    pub fn end(&self) -> &usize {
        (&self.head).max(&self.tail)
    }

    pub const fn len(&self) -> usize {
        self.head.abs_diff(self.tail)
    }

    pub const fn is_empty(&self) -> bool {
        self.head == self.tail
    }
}

impl IntoIterator for Selection {
    type Item = usize;
    type IntoIter = Range<usize>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}

impl From<Selection> for Range<usize> {
    fn from(value: Selection) -> Self {
        let [start, end] = std::cmp::minmax(value.head, value.tail);
        start..end
    }
}

impl RangeBounds<usize> for Selection {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(self.start())
    }

    fn end_bound(&self) -> Bound<&usize> {
        Bound::Excluded(self.end())
    }

    #[inline]
    fn contains<U>(&self, item: &U) -> bool
    where
        usize: PartialOrd<U>,
        U: ?Sized + PartialOrd<usize>,
    {
        Range::from(*self).contains(item)
    }
}

impl std::ops::Index<Selection> for str {
    type Output = str;

    #[inline]
    fn index(&self, index: Selection) -> &Self::Output {
        self.index(Range::from(index))
    }
}

#[derive(Debug)]
pub struct TextStyle<T> {
    pub font: T,
    pub font_size: f32,
    pub spacing: f32,
    pub line_space: f32,
    pub background_color: Color,
    pub foreground_color: Color,
    pub selection_color: Color,
    pub min_selection_width: f32,
}

impl<T> TextStyle<T> {
    pub const fn line_height(&self) -> f32 {
        self.font_size + self.line_space
    }
}

impl<T: RaylibFont> TextStyle<T> {
    pub fn text_width(&self, text: &str) -> f32 {
        self.font.measure_text(text, self.font_size, self.spacing).x
    }
}

const fn calculate_fittable_lines(clip_height: f32, pad_y: f32, line_height: f32) -> usize {
    ((clip_height - 2.0 * pad_y) / line_height) as usize
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TextEditor {
    pub content: String,
    pub scroll: usize,
    pub selection: Selection,
    clip: Rectangle,
    pad: Vector2,
    fittable_lines: usize,
    pub wrap: u32,
}

impl TextEditor {
    pub const fn new(clip: Rectangle, pad: Vector2, wrap: u32, line_height: f32) -> Self {
        Self {
            content: String::new(),
            scroll: 0,
            selection: Selection { head: 0, tail: 0 },
            clip,
            pad,
            fittable_lines: calculate_fittable_lines(clip.height, pad.y, line_height),
            wrap,
        }
    }

    const fn calculate_fittable_lines(&self, line_height: f32) -> usize {
        calculate_fittable_lines(self.clip.height, self.pad.y, line_height)
    }

    pub const fn fittable_lines(&self) -> usize {
        self.fittable_lines
    }

    pub const fn clip(&self) -> &Rectangle {
        &self.clip
    }

    pub const fn pad(&self) -> &Vector2 {
        &self.pad
    }

    pub fn update_clip<F>(&mut self, line_height: f32, f: F)
    where
        F: FnOnce(&mut Rectangle),
    {
        f(&mut self.clip);
        self.fittable_lines = self.calculate_fittable_lines(line_height);
    }

    pub fn update_pad<F>(&mut self, line_height: f32, f: F)
    where
        F: FnOnce(&mut Vector2),
    {
        f(&mut self.pad);
        self.fittable_lines = self.calculate_fittable_lines(line_height);
    }

    pub fn update(&mut self, rl: &mut RaylibHandle) {
        let is_shifting = rl.is_key_down(KEY_LEFT_SHIFT) || rl.is_key_down(KEY_RIGHT_SHIFT);
        let is_ctrling = rl.is_key_down(KEY_LEFT_CONTROL) || rl.is_key_down(KEY_RIGHT_CONTROL);
        let is_alting = rl.is_key_down(KEY_LEFT_ALT) || rl.is_key_down(KEY_RIGHT_ALT);

        let mut is_moved = false;
        if rl.is_key_pressed(KEY_RIGHT) {
            is_moved = true;
            self.selection.tail = if self.selection.is_empty() || is_shifting {
                if is_ctrling {
                    self.content.word_end(
                        self.selection
                            .tail
                            .saturating_add(1)
                            .min(self.content.len()),
                    )
                } else {
                    self.selection
                        .tail
                        .saturating_add(1)
                        .min(self.content.len())
                }
            } else {
                *self.selection.end()
            };
        }
        if rl.is_key_pressed(KEY_LEFT) {
            is_moved = true;
            self.selection.tail = if self.selection.is_empty() || is_shifting {
                if is_ctrling {
                    self.content
                        .word_start(self.selection.tail.saturating_sub(1))
                } else {
                    self.selection.tail.saturating_sub(1)
                }
            } else {
                *self.selection.start()
            };
        }
        if rl.is_key_pressed(KEY_END) {
            is_moved = true;
            self.selection.tail = if is_ctrling {
                self.content.len()
            } else {
                self.content.line_end(self.selection.tail)
            };
        }
        if rl.is_key_pressed(KEY_HOME) {
            is_moved = true;
            self.selection.tail = if is_ctrling {
                0
            } else {
                self.content.line_start(self.selection.tail)
            };
        }
        if is_moved && !is_shifting {
            self.selection.head = self.selection.tail;
        }

        if let Some(ch) = rl
            .get_char_pressed()
            .or_else(|| rl.is_key_pressed(KEY_ENTER).then_some('\n'))
        {
            let mut buf = [0; 4];
            self.content
                .replace_range(self.selection, ch.encode_utf8(&mut buf));
            self.selection.tail += ch.len_utf8();
            self.selection.head = self.selection.tail;
        } else if rl.is_key_pressed(KEY_BACKSPACE) {
            if self.selection.is_empty() {
                self.selection.tail = self.selection.tail.saturating_sub(1);
            }
            self.content.replace_range(self.selection, "");
            self.selection.head = self.selection.tail;
        } else if rl.is_key_pressed(KEY_DELETE) {
            if self.selection.is_empty() {
                self.selection.tail = self
                    .selection
                    .tail
                    .saturating_add(1)
                    .min(self.content.len());
            }
            if *self.selection.start() < self.content.len() {
                self.content.replace_range(self.selection, "");
            }
            self.selection.head = self.selection.head.min(self.content.len());
            self.selection.tail = self.selection.head;
        }

        self.scroll = self
            .scroll
            .saturating_sub_signed(rl.get_mouse_wheel_move() as isize)
            .min(self.content.lines().count());
    }

    pub fn clipped_lines(&self) -> impl Iterator<Item = (Range<usize>, &str)> {
        self.content
            .lines()
            .skip(self.scroll)
            .take(self.fittable_lines())
            .map(|line| {
                (
                    self.content
                        .substr_range(line)
                        .expect("all lines of document should be in document"),
                    line,
                )
            })
    }
}

fn main() {
    let (mut rl, thread) = init().title("Amitxt").resizable().build();

    let style = TextStyle {
        font: rl.get_font_default(),
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
        style.line_height(),
    );

    while !rl.window_should_close() {
        if rl.is_window_resized() {
            document.update_clip(style.line_height(), |clip| {
                clip.height = rl.get_screen_height() as f32
            });
        }

        document.update(&mut rl);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(style.background_color);

        let selection_lines = document.content.line_indices(document.selection.into());
        for (screen_linenum, (line_range, line)) in document.clipped_lines().enumerate() {
            let document_linenum = screen_linenum - document.scroll;
            if selection_lines.contains(&document_linenum) {
                let selection_range = Range::from(document.selection);
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
                let pre_width = style.text_width(&line[..inline_start]);
                let selected_width = style.text_width(&line[inline_start..inline_end]);
                d.draw_rectangle_rec(
                    Rectangle::new(
                        document.pad.x
                            + pre_width
                            + if selection_range.is_empty() {
                                -0.5 * style.min_selection_width
                            } else {
                                0.0
                            },
                        document.pad.y
                            + (document_linenum - document.scroll) as f32 * (style.line_height()),
                        (selected_width + style.spacing).max(style.min_selection_width),
                        style.font_size,
                    ),
                    style.selection_color,
                );
            }

            d.draw_text_pro(
                &style.font,
                line,
                Vector2::new(document.clip.x, document.clip.y)
                    + document.pad
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
