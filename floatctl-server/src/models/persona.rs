//! Persona enum - Spec 3.2
//!
//! Personas for the inbox system.

use serde::{Deserialize, Serialize};

use super::ValidationError;

/// Valid personas for inbox messaging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Persona {
    Evna,
    Kitty,
    Cowboy,
    Daddy,
}

impl Persona {
    /// Parse persona from string.
    pub fn from_str(s: &str) -> Result<Self, ValidationError> {
        match s.to_lowercase().as_str() {
            "evna" => Ok(Self::Evna),
            "kitty" => Ok(Self::Kitty),
            "cowboy" => Ok(Self::Cowboy),
            "daddy" => Ok(Self::Daddy),
            _ => Err(ValidationError::InvalidVariant {
                field: "persona",
                value: s.to_owned(),
            }),
        }
    }

    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Evna => "evna",
            Self::Kitty => "kitty",
            Self::Cowboy => "cowboy",
            Self::Daddy => "daddy",
        }
    }

    /// Get all valid personas.
    pub fn all() -> &'static [Self] {
        &[Self::Evna, Self::Kitty, Self::Cowboy, Self::Daddy]
    }
}

impl std::fmt::Display for Persona {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_personas() {
        assert!(Persona::from_str("evna").is_ok());
        assert!(Persona::from_str("kitty").is_ok());
        assert!(Persona::from_str("cowboy").is_ok());
        assert!(Persona::from_str("daddy").is_ok());
    }

    #[test]
    fn case_insensitive() {
        assert_eq!(Persona::from_str("EVNA").unwrap(), Persona::Evna);
        assert_eq!(Persona::from_str("Kitty").unwrap(), Persona::Kitty);
    }

    #[test]
    fn invalid_persona() {
        let err = Persona::from_str("invalid").unwrap_err();
        assert!(matches!(err, ValidationError::InvalidVariant { .. }));
    }

    #[test]
    fn roundtrip() {
        for persona in Persona::all() {
            let s = persona.as_str();
            let parsed = Persona::from_str(s).unwrap();
            assert_eq!(*persona, parsed);
        }
    }
}
