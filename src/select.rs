use std::ops::*;

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
