//! Provides tools for parsing CVar overrides ([CVarOverride]) and config files.
use std::{error::Error, fmt::Display, str::FromStr};

/// A partially parsed CVar override. This ensures its in the correct format, but does not ensure it'll deserialize!
#[derive(Clone, Debug)]
pub struct CVarOverride(pub(crate) String, pub(crate) toml_edit::Value);

/// Errors that can occur parsing a [CVarOverride]
#[derive(Debug)]
#[non_exhaustive]
pub enum CVarOverrideParseError {
    /// Error indicating the override is invalid as the left side is not a path.
    InvalidPath,
    /// Error indicating the override is invalid as the right side is not valid TOML.
    InvalidToml,
    /// Error indicating the override is invalid as it doesn't even look like an override (`left=right`)
    DoesntLookLikeAnOverride,
}

impl Error for CVarOverrideParseError {}

impl Display for CVarOverrideParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CVarOverrideParseError::InvalidPath => write!(
                f,
                "Not a valid override, a CVar override must have a CVar path (a.b.c) on the left."
            ),
            CVarOverrideParseError::InvalidToml => write!(
                f,
                "Not a valid override, a CVar override must have TOML on the right."
            ),
            CVarOverrideParseError::DoesntLookLikeAnOverride => write!(
                f,
                "Not a valid override, a CVar override must be of form `left=right`"
            ),
        }
    }
}

impl TryFrom<&str> for CVarOverride {
    type Error = CVarOverrideParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let (left, right) = value
            .split_once('=')
            .ok_or(CVarOverrideParseError::DoesntLookLikeAnOverride)?;

        let value =
            toml_edit::Value::from_str(right).map_err(|_| CVarOverrideParseError::InvalidToml)?;

        Ok(CVarOverride(left.to_owned(), value))
    }
}

impl FromStr for CVarOverride {
    type Err = CVarOverrideParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from(s)
    }
}
