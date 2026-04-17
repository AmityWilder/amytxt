//! Notepad without AI

#![feature(
    substr_range,
    cmp_minmax,
    const_iter,
    // const_array,
    const_bool,
    const_cmp,
    const_clone,
    const_convert,
    const_default,
    // const_for,
    const_index,
    // const_closures,
    // const_control_flow,
    const_format_args,
    // const_ops,
    const_range,
    // const_option_ops,
    // const_destruct,
    // const_path_separators,
    // const_range_bounds,
    const_trait_impl,
    // const_result_unwrap_unchecked,
    derive_const,
    // const_slice_make_iter,
    min_specialization
)]
#![warn(
    clippy::missing_const_for_fn,
    missing_docs,
    clippy::multiple_unsafe_ops_per_block,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    clippy::missing_safety_doc,
    clippy::undocumented_unsafe_blocks,
    clippy::missing_panics_doc,
    clippy::missing_docs_in_private_items
)]

use raylib::prelude::{KeyboardKey::*, *};
use std::ops::*;

/// Types that can be treated as characters in text
pub const trait Character: Sized + Copy + [const] Ord {
    /// This type's equivalent of `' '`
    const SPACE: Self;

    /// This type's equivalent of `'\n'`
    const NEWLINE: Self;

    /// The maximum buffer size needed to encode any character
    const MAX_SIZE: usize;

    /// The number of bytes needed to encode the character
    #[inline]
    fn size(self) -> usize {
        const { std::mem::size_of::<Self>() }
    }
}

impl const Character for char {
    const SPACE: Self = ' ';
    const NEWLINE: Self = '\n';
    const MAX_SIZE: usize = char::MAX_LEN_UTF8;

    #[inline]
    fn size(self) -> usize {
        self.len_utf8()
    }
}
impl const Character for u8 {
    const SPACE: Self = b' ';
    const NEWLINE: Self = b'\n';
    const MAX_SIZE: usize = std::mem::size_of::<Self>();
}
impl const Character for std::ffi::c_char {
    const SPACE: Self = b' ' as Self;
    const NEWLINE: Self = b'\n' as Self;
    const MAX_SIZE: usize = std::mem::size_of::<Self>();
}

/// A subset of an array
///
/// Does not need to be [`Sized`], because [`Index`] will always return a reference to it
pub const trait Slice<'a>: 'a + [const] Index<Range<usize>, Output = Self> {
    /// An empty slice, allowing [`EditableDocument`] to use it for erasing
    fn empty() -> &'a Self;
}
impl<'a> const Slice<'a> for str {
    fn empty() -> &'a Self {
        const { Default::default() }
    }
}
impl<'a, T: 'a> const Slice<'a> for [T] {
    fn empty() -> &'a Self {
        const { Default::default() }
    }
}

/// Any form of text that can be selected and edited like a Word document
pub const trait Document {
    /// The character type for the implementation
    type Char: [const] Character;

    /// The type given by indexing the document.
    ///
    /// While a document may output slices of a different type from itself (e.g. String -> str),
    /// the slice should always output its own type (str -> str).
    type Slice<'a>: ?Sized + [const] Slice<'a> + [const] Document<Slice<'a> = Self::Slice<'a>>;

    /// An iterator over lines of text in a document
    type Lines<'a>: [const] Iterator<Item = &'a Self::Slice<'a>>
    where
        Self: 'a;

    /// Convert a [`Char`] into a [`Slice`] for the purpose of insertion through [`replace_range`]
    ///
    /// [`Char`]: Self::Char
    /// [`Slice`]: Self::Slice
    /// [`replace_range`]: EditableDocument::replace_range
    fn char_to_slice(ch: Self::Char, buf: &mut [u8]) -> &mut Self::Slice<'_>;

    /// The range of values that are safe to index into
    fn len(&self) -> usize;

    /// Whether the document or slice contains no elements
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over lines of text in the document
    fn lines(&self) -> Self::Lines<'_>;

    /// Returns the number of lines in the document
    fn line_count(&self) -> usize;

    /// Gives a [`Self::Slice`] covering the entire document, which can then be range-indexed
    fn as_slice(&self) -> &Self::Slice<'_>;

    /// Returns the start position of the character following the one starting at `pos`, clamped to the document bounds
    fn next_char(&self, pos: usize) -> usize;

    /// Returns the start position of the character preceeding the one starting at `pos`, clamped to the document bounds
    fn prev_char(&self, pos: usize) -> usize;

    /// Gives the position following the last instance of `delim` at or before `pos`.
    fn start_of(&self, pos: usize, delim: Self::Char) -> usize;

    /// Gives the position of the first instance of `delim` at or after `pos`.
    fn end_of(&self, pos: usize, delim: Self::Char) -> usize;

    /// Counts the number of newlines before `pos`.
    fn line_index(&self, pos: usize) -> usize;

    /// Gives the position following the last space or newline at or before `pos`.
    fn word_start(&self, pos: usize) -> usize {
        self.start_of(pos, Self::Char::SPACE)
            .max(self.line_start(pos))
    }

    /// Gives the position of the first space or newline at or after `pos`.
    fn word_end(&self, pos: usize) -> usize {
        self.end_of(pos, Self::Char::SPACE).min(self.line_end(pos))
    }

    /// Gives the position following the last newline at or before `pos`.
    fn line_start(&self, pos: usize) -> usize {
        self.start_of(pos, Self::Char::NEWLINE)
    }

    /// Gives the position of the first newline at or after `pos`.
    fn line_end(&self, pos: usize) -> usize {
        self.end_of(pos, Self::Char::NEWLINE)
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
    type Char = char;
    type Slice<'a> = str;
    type Lines<'a>
        = std::str::Lines<'a>
    where
        Self: 'a;

    #[inline]
    fn char_to_slice(ch: char, buf: &mut [u8]) -> &mut str {
        ch.encode_utf8(buf)
    }

    #[inline]
    fn len(&self) -> usize {
        self.len()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    #[inline]
    fn lines(&self) -> Self::Lines<'_> {
        self.lines()
    }

    #[inline]
    fn line_count(&self) -> usize {
        self.lines().count()
    }

    #[inline]
    fn as_slice(&self) -> &Self::Slice<'_> {
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
}

impl<T, U: ?Sized> const Document for T
where
    T: [const] Deref<Target = U>,
    for<'a> U: [const] Slice<'a, Output = U> + [const] Document<Slice<'a> = U>,
{
    type Char = U::Char;
    type Slice<'a> = U::Slice<'a>;
    type Lines<'a>
        = U::Lines<'a>
    where
        Self: 'a;

    #[inline]
    fn char_to_slice(ch: Self::Char, buf: &mut [u8]) -> &mut Self::Slice<'_> {
        U::char_to_slice(ch, buf)
    }

    #[inline]
    fn len(&self) -> usize {
        self.deref().len()
    }

    #[inline]
    fn as_slice(&self) -> &Self::Slice<'_> {
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
    fn lines(&self) -> Self::Lines<'_> {
        self.deref().lines()
    }

    #[inline]
    fn line_count(&self) -> usize {
        self.deref().line_count()
    }

    #[inline]
    fn start_of(&self, pos: usize, delim: Self::Char) -> usize {
        self.deref().start_of(pos, delim)
    }

    #[inline]
    fn end_of(&self, pos: usize, delim: Self::Char) -> usize {
        self.deref().end_of(pos, delim)
    }

    #[inline]
    fn line_index(&self, pos: usize) -> usize {
        self.deref().line_index(pos)
    }
}

/// A [`Document`] that can be appended
pub const trait EditableDocument: [const] Document {
    /// Replace a slice of a document with another slice of possibly different size
    ///
    /// Replacing a non-empty range with an empty slice erases the range
    ///
    /// Replacing an empty range with a non-empty slice inserts the range
    fn replace_range<R>(&mut self, range: R, replace_with: &Self::Slice<'_>)
    where
        R: [const] RangeBounds<usize>;

    /// Equivalent to replacing the range with an empty slice
    fn erase_range<R>(&mut self, range: R)
    where
        R: [const] RangeBounds<usize>,
    {
        self.replace_range(range, Self::Slice::empty());
    }
}

impl EditableDocument for String {
    #[inline]
    fn replace_range<R>(&mut self, range: R, replace_with: &Self::Slice<'_>)
    where
        R: RangeBounds<usize>,
    {
        self.replace_range(range, replace_with);
    }
}

/// The range of bytes selected by the cursor
#[derive(Debug, Copy, Hash)]
#[derive_const(Clone, PartialEq, Eq, Default)]
pub struct Selection {
    /// Where the selection was initially placed
    pub head: usize,
    /// The part of the selection that moves with the arrow keys
    pub tail: usize,
}

impl Selection {
    /// The front side of the range
    pub const fn start(&self) -> &usize {
        (&self.head).min(&self.tail)
    }

    /// The back side of the range
    pub const fn end(&self) -> &usize {
        (&self.head).max(&self.tail)
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
    pub const fn range(self) -> Range<usize> {
        self.into()
    }
}

impl const IntoIterator for Selection {
    type Item = usize;
    type IntoIter = Range<usize>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.into()
    }
}

impl const From<Selection> for Range<usize> {
    fn from(value: Selection) -> Self {
        let [start, end] = std::cmp::minmax(value.head, value.tail);
        start..end
    }
}

impl const RangeBounds<usize> for Selection {
    fn start_bound(&self) -> Bound<&usize> {
        Bound::Included(self.start())
    }

    fn end_bound(&self) -> Bound<&usize> {
        Bound::Excluded(self.end())
    }

    #[inline]
    fn contains<U>(&self, item: &U) -> bool
    where
        usize: [const] PartialOrd<U>,
        U: ?Sized + [const] PartialOrd<usize>,
    {
        Range::from(*self).contains(item)
    }
}

impl const std::ops::Index<Selection> for str {
    type Output = str;

    #[inline]
    fn index(&self, index: Selection) -> &Self::Output {
        self.index(Range::from(index))
    }
}

/// Parameters for measuring and displaying text
#[derive(Debug)]
pub struct TextStyle<T> {
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

impl<T> const Default for TextStyle<T>
where
    T: [const] Default,
{
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: Default::default(),
            spacing: Default::default(),
            line_space: Default::default(),
            background_color: Color::BLANK,
            foreground_color: Color::BLANK,
            selection_color: Color::BLANK,
            min_selection_width: Default::default(),
        }
    }
}

impl<T> TextStyle<T> {
    /// The distance between the top of the current line and the top of the next line
    pub const fn line_height(&self) -> f32 {
        self.font_size + self.line_space
    }
}

impl<T: RaylibFont> TextStyle<T> {
    /// The width, in pixels, of `text` using the `self` style
    pub fn text_width(&self, text: &str) -> f32 {
        self.font.measure_text(text, self.font_size, self.spacing).x
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
#[derive(Debug, Copy, Hash)]
#[derive_const(Clone, PartialEq, Eq)]
pub enum KeyInput<Ch = char> {
    /// A character being typed (includes enter/return as `'\n'`)
    Char(Ch),
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
}

impl<Doc: [const] Default> const Default for TextEditor<Doc> {
    fn default() -> Self {
        Self {
            content: Default::default(),
            scroll: Default::default(),
            selection: Default::default(),
            clip: Rectangle::new(0.0, 0.0, 0.0, 0.0),
            pad: Vector2::zero(),
            fittable_lines: Default::default(),
            wrap: Default::default(),
        }
    }
}

impl<Doc> TextEditor<Doc> {
    /// Construct a new [`TextEditor`] with a blank document
    pub const fn new(
        clip: Rectangle,
        pad: Vector2,
        wrap: u32,
        font_size: f32,
        line_space: f32,
    ) -> Self
    where
        Doc: [const] Default,
    {
        Self {
            content: Doc::default(),
            scroll: 0,
            selection: Selection { head: 0, tail: 0 },
            clip,
            pad,
            fittable_lines: fittable_lines(clip.height, pad.y, font_size, line_space),
            wrap,
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
    pub fn update<I>(&mut self, rl: &mut RaylibHandle, inputs: I)
    where
        Doc: EditableDocument,
        I: IntoIterator<Item = KeyInput<Doc::Char>>,
    {
        let is_shifting = rl.is_key_down(KEY_LEFT_SHIFT) || rl.is_key_down(KEY_RIGHT_SHIFT);
        let is_ctrling = rl.is_key_down(KEY_LEFT_CONTROL) || rl.is_key_down(KEY_RIGHT_CONTROL);
        let _is_alting = rl.is_key_down(KEY_LEFT_ALT) || rl.is_key_down(KEY_RIGHT_ALT);

        for input in inputs {
            debug_assert!(
                *self.selection.end() <= self.content.len(),
                "the selection should always be within bounds"
            );
            match input {
                KeyInput::Char(ch) => {
                    // TODO: implement letter-combo hotkeys like ctrl+c/ctrl+v here
                    self.content
                        .replace_range(self.selection, Doc::char_to_slice(ch, &mut [0; 4]));
                    self.selection.tail += ch.size();
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
    pub fn draw<D, T>(&self, d: &mut D, style: &TextStyle<T>)
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
        style.font_size,
        style.line_space,
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
        document.update(&mut rl, input_buffer);

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(style.background_color);

        document.draw(&mut d, &style);
    }
}
