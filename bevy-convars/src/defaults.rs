//! Helpers for working with the default values of CVars.

use bevy_ecs::{
    change_detection::{DetectChanges as _, DetectChangesMut as _},
    system::{Res, ResMut},
    world::{Mut, Ref},
};

use crate::reflect::CVarMeta;

trait Sealed {}
impl<T: CVarMeta> Sealed for Ref<'_, T> {}
impl<T: CVarMeta> Sealed for Res<'_, T> {}
impl<T: CVarMeta> Sealed for Mut<'_, T> {}
impl<T: CVarMeta> Sealed for ResMut<'_, T> {}

/// Extension trait to add default value detection methods to CVar refs.
#[allow(private_bounds)]
pub trait IsDefault: Sealed {
    /// Returns whether or not the value is the default one.
    fn is_default(&self) -> bool;
}

/// Extension trait to allow flagging a CVar as being its default value, even if modified.
pub trait IsDefaultMut: IsDefault {
    /// Set the current value to appear as the default one.
    /// This does not modify the actual default value for the type, only trigger change detection such that the current value is treated as default.
    fn set_is_default(&mut self);

    /// Reset the value to its default.
    fn reset_to_default(&mut self);
}

// Deliberately conservative implementations.
impl<T: CVarMeta> IsDefault for Ref<'_, T> {
    fn is_default(&self) -> bool {
        self.added() == self.last_changed()
    }
}

impl<T: CVarMeta> IsDefault for Res<'_, T> {
    fn is_default(&self) -> bool {
        self.added() == self.last_changed()
    }
}

impl<T: CVarMeta> IsDefault for ResMut<'_, T> {
    fn is_default(&self) -> bool {
        self.added() == self.last_changed()
    }
}

impl<T: CVarMeta> IsDefault for Mut<'_, T> {
    fn is_default(&self) -> bool {
        self.added() == self.last_changed()
    }
}

impl<T: CVarMeta> IsDefaultMut for Mut<'_, T> {
    fn set_is_default(&mut self) {
        self.set_added();
    }

    fn reset_to_default(&mut self) {
        T::set_to_default(self.as_mut());
        self.set_added();
    }
}

impl<T: CVarMeta> IsDefaultMut for ResMut<'_, T> {
    fn set_is_default(&mut self) {
        self.set_added();
    }

    fn reset_to_default(&mut self) {
        T::set_to_default(self.as_mut());
        self.set_added();
    }
}
