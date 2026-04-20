use crate::{style::Style, text::MeasureText};

pub struct WrapLine<'a, 'b, T> {
    /// The line currently being split into wrapping
    line: &'a str,
    /// The font, font_height, and spacing
    style: &'b Style<T>,
    /// The maximum width before wrapping
    wrap_width: f32,
}

impl<'a, 'b, T> WrapLine<'a, 'b, T> {
    fn new(line: &'a str, style: &'b Style<T>, wrap_width: f32) -> Self {
        debug_assert!(!line.contains('\n'));
        Self {
            line,
            style,
            wrap_width,
        }
    }
}

impl<'a, T> Iterator for WrapLine<'a, '_, T>
where
    T: MeasureText,
{
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.line.is_empty() {
            return None;
        }

        // find the first character that doesn't fit in the line
        if let Some((mid, _)) = self
            .line
            .char_indices()
            .map(|(i, _)| (i, self.style.text_width(&self.line[..i])))
            .find(|(_, w)| *w > self.wrap_width)
        {
            // find the start of the word that splits
            match self.line[..mid]
                .rmatch_indices(|ch: char| !ch.is_whitespace())
                .next()
            {
                Some((mid, splitter)) => {
                    let pre_wrap = &self.line[..mid];
                    self.line = &self.line[mid + splitter.len()..];
                    Some(pre_wrap)
                }
                None => {
                    let pre_wrap;
                    (pre_wrap, self.line) = self.line.split_at(mid);
                    Some(pre_wrap)
                }
            }
        } else {
            Some(self.line)
        }
    }
}

/// Iterator over lines in a string, including virtual lines created by wrapping.
///
/// Tries to wrap by word, then by character if a single word is wider than an entire line
#[derive(Debug, Clone)]
pub struct WrappedLines<'a, 'b, I, T> {
    /// Iterator over lines
    lines: I,
    /// The line currently being split into wrapping
    curr_line: Option<&'a str>,
    /// The font, font_height, and spacing
    style: &'b Style<T>,
    /// The maximum width before wrapping
    wrap_width: f32,
}

impl<'a, 'b, I, T> WrappedLines<'a, 'b, I, T> {
    /// Construct a new [`WrappedLines`] iterator
    const fn new(lines: I, style: &'b Style<T>, wrap_width: f32) -> Self {
        Self {
            lines,
            curr_line: None,
            style,
            wrap_width,
        }
    }
}

impl<'a, I, T> Iterator for WrappedLines<'a, '_, I, T>
where
    I: Iterator<Item = &'a str>,
    T: MeasureText,
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
    fn wrapped_lines<'a, 'b, T>(
        &'a self,
        style: &'b Style<T>,
        wrap_width: f32,
    ) -> WrappedLines<'a, 'b, std::str::Lines<'a>, T>
    where
        T: MeasureText;
}

impl WrappedLinesExt for str {
    #[inline]
    fn wrapped_lines<'a, 'b, T>(
        &'a self,
        style: &'b Style<T>,
        wrap_width: f32,
    ) -> WrappedLines<'a, 'b, std::str::Lines<'a>, T>
    where
        T: MeasureText,
    {
        WrappedLines::new(self.lines(), style, wrap_width)
    }
}
