//! Contains types for reflecting over CVars statically and dynamically.

use std::any::TypeId;

use bevy_ecs::system::Resource;
use bevy_reflect::{FromType, PartialReflect};

use crate::{CVarError, CVarFlags};

/// Static meta information about a cvar, like its contained type and path.
pub trait CVarMeta: Resource + std::ops::Deref<Target = Self::Inner> {
    /// The inner type of the CVar.
    type Inner: std::fmt::Debug;
    /// The path of the CVar within the config.
    const CVAR_PATH: &'static str;
    /// The flags applied to this CVar.
    fn flags() -> CVarFlags;
}

/// Provides bevy reflection metadata for CVars.
#[derive(Clone)]
pub struct ReflectCVar {
    reflect_inner: for<'a> fn(&'a dyn PartialReflect) -> Result<&'a dyn PartialReflect, CVarError>,
    reflect_inner_mut:
        for<'a> fn(&'a mut dyn PartialReflect) -> Result<&'a mut dyn PartialReflect, CVarError>,
    inner_type: TypeId,
    path: &'static str,
    flags: CVarFlags,
}

impl ReflectCVar {
    /// Returns the inner type (i.e. value type) of the CVar.
    pub fn inner_type(&self) -> TypeId {
        self.inner_type
    }

    /// Returns the path of the CVar.
    pub fn cvar_path(&self) -> &'static str {
        self.path
    }

    /// Returns the CVar's flags.
    pub fn flags(&self) -> CVarFlags {
        self.flags
    }

    /// Reflect over the inner value of the CVar, returning a reference to it.
    pub fn reflect_inner<'a>(
        &self,
        cvar: &'a dyn PartialReflect,
    ) -> Result<&'a dyn PartialReflect, CVarError> {
        (self.reflect_inner)(cvar)
    }

    /// Reflect over the inner value of the CVar, returning a mutable reference to it.
    pub fn reflect_inner_mut<'a>(
        &self,
        cvar: &'a mut dyn PartialReflect,
    ) -> Result<&'a mut dyn PartialReflect, CVarError> {
        (self.reflect_inner_mut)(cvar)
    }

    /// Apply a reflection
    pub fn reflect_apply(
        &self,
        cvar: &mut dyn PartialReflect,
        value: &dyn PartialReflect,
    ) -> Result<(), CVarError> {
        let inner_mut = self.reflect_inner_mut(cvar)?;

        inner_mut.try_apply(value)?;
        Ok(())
    }
}

impl<T: CVarMeta> FromType<T> for ReflectCVar {
    fn from_type() -> Self {
        ReflectCVar {
            inner_type: std::any::TypeId::of::<T::Inner>(),
            reflect_inner: |r| {
                r.reflect_ref()
                    .as_tuple_struct()
                    .map_err(|_| CVarError::BadCVarType)?
                    .field(0)
                    .ok_or(CVarError::BadCVarType)
            },

            reflect_inner_mut: |r| {
                r.reflect_mut()
                    .as_tuple_struct()
                    .map_err(|_| CVarError::BadCVarType)?
                    .field_mut(0)
                    .ok_or(CVarError::BadCVarType)
            },

            path: T::CVAR_PATH,
            flags: T::flags(),
        }
    }
}
