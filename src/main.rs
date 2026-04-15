#![feature(substr_range, cmp_minmax)]

use raylib::prelude::{KeyboardKey::*, *};
use std::ops::{Bound, Range, RangeBounds, RangeInclusive};

trait Document {
    fn line_start(&self, pos: usize) -> usize;
    fn line_end(&self, pos: usize) -> usize;
    fn line_index(&self, pos: usize) -> usize;

    fn line_range(&self, range: Range<usize>) -> Range<usize> {
        self.line_start(range.start)..self.line_end(range.end)
    }

    fn line_indices(&self, range: Range<usize>) -> RangeInclusive<usize> {
        self.line_index(range.start)..=self.line_index(range.end)
    }
}

impl Document for str {
    fn line_start(&self, pos: usize) -> usize {
        self[..pos.min(self.len())]
            .rfind('\n')
            .map(|i| i + '\n'.len_utf8())
            .unwrap_or(0)
    }

    fn line_end(&self, pos: usize) -> usize {
        self[pos.min(self.len())..]
            .find('\n')
            .map(|i| i + pos)
            .unwrap_or(self.len())
    }

    fn line_index(&self, pos: usize) -> usize {
        self[..pos.min(self.len())].matches('\n').count()
    }
}

impl<T: ?Sized + Document> Document for &T {
    #[inline]
    fn line_start(&self, pos: usize) -> usize {
        (*self).line_start(pos)
    }

    #[inline]
    fn line_end(&self, pos: usize) -> usize {
        (*self).line_end(pos)
    }

    #[inline]
    fn line_index(&self, pos: usize) -> usize {
        (*self).line_index(pos)
    }

    #[inline]
    fn line_range(&self, range: Range<usize>) -> Range<usize> {
        (*self).line_range(range)
    }

    #[inline]
    fn line_indices(&self, range: Range<usize>) -> RangeInclusive<usize> {
        (*self).line_indices(range)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Selection {
    head: usize,
    tail: usize,
}

impl Selection {
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
        Bound::Included((&self.head).min(&self.tail))
    }

    fn end_bound(&self) -> Bound<&usize> {
        Bound::Excluded((&self.head).max(&self.tail))
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
    let mut document = String::from("hello world");
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
            selection.tail = selection.tail.saturating_add(1);
        }
        if rl.is_key_pressed(KEY_LEFT) {
            is_moved = true;
            selection.tail = selection.tail.saturating_sub(1);
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

        if let Some(ch) = rl.get_char_pressed() {
            let mut buf = [0; 4];
            document.replace_range(selection, ch.encode_utf8(&mut buf));
            selection.tail += ch.len_utf8();
            selection.head = selection.tail;
        } else if rl.is_key_pressed(KEY_BACKSPACE) {
            if selection.is_empty() {
                selection.tail -= 1;
            }
            document.replace_range(selection, "");
            selection.head = selection.tail;
        } else if rl.is_key_pressed(KEY_DELETE) {
            if selection.is_empty() {
                selection.tail += 1;
            }
            document.replace_range(selection, "");
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
