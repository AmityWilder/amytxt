use super::Document;
use std::ops::*;

/// A [`Document`] that can be appended
pub trait EditableDocument: DerefMut<Target: Document> {
    /// Replace a slice of a document with another slice of possibly different size
    ///
    /// Replacing a non-empty range with an empty slice erases the range
    ///
    /// Replacing an empty range with a non-empty slice inserts the range
    fn replace_range<R>(&mut self, range: R, replace_with: &str)
    where
        R: RangeBounds<usize>;

    /// Equivalent to replacing the range with an empty slice
    #[inline]
    fn erase_range<R>(&mut self, range: R)
    where
        R: RangeBounds<usize>,
    {
        self.replace_range(range, const { "" });
    }

    /// Equivalent to replacing the range with an empty slice
    #[inline]
    fn insert_at(&mut self, index: usize, what: &str) {
        self.replace_range(index..index, what);
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
