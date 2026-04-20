//! Notepad without AI

#![feature(substr_range, cmp_minmax, min_specialization)]
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

use raylib::prelude::{KeyboardKey::*, MouseButton::*, *};
use std::{
    ops::*,
    time::{Duration, Instant},
};

/// Owned If Loaded Font
#[derive(Debug)]
pub enum OilFont {
    /// An unowned font that will be unloaded independently.
    Weak(WeakFont),
    /// An owned font that must be unloaded when this enum is dropped.
    Strong(Font),
}

impl AsRef<ffi::Font> for OilFont {
    fn as_ref(&self) -> &ffi::Font {
        match self {
            Self::Weak(weak_font) => weak_font.as_ref(),
            Self::Strong(font) => font.as_ref(),
        }
    }
}

impl AsMut<ffi::Font> for OilFont {
    fn as_mut(&mut self) -> &mut ffi::Font {
        match self {
            Self::Weak(weak_font) => weak_font.as_mut(),
            Self::Strong(font) => font.as_mut(),
        }
    }
}

impl RaylibFont for OilFont {}

impl From<WeakFont> for OilFont {
    fn from(font: WeakFont) -> Self {
        Self::Weak(font)
    }
}

impl From<Font> for OilFont {
    fn from(font: Font) -> Self {
        Self::Strong(font)
    }
}

impl OilFont {
    /// Returns [`None`] if `ch` has negative width (like newline)
    fn measure_base_char(&self, ch: char) -> Option<f32> {
        (ch != '\n').then(|| {
            let index = usize::try_from(self.get_glyph_index(ch)).unwrap();
            let font = self.as_ref();
            let glyph_count = usize::try_from(font.glyphCount).unwrap();
            // SAFETY: It is Raylib's responsibility to uphold this contract
            let glyphs = unsafe { std::slice::from_raw_parts(font.glyphs, glyph_count) };
            // SAFETY: It is Raylib's responsibility to uphold this contract
            let recs = unsafe { std::slice::from_raw_parts(font.recs, glyph_count) };
            if glyphs[index].advanceX > 0 {
                glyphs[index].advanceX as f32
            } else {
                recs[index].width + glyphs[index].offsetX as f32
            }
        })
    }

    /// Assumes text is one line and does not wrap
    fn measure_text(&self, text: &str, font_size: f32, spacing: f32) -> f32 {
        debug_assert!(self.base_size() > 0);
        debug_assert!(font_size == 0.0 || font_size.is_normal());
        debug_assert!(spacing == 0.0 || spacing.is_normal());
        text.chars()
            .map(|ch| {
                debug_assert_ne!(ch, '\n');
                self.measure_base_char(ch)
                    .expect("no glyph should have negative width")
            })
            .sum::<f32>()
            * (font_size / self.base_size() as f32) // scale factor
            + text.chars().count().saturating_sub(1) as f32 * spacing
    }
}

/// Iterator over lines in a string, including virtual lines created by wrapping.
///
/// Tries to wrap by word, then by character if a single word is wider than an entire line
#[derive(Debug, Clone)]
pub struct WrappedLines<'a, 'b, I> {
    /// Iterator over lines
    lines: I,
    /// The line currently being split into wrapping
    curr_line: Option<&'a str>,
    /// The font, font_height, and spacing
    style: &'b TextStyle,
    /// The maximum width before wrapping
    wrap_width: f32,
}

impl<'a, 'b, I> WrappedLines<'a, 'b, I> {
    /// Construct a new [`WrappedLines`] iterator
    const fn new(lines: I, style: &'b TextStyle, wrap_width: f32) -> Self {
        Self {
            lines,
            curr_line: None,
            style,
            wrap_width,
        }
    }
}

impl<'a, I> Iterator for WrappedLines<'a, '_, I>
where
    I: Iterator<Item = &'a str>,
{
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.curr_line.is_none() {
            self.curr_line = self.lines.next();
        }
        self.curr_line.take().map(|curr_line| {
            debug_assert!(!curr_line.contains('\n'));
            // find the first character that doesn't fit in the line
            if let Some((split_at, _)) = curr_line
                .char_indices()
                .map(|(i, _)| (i, self.style.text_width(&curr_line[..i])))
                .find(|(_, w)| *w > self.wrap_width)
            {
                // find the start of the word that splits
                let split_at = curr_line[..split_at].rfind(' ').unwrap_or(split_at);
                let pre_wrap = &curr_line[..split_at];
                self.curr_line = Some(&curr_line[split_at + ' '.len_utf8()..]);
                pre_wrap
            } else {
                curr_line
            }
        })
    }
}

/// Wrappable text
pub trait WrappedLinesExt {
    /// Returns an iterator over wrapped lines of text
    fn wrapped_lines<'a, 'b>(
        &'a self,
        style: &'b TextStyle,
        wrap_width: f32,
    ) -> WrappedLines<'a, 'b, std::str::Lines<'a>>;
}

impl WrappedLinesExt for str {
    #[inline]
    fn wrapped_lines<'a, 'b>(
        &'a self,
        style: &'b TextStyle,
        wrap_width: f32,
    ) -> WrappedLines<'a, 'b, std::str::Lines<'a>> {
        WrappedLines::new(self.lines(), style, wrap_width)
    }
}

/// Any form of text that can be selected and edited like a Word document
pub trait Document {
    /// The range of values that are safe to index into
    fn len(&self) -> usize;

    /// Whether the document or slice contains no elements
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of lines in the document
    fn line_count(&self) -> usize;

    /// Gives a [`Self::Slice`] covering the entire document, which can then be range-indexed
    fn as_slice(&self) -> &str;

    /// Returns the start position of the character following the one starting at `pos`, clamped to the document bounds
    fn next_char(&self, pos: usize) -> usize;

    /// Returns the start position of the character preceeding the one starting at `pos`, clamped to the document bounds
    fn prev_char(&self, pos: usize) -> usize;

    /// Gives the position following the last instance of `delim` at or before `pos`.
    fn start_of(&self, pos: usize, delim: char) -> usize;

    /// Gives the position of the first instance of `delim` at or after `pos`.
    fn end_of(&self, pos: usize, delim: char) -> usize;

    /// Counts the number of newlines before `pos`.
    fn line_index(&self, pos: usize) -> usize;

    /// Gives the position following the last space or newline at or before `pos`.
    fn word_start(&self, pos: usize) -> usize {
        self.start_of(pos, ' ').max(self.line_start(pos))
    }

    /// Gives the position of the first space or newline at or after `pos`.
    fn word_end(&self, pos: usize) -> usize {
        self.end_of(pos, ' ').min(self.line_end(pos))
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

    /// Given a vertical point within the document, identifies the index of the
    /// last line that is not further in the document than the point.
    ///
    /// # Warning
    /// Assumes there is no line wrapping in the document.
    ///
    /// # Panics
    /// This method may panic if any of the following is true:
    /// - `line_height` is non-positive
    /// - `point` or `line_height` is subnormal
    fn find_line(&self, line_height: f32, point: f32, offset: usize) -> usize {
        assert!(line_height > 0.0);
        assert!((point == 0.0 || point.is_normal()) && line_height.is_normal());
        ((point / line_height) as usize)
            .saturating_add(offset)
            .min(self.line_count().saturating_sub(1))
    }

    /// Finds the range between newline characters on the line found with [`Self::find_line`]
    fn find_line_slice(&self, line_height: f32, point: f32, offset: usize) -> &str;

    /// Given a horizontal point within a line, locates the nearest character
    /// position to the point in the line, clamped to the line bounds.
    ///
    /// Uses `F` to measure subsets of the slice.
    ///
    /// Rounds towards the start of the document.
    ///
    /// # Warning
    /// Assumes `self` is single-line slice and is not wrapped.
    fn find_pos<F>(&self, point: f32, measure: F) -> usize
    where
        F: FnMut(&str) -> f32;

    /// Get the range of the slice within self
    ///
    /// # Panics
    /// This method may panic if `slice` is not a subset of self
    fn subset_range(&self, slice: &str) -> Range<usize>;
}

impl Document for str {
    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn line_count(&self) -> usize {
        self.lines().count()
    }

    #[inline]
    fn as_slice(&self) -> &str {
        self
    }

    fn next_char(&self, pos: usize) -> usize {
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        debug_assert!(self.is_char_boundary(pos), "pos should be at a valid char");
        self[pos..]
            .chars()
            .next()
            .map_or(self.len(), |ch| pos + ch.len_utf8())
    }

    fn prev_char(&self, pos: usize) -> usize {
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        debug_assert!(self.is_char_boundary(pos), "pos should be at a valid char");
        self[..pos].char_indices().next_back().map_or(0, |(i, _)| i)
    }

    fn start_of(&self, pos: usize, delim: char) -> usize {
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        match self[..pos].rfind(delim) {
            Some(i) => i + delim.len_utf8(),
            None => 0,
        }
    }

    fn end_of(&self, pos: usize, delim: char) -> usize {
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        match self[pos..].find(delim) {
            Some(i) => i + pos,
            None => self.len(),
        }
    }

    fn line_index(&self, pos: usize) -> usize {
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        self[..pos].matches('\n').count()
    }

    fn word_start(&self, pos: usize) -> usize {
        const {
            assert!(' '.len_utf8() == '\n'.len_utf8());
        }
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        match self[..pos].rfind(const { [' ', '\n'].as_slice() }) {
            Some(i) => i + const { ' '.len_utf8() },
            None => 0,
        }
    }

    fn word_end(&self, pos: usize) -> usize {
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        match self[pos..].find(const { [' ', '\n'].as_slice() }) {
            Some(i) => i + pos,
            None => self.len(),
        }
    }

    fn find_line_slice(&self, line_height: f32, point: f32, offset: usize) -> &str {
        self.lines()
            .nth(self.find_line(line_height, point, offset))
            .unwrap_or(&self[self.len()..])
    }

    // TODO: Could this be converted to binary search?
    fn find_pos<F>(&self, point: f32, mut measure: F) -> usize
    where
        F: FnMut(&str) -> f32,
    {
        debug_assert!(!self.contains('\n'), "find_pos expects a single line");
        self.char_indices()
            .map(|(idx, _)| idx)
            // find the character overlapping the point
            // i.e. the first character that is further right than the point
            .find(|&idx| point <= measure(&self[..self.next_char(idx)]))
            // // round to the closer boundary
            // .map(|idx| {
            //     let next_idx = self.next_char(idx);
            //     point > measure(&self[idx..next_idx])
            // })
            .unwrap_or(self.len())
    }

    #[inline]
    fn subset_range(&self, slice: &str) -> Range<usize> {
        self.substr_range(slice)
            .expect("slice should be a subset of self")
    }
}

impl<T> Document for T
where
    T: Deref<Target = str>,
{
    #[inline]
    fn len(&self) -> usize {
        self.deref().len()
    }

    #[inline]
    fn as_slice(&self) -> &str {
        self.deref().as_slice()
    }

    #[inline]
    fn next_char(&self, pos: usize) -> usize {
        self.deref().next_char(pos)
    }

    #[inline]
    fn prev_char(&self, pos: usize) -> usize {
        self.deref().prev_char(pos)
    }

    #[inline]
    fn line_count(&self) -> usize {
        self.deref().line_count()
    }

    #[inline]
    fn start_of(&self, pos: usize, delim: char) -> usize {
        self.deref().start_of(pos, delim)
    }

    #[inline]
    fn end_of(&self, pos: usize, delim: char) -> usize {
        self.deref().end_of(pos, delim)
    }

    #[inline]
    fn line_index(&self, pos: usize) -> usize {
        self.deref().line_index(pos)
    }

    #[inline]
    fn find_line_slice(&self, line_height: f32, point: f32, offset: usize) -> &str {
        self.deref().find_line_slice(line_height, point, offset)
    }

    #[inline]
    fn find_pos<F>(&self, point: f32, measure: F) -> usize
    where
        F: FnMut(&str) -> f32,
    {
        self.deref().find_pos(point, measure)
    }

    #[inline]
    fn subset_range(&self, slice: &str) -> Range<usize> {
        self.deref().subset_range(slice)
    }
}

/// A [`Document`] that can be appended
pub trait EditableDocument: Document {
    /// Replace a slice of a document with another slice of possibly different size
    ///
    /// Replacing a non-empty range with an empty slice erases the range
    ///
    /// Replacing an empty range with a non-empty slice inserts the range
    fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>;

    /// Equivalent to replacing the range with an empty slice
    fn erase_range<R>(&mut self, range: R)
    where
        R: RangeBounds<usize>,
    {
        self.replace_range(range, const { "" });
    }
}

impl EditableDocument for String {
    #[inline]
    fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>,
    {
        self.replace_range(range, replace_with);
    }
}

/// The range of bytes selected by the cursor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Selection {
    /// Where the selection was initially placed
    pub head: usize,
    /// The part of the selection that moves with the arrow keys
    pub tail: usize,
}

impl Selection {
    /// The front side of the range
    pub const fn start(&self) -> &usize {
        if self.tail < self.head {
            &self.tail
        } else {
            &self.head
        }
    }

    /// The back side of the range
    pub const fn end(&self) -> &usize {
        if self.tail < self.head {
            &self.head
        } else {
            &self.tail
        }
    }

    /// The number of bytes selected
    pub const fn len(&self) -> usize {
        self.head.abs_diff(self.tail)
    }

    /// Whether the selection is for insertion, rather than replacement
    pub const fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    /// Shorthand for `Range::from(self)`
    pub fn range(self) -> Range<usize> {
        self.into()
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

/// Parameters for measuring and displaying text
#[derive(Debug)]
pub struct TextStyle {
    /// The [`RaylibFont`] font the text is rendered in
    pub font: OilFont,
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

impl TextStyle {
    /// The distance between the top of the current line and the top of the next line
    pub const fn line_height(&self) -> f32 {
        self.font_size + self.line_space
    }

    /// The width, in pixels, of `text` using the `self` style
    pub fn text_width(&self, text: &str) -> f32 {
        self.font.measure_text(text, self.font_size, self.spacing)
    }
}

/// The number of lines of text that can fit into the content region a rectangle
/// that is `clip_height` pixels tall with `pad_y` pixels of padding
///
/// Assumes the padding is the same on the bottom as it is on the top
const fn fittable_lines(clip_height: f32, pad_y: f32, font_size: f32, line_space: f32) -> usize {
    // because the line space is blank, we can fit one extra outside of the clipping region
    ((clip_height - 2.0 * pad_y + line_space) / (font_size + line_space)) as usize
}

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

/// Repeat a [`KeyInput`] while it is held down
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyRepeater {
    /// The key to repeat and the timestamp of when it will be to repeat
    ///
    /// Only the most recently pressed key can be repeated
    pub input: Option<(KeyInput, Instant)>,
    /// How long to wait before repeating a key input after the initial press
    pub start_delay: Duration,
    /// How long to wait between key input repetitions after `start_delay`
    pub period: Duration,
}

impl Default for KeyRepeater {
    fn default() -> Self {
        Self::new(Duration::ZERO, Duration::ZERO)
    }
}

impl KeyRepeater {
    /// Construct a new [`KeyRepeater`] without an input initially
    pub const fn new(start_delay: Duration, period: Duration) -> Self {
        Self {
            input: None,
            start_delay,
            period,
        }
    }

    /// Indicate that a key has been pressed and replace the held key
    pub fn press(&mut self, input: KeyInput) {
        self.input = Some((input, Instant::now() + self.start_delay));
    }

    /// Indicate that a key has been released and clear the held key if it matches
    pub fn release(&mut self, input: &KeyInput) {
        if self.input.as_ref().is_some_and(|(key, _)| key == input) {
            self.input = None;
        }
    }

    /// Get the key input if there is one and this tick should repeat it
    pub fn check_repeat(&self) -> Option<KeyInput> {
        self.input.and_then(|(key, pressed)| {
            let now = Instant::now();
            now.checked_duration_since(pressed)
                .is_some_and(|duration| duration.as_nanos() % self.period.as_nanos() == 0)
                .then_some(key)
        })
    }
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
    /// (TODO: isn't this is dependent on [`TextStyle`]?)
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
    pub fn update<I>(&mut self, rl: &mut RaylibHandle, inputs: I, style: &TextStyle)
    where
        Doc: for<'a> EditableDocument,
        I: IntoIterator<Item = KeyInput>,
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
    pub fn draw<D>(&self, d: &mut D, style: &TextStyle)
    where
        D: RaylibDraw + std::ops::DerefMut<Target = RaylibHandle>,
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

fn main() {
    let (mut rl, thread) = init().title("Amitxt").resizable().build();

    let style = TextStyle {
        font: rl.get_font_default().into(),
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
