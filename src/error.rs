use std::fmt;

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use cdk::mint_url::MintUrl;
use cdk::nuts::CurrencyUnit;
use uuid::Uuid;

use crate::types::QuoteState;

#[derive(Debug)]
pub enum PosError {
    InvalidUuid(String),
    QuoteNotFound(Uuid),
    InvalidChannelSize {
        size: u64,
        min: u64,
        max: u64,
    },
    UnsupportedMint(MintUrl),
    UnsupportedCurrencyUnit {
        given: String,
        allowed: Vec<CurrencyUnit>,
    },
    InvalidQuoteState {
        id: Uuid,
        state: QuoteState,
    },
    InsufficientPayment {
        expected: u64,
        received: u64,
    },
    DatabaseError(String),
    ChannelOpenError(String),
    WalletError(String),
    ProofVerificationError(String),
    InternalError(String),
}

impl fmt::Display for PosError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUuid(id) => write!(f, "Invalid UUID format: {}", id),
            Self::QuoteNotFound(id) => write!(f, "Quote not found: {}", id),
            Self::InvalidChannelSize { size, min, max } => {
                write!(
                    f,
                    "Channel size {} outside allowed range ({}-{})",
                    size, min, max
                )
            }
            Self::UnsupportedMint(mint) => write!(f, "Unsupported mint: {}", mint),
            Self::UnsupportedCurrencyUnit { given, allowed } => write!(
                f,
                "Unsupported currency unit: {}. Allowed units are: {}",
                given,
                allowed
                    .iter()
                    .map(|u| u.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Self::InvalidQuoteState { id, state } => {
                write!(f, "Quote {} has invalid state: {:?}", id, state)
            }
            Self::InsufficientPayment { expected, received } => {
                write!(
                    f,
                    "Insufficient payment: expected {}, received {}",
                    expected, received
                )
            }
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::ChannelOpenError(msg) => write!(f, "Failed to open channel: {}", msg),
            Self::WalletError(msg) => write!(f, "Wallet error: {}", msg),
            Self::ProofVerificationError(msg) => write!(f, "Proof verification error: {}", msg),
            Self::InternalError(msg) => write!(f, "Internal server error: {}", msg),
        }
    }
}

impl IntoResponse for PosError {
    fn into_response(self) -> Response {
        let status = match &self {
            Self::InvalidUuid(_)
            | Self::InvalidChannelSize { .. }
            | Self::UnsupportedMint(_)
            | Self::UnsupportedCurrencyUnit { .. }
            | Self::InvalidQuoteState { .. }
            | Self::InsufficientPayment { .. } => StatusCode::BAD_REQUEST,

            Self::QuoteNotFound(_) => StatusCode::NOT_FOUND,

            Self::DatabaseError(_)
            | Self::ChannelOpenError(_)
            | Self::WalletError(_)
            | Self::ProofVerificationError(_)
            | Self::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        tracing::error!("POS error: {}", self);
        (status, self.to_string()).into_response()
    }
}
