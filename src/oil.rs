//! Owned If Loaded

use std::{fmt, ops::*};

/// The ability to manually unload/deallocate a resource
pub trait Unload<D> {
    /// The error type that can arise from failing to deallocate
    type Error;

    /// Manually unload `self` using the deallocator
    fn unload(self, deallocator: D) -> Result<(), Self::Error>;
}

/// Owned If Loaded
///
/// Tracks responsibility of Drop
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Oil<T, U> {
    /// An owned instance that must be unloaded when this enum is dropped.
    Strong(T),
    /// An unowned instance that will be unloaded independently.
    Weak(U),
}
pub use Oil::*;

impl<T, U> fmt::Debug for Oil<T, U>
where
    T: fmt::Debug,
    U: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Weak(ref b) => fmt::Debug::fmt(b, f),
            Strong(ref o) => fmt::Debug::fmt(o, f),
        }
    }
}

impl<T, U> fmt::Display for Oil<T, U>
where
    T: fmt::Display,
    U: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Weak(ref b) => fmt::Display::fmt(b, f),
            Strong(ref o) => fmt::Display::fmt(o, f),
        }
    }
}

impl<T, U: [const] Default> const Default for Oil<T, U> {
    fn default() -> Self {
        Self::Weak(Default::default())
    }
}

impl<T, U, Q> Deref for Oil<T, U>
where
    T: Deref<Target = Q>,
    U: Deref<Target = Q>,
{
    type Target = Q;

    fn deref(&self) -> &Q {
        match self {
            Oil::Strong(x) => x.deref(),
            Oil::Weak(x) => x.deref(),
        }
    }
}

impl<T, U, Q> DerefMut for Oil<T, U>
where
    T: DerefMut<Target = Q>,
    U: DerefMut<Target = Q>,
{
    fn deref_mut(&mut self) -> &mut Q {
        match self {
            Oil::Strong(x) => x.deref_mut(),
            Oil::Weak(x) => x.deref_mut(),
        }
    }
}

impl<T, U, Q> AsRef<Q> for Oil<T, U>
where
    T: AsRef<Q>,
    U: AsRef<Q>,
{
    fn as_ref(&self) -> &Q {
        match self {
            Oil::Strong(x) => x.as_ref(),
            Oil::Weak(x) => x.as_ref(),
        }
    }
}

impl<T, U, Q> AsMut<Q> for Oil<T, U>
where
    T: AsMut<Q>,
    U: AsMut<Q>,
{
    fn as_mut(&mut self) -> &mut Q {
        match self {
            Oil::Strong(x) => x.as_mut(),
            Oil::Weak(x) => x.as_mut(),
        }
    }
}

impl<T, U, D> Unload<D> for Oil<T, U>
where
    U: Unload<D>,
{
    type Error = U::Error;

    fn unload(self, deallocator: D) -> Result<(), Self::Error> {
        match self {
            Strong(_) => Ok(()),
            Weak(weak) => weak.unload(deallocator),
        }
    }
}
