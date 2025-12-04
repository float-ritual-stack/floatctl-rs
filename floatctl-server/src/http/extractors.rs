//! Custom Axum extractors

use axum::extract::{FromRequestParts, Path};
use axum::http::request::Parts;
use uuid::Uuid;

use crate::models::{BoardName, Persona, ValidationError};
use super::error::ApiError;

/// Extract and validate a board name from path
pub struct ValidBoardName(pub BoardName);

#[axum::async_trait]
impl<S> FromRequestParts<S> for ValidBoardName
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(name): Path<String> = Path::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::Validation(ValidationError::Empty { field: "board name" }))?;

        let board_name = BoardName::new(&name)?;
        Ok(Self(board_name))
    }
}

/// Extract and validate a persona from path
pub struct ValidPersona(pub Persona);

#[axum::async_trait]
impl<S> FromRequestParts<S> for ValidPersona
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(name): Path<String> = Path::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::Validation(ValidationError::Empty { field: "persona" }))?;

        let persona = Persona::from_str(&name)?;
        Ok(Self(persona))
    }
}

/// Extract and validate a UUID from path
pub struct ValidUuid(pub Uuid);

#[axum::async_trait]
impl<S> FromRequestParts<S> for ValidUuid
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let Path(id): Path<String> = Path::from_request_parts(parts, state)
            .await
            .map_err(|_| ApiError::Validation(ValidationError::Empty { field: "id" }))?;

        let uuid = Uuid::parse_str(&id).map_err(|_| {
            ApiError::Validation(ValidationError::InvalidFormat {
                field: "id",
                reason: "invalid UUID format",
            })
        })?;

        Ok(Self(uuid))
    }
}
