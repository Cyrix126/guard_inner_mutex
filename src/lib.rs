use std::ops::{Deref, DerefMut};

use parking_lot::{MappedMutexGuard, MutexGuard};

/// Generic struct as a front end put to the inner struct
pub struct InnerGuard<'a, I>(MutexGuard<'a, I>);

impl<'a, I> Deref for InnerGuard<'a, I> {
    type Target = I;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'a, I> DerefMut for InnerGuard<'a, I> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Allow referencing fields as gated mutex.
/// The nice bonus is that we don't even need to set two
/// method in traits to get &T and &mut T
/// example:
///
/// definition in trait:
/// fn output(&self) -> FieldGuard<'_, String>;
///
/// implementation:
/// fn output(&self) -> FieldGuard<'_, String> {
///     FieldGuard(MutexGuard::map(self.inner.lock(), |inner| {
///         &mut inner.gui_api.output
///     }))
/// }
pub struct FieldGuard<'a, T>(pub MappedMutexGuard<'a, T>);
impl<'a, T> Deref for FieldGuard<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<'a, T> DerefMut for FieldGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Simply returns the inner wrapped into an InnerGuard
/// Example: InnerGuard(self.inner.lock().unwrap())
pub trait InnerGuarded<I> {
    fn lock(&self) -> InnerGuard<'_, I>;
}
