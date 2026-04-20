use raylib::prelude::*;
use std::ops::*;

pub mod edit;
pub mod wrap;

/// Provides a native Rust implementation of [`ffi::MeasureTextEx`]
pub trait MeasureText {
    /// Returns [`None`] if `ch` has negative width (like newline)
    fn measure_base_char(&self, ch: char) -> Option<f32>;

    /// Assumes text is one line and does not wrap
    fn measure_str(&self, text: &str, font_size: f32, spacing: f32) -> f32;
}

impl<T: RaylibFont> MeasureText for T {
    /// Returns [`None`] if `ch` has negative width (like newline)
    fn measure_base_char(&self, ch: char) -> Option<f32> {
        (ch != '\n').then(|| {
            let index = usize::try_from(self.get_glyph_index(ch)).unwrap();
            let font: &ffi::Font = self.as_ref();
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
    fn measure_str(&self, text: &str, font_size: f32, spacing: f32) -> f32 {
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
        match self[..pos].rmatch_indices(char::is_whitespace).next() {
            Some((i, s)) => i + s.len(),
            None => 0,
        }
    }

    fn word_end(&self, pos: usize) -> usize {
        debug_assert!(pos <= self.len(), "pos should be in bounds");
        match self[pos..].find(char::is_whitespace) {
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
