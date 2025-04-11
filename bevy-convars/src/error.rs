use std::fmt::Display;

use bevy_ecs::world::error::ResourceFetchError;
use bevy_reflect::ApplyError;
#[cfg(feature = "parse_cvars")]
use toml_edit::TomlError;

/// Errors that can occur when manipulating CVars.
#[derive(Debug)]
#[non_exhaustive]
pub enum CVarError {
    /// Error indicating a CVar was never registered or is invalid.
    UnknownCVar,
    /// Error indicating the given CVar type is invalid.
    BadCVarType,
    /// Error indicating the CVar type is missing a [ComponentId](bevy_ecs::component::ComponentId) and is likely not registered correctly.
    MissingCid,
    /// Error indicating the underlying type of the CVar cannot be deserialized, and as such cannot be reflected over.
    CannotDeserialize,
    /// Error indicating the CVar failed to deserialize.
    FailedDeserialize(String),
    /// Error indicating applying a value to the CVar failed, containing the inner error.
    FailedApply {
        /// The inner error.
        inner: ApplyError,
    },
    /// Error indicating that the world could not fulfill the requested operation due to an access conflict with an ongoing operation.
    AccessConflict,
    #[cfg(feature = "parse_cvars")]
    /// An error when parsing a TOML document.
    TomlError(TomlError),
}

impl std::error::Error for CVarError {}

impl Display for CVarError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CVarError::UnknownCVar => write!(f, "Unknown CVar."),
            CVarError::BadCVarType => write!(
                f,
                "CVar is not internally a Tuple Struct of the expected layout, did you try to register it manually?"
            ),
            CVarError::MissingCid => write!(f, "Missing ComponentID, was the resource registered?"),
            CVarError::CannotDeserialize => {
                write!(f, "Underlying CVar type cannot be deserialized.")
            }
            CVarError::FailedDeserialize(inner) => write!(f, "Failed to deserialize: {inner}"),
            CVarError::FailedApply { inner } => {
                write!(f, "Failed to apply value to CVar. ({inner:?})")
            }
            CVarError::AccessConflict => write!(
                f,
                "The requested operation conflicts with another ongoing operation on the world and cannot be performed."
            ),
            #[cfg(feature = "parse_cvars")]
            CVarError::TomlError(toml_error) => write!(f, "TOML parsing error: {toml_error}"),
        }
    }
}

impl From<ApplyError> for CVarError {
    fn from(value: ApplyError) -> Self {
        Self::FailedApply { inner: value }
    }
}

#[cfg(feature = "parse_cvars")]
impl From<TomlError> for CVarError {
    fn from(value: TomlError) -> Self {
        Self::TomlError(value)
    }
}

impl From<ResourceFetchError> for CVarError {
    fn from(value: ResourceFetchError) -> Self {
        match value {
            ResourceFetchError::NotRegistered => CVarError::UnknownCVar,
            ResourceFetchError::DoesNotExist(_) => CVarError::UnknownCVar,
            ResourceFetchError::NoResourceAccess(_) => CVarError::AccessConflict,
        }
    }
}
