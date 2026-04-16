#![feature(substr_range, cmp_minmax)]

use raylib::prelude::{KeyboardKey::*, *};
use std::ops::{Bound, Range, RangeBounds, RangeInclusive};

trait Document {
    fn start_of(&self, pos: usize, delim: char) -> usize;
    fn end_of(&self, pos: usize, delim: char) -> usize;

    fn word_start(&self, pos: usize) -> usize {
        self.start_of(pos, ' ')
    }
    fn word_end(&self, pos: usize) -> usize {
        self.end_of(pos, ' ')
    }

    fn line_start(&self, pos: usize) -> usize {
        self.start_of(pos, '\n')
    }
    fn line_end(&self, pos: usize) -> usize {
        self.end_of(pos, '\n')
    }
    fn line_index(&self, pos: usize) -> usize;

    fn line_range(&self, range: Range<usize>) -> Range<usize> {
        self.line_start(range.start)..self.line_end(range.end)
    }

    fn line_indices(&self, range: Range<usize>) -> RangeInclusive<usize> {
        self.line_index(range.start)..=self.line_index(range.end)
    }
}

impl Document for str {
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
    head: usize,
    tail: usize,
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

fn main() {
    let mut document = String::new();
    let mut selection = Selection { head: 0, tail: 0 };
    let font_size = 20.0;
    let spacing = font_size / 10.0;
    let line_space = 2.0;
    let padding = Vector2::new(5.0, 5.0);
    let background_color = Color::new(27, 27, 27, 255);
    let foreground_color = Color::WHITE;
    let selection_color = Color::BLUEVIOLET;
    let min_selection_width = 2.0;
    let mut scroll: usize = 0;
    let wrap_width = 600;

    let (mut rl, thread) = init().title("Amitxt").resizable().build();
    let font = rl.get_font_default();

    let mut lines_per_screen =
        (rl.get_screen_height() as f32 - 2.0 * padding.y / (font_size + line_space)) as usize;

    while !rl.window_should_close() {
        let is_shifting = rl.is_key_down(KEY_LEFT_SHIFT) || rl.is_key_down(KEY_RIGHT_SHIFT);
        let is_ctrling = rl.is_key_down(KEY_LEFT_CONTROL) || rl.is_key_down(KEY_RIGHT_CONTROL);
        let is_alting = rl.is_key_down(KEY_LEFT_ALT) || rl.is_key_down(KEY_RIGHT_ALT);

        let mut is_moved = false;
        if rl.is_key_pressed(KEY_RIGHT) {
            is_moved = true;
            selection.tail = if selection.is_empty() || is_shifting {
                if is_ctrling {
                    document.word_end(selection.tail.saturating_add(1).min(document.len()))
                } else {
                    selection.tail.saturating_add(1).min(document.len())
                }
            } else {
                *selection.end()
            };
        }
        if rl.is_key_pressed(KEY_LEFT) {
            is_moved = true;
            selection.tail = if selection.is_empty() || is_shifting {
                if is_ctrling {
                    document.word_start(selection.tail.saturating_sub(1))
                } else {
                    selection.tail.saturating_sub(1)
                }
            } else {
                *selection.start()
            };
        }
        if rl.is_key_pressed(KEY_END) {
            is_moved = true;
            selection.tail = if is_ctrling {
                document.len()
            } else {
                document.line_end(selection.tail)
            };
        }
        if rl.is_key_pressed(KEY_HOME) {
            is_moved = true;
            selection.tail = if is_ctrling {
                0
            } else {
                document.line_start(selection.tail)
            };
        }
        if is_moved && !is_shifting {
            selection.head = selection.tail;
        }

        if let Some(ch) = rl
            .get_char_pressed()
            .or_else(|| rl.is_key_pressed(KEY_ENTER).then_some('\n'))
        {
            let mut buf = [0; 4];
            document.replace_range(selection, ch.encode_utf8(&mut buf));
            selection.tail += ch.len_utf8();
            selection.head = selection.tail;
        } else if rl.is_key_pressed(KEY_BACKSPACE) {
            if selection.is_empty() {
                selection.tail = selection.tail.saturating_sub(1);
            }
            document.replace_range(selection, "");
            selection.head = selection.tail;
        } else if rl.is_key_pressed(KEY_DELETE) {
            if selection.is_empty() {
                selection.tail = selection.tail.saturating_add(1).min(document.len());
            }
            if *selection.start() < document.len() {
                document.replace_range(selection, "");
            }
            selection.head = selection.head.min(document.len());
            selection.tail = selection.head;
        }

        if rl.is_window_resized() {
            lines_per_screen = (rl.get_screen_height() as f32
                - 2.0 * padding.y / (font_size + line_space))
                as usize;
        }

        scroll = scroll
            .saturating_sub_signed(rl.get_mouse_wheel_move() as isize)
            .min(document.lines().count());

        let mut d = rl.begin_drawing(&thread);
        d.clear_background(background_color);

        let selection_lines = document.line_indices(selection.into());
        for (screen_linenum, line) in document
            .lines()
            .skip(scroll)
            .take(lines_per_screen)
            .enumerate()
        {
            let document_linenum = screen_linenum - scroll;
            let line_range = document
                .substr_range(line)
                .expect("all lines of document should be in document");
            if selection_lines.contains(&document_linenum) {
                let selection_range = Range::from(selection);
                let start = selection_range.start.saturating_sub(line_range.start);
                let end = selection_range
                    .end
                    .checked_sub(line_range.start)
                    .unwrap_or(line_range.len())
                    .min(line_range.len());
                let pre_width = selection_range
                    .start
                    .checked_sub(line_range.start)
                    .map_or(0.0, |start| {
                        font.measure_text(&line[..start], font_size, spacing).x
                    });
                let selected_width = font.measure_text(&line[start..end], font_size, spacing).x;
                d.draw_rectangle_rec(
                    Rectangle::new(
                        padding.x
                            + pre_width
                            + if selection_range.is_empty() {
                                -0.5 * min_selection_width
                            } else {
                                0.0
                            },
                        padding.y + (document_linenum - scroll) as f32 * (font_size + line_space),
                        (selected_width + spacing).max(min_selection_width),
                        font_size,
                    ),
                    selection_color,
                );
            }

            d.draw_text_pro(
                &font,
                line,
                padding + Vector2::new(0.0, screen_linenum as f32 * (font_size + line_space)),
                Vector2::zero(),
                0.0,
                font_size,
                spacing,
                foreground_color,
            );
        }
    }
}
