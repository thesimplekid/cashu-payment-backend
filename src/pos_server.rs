use axum::routing::{get, post};
use axum::{Router, extract::Json, extract::State};
use cdk::amount::{Amount, SplitTarget};
use cdk::nuts::{CurrencyUnit, PaymentRequest, PaymentRequestPayload, Transport, TransportType};
use cdk::wallet::types::WalletKey;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

use crate::CashuPos;
use crate::db::Db;
use crate::error::PosError;
use crate::types::{CashuPosInfo, QuoteInfo, QuoteState};

/// Cashu Pos State
#[derive(Clone)]
pub struct CashuPosState {
    node: Arc<CashuPos>,
    payment_url: String,
    db: Db,
    cashu_pos_info: CashuPosInfo,
}

pub async fn create_cashu_pos_router(
    node: Arc<CashuPos>,
    pos_info: CashuPosInfo,
    payment_url: String,
    db: Db,
) -> anyhow::Result<Router> {
    let state = CashuPosState {
        node,
        cashu_pos_info: pos_info,
        payment_url,
        db,
    };

    let router = Router::new()
        .route("/create", get(get_channel_quote))
        .route("/payment", post(post_receive_payment))
        .route("/check/{id}", get(get_quote_state))
        .with_state(state);

    Ok(router)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelQuoteResponse {
    checking_id: Uuid,
    payment_request: String,
}

pub async fn get_channel_quote(
    State(state): State<CashuPosState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ChannelQuoteResponse>, PosError> {
    // Extract amount from query parameters
    let amount = params
        .get("amount")
        .ok_or_else(|| PosError::InternalError("Missing amount parameter".to_string()))?
        .parse::<u64>()
        .map_err(|_| PosError::InternalError("Invalid amount format".to_string()))?;

    // Extract currency unit from query parameters, default to SAT if not provided
    let unit = match params.get("unit") {
        Some(unit_str) => {
            // Check if the unit is supported (currently only SAT and USD)
            let allowed_units = vec![CurrencyUnit::Sat, CurrencyUnit::Usd];

            let unit = CurrencyUnit::from_str(unit_str).map_err(|_| {
                PosError::InternalError(format!("Invalid currency unit format: {}", unit_str))
            })?;

            // Check if the unit is supported
            if !allowed_units.contains(&unit) {
                return Err(PosError::UnsupportedCurrencyUnit {
                    given: unit_str.to_string(),
                    allowed: allowed_units,
                });
            }

            unit
        }
        None => CurrencyUnit::Sat,
    };

    tracing::debug!(
        "Received channel quote request with amount: {} {}",
        amount,
        unit
    );

    let payment_id = Uuid::new_v4();

    let transport = Transport::builder()
        .transport_type(TransportType::HttpPost)
        .target(state.payment_url)
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build transport: {}", e);
            PosError::InternalError(format!("Failed to build transport: {}", e))
        })?;

    let payment_request = PaymentRequest::builder()
        .payment_id(payment_id)
        .amount(amount)
        .unit(unit.clone())
        .single_use(true)
        .mints(state.cashu_pos_info.accepted_mints)
        .add_transport(transport)
        .build();

    let quote = QuoteInfo {
        id: payment_id,
        state: QuoteState::Unpaid,
        amount,
        unit,
    };

    state.db.add_quote(&quote).map_err(|e| {
        tracing::error!("Failed to add quote to database: {}", e);
        PosError::DatabaseError(e.to_string())
    })?;

    tracing::info!("Created new channel quote: {}", payment_id);

    Ok(Json(ChannelQuoteResponse {
        checking_id: payment_id,
        payment_request: payment_request.to_string(),
    }))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteStateResponse {
    pub id: Uuid,
    pub state: QuoteState,
}

pub async fn get_quote_state(
    State(state): State<CashuPosState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<QuoteStateResponse>, PosError> {
    tracing::debug!("Received quote state request for ID: {}", id);

    let id = Uuid::from_str(&id).map_err(|e| {
        tracing::warn!("Invalid UUID format: {} - {}", id, e);
        PosError::InvalidUuid(id.clone())
    })?;

    let quote = state.db.get_quote(id).map_err(|e| {
        tracing::warn!("Quote not found: {} - {}", id, e);
        PosError::QuoteNotFound(id)
    })?;

    let response = QuoteStateResponse {
        id: quote.id,
        state: quote.state,
    };

    tracing::debug!("Returning quote state for {}: {:?}", id, response);
    Ok(Json(response))
}

pub async fn post_receive_payment(
    State(state): State<CashuPosState>,
    Json(payload): Json<PaymentRequestPayload>,
) -> Result<(), PosError> {
    tracing::debug!("Received payment for mint: {}", payload.mint);

    // Validate mint
    if !state.cashu_pos_info.accepted_mints.contains(&payload.mint) {
        return Err(PosError::UnsupportedMint(payload.mint.clone()));
    }

    // Validate payment ID
    let id = payload.id.ok_or_else(|| {
        tracing::warn!("Missing payment ID in request");
        PosError::InvalidUuid("missing".to_string())
    })?;

    let id = Uuid::from_str(&id).map_err(|e| {
        tracing::warn!("Invalid UUID format: {} - {}", id, e);
        PosError::InvalidUuid(id.clone())
    })?;

    // Get quote
    let quote = state.db.get_quote(id).map_err(|e| {
        tracing::warn!("Quote not found: {} - {}", id, e);
        PosError::QuoteNotFound(id)
    })?;

    // Validate quote state
    if quote.state != QuoteState::Unpaid {
        tracing::warn!("Quote {} has invalid state: {:?}", id, quote.state);
        return Err(PosError::InvalidQuoteState {
            id,
            state: quote.state,
        });
    }

    // Validate payment amount
    let received_amount =
        Amount::try_sum(payload.proofs.iter().map(|p| p.amount)).map_err(|e| {
            tracing::warn!("Failed to sum proof amounts: {}", e);
            PosError::InternalError("Failed to sum proof amounts".to_string())
        })?;

    if Amount::from(quote.amount) < received_amount {
        tracing::warn!(
            "Insufficient payment: expected {}, received {}",
            quote.amount,
            received_amount
        );
        return Err(PosError::InsufficientPayment {
            expected: quote.amount,
            received: received_amount.into(),
        });
    }

    // Get wallet for the mint with the correct currency unit
    let wallet = state
        .node
        .wallet
        .get_wallet(&WalletKey::new(payload.mint.clone(), quote.unit.clone()))
        .await
        .ok_or_else(|| {
            let msg = format!(
                "Wallet not created for {} with unit {:?}",
                payload.mint, quote.unit
            );
            tracing::warn!("{}", msg);
            PosError::WalletError(msg)
        })?;

    // Receive and verify proofs
    let amount = wallet
        .receive_proofs(payload.proofs, SplitTarget::default(), &[], &[])
        .await
        .map_err(|e| {
            tracing::error!("Could not receive proofs for {}: {}", id, e);
            PosError::ProofVerificationError(e.to_string())
        })?;

    tracing::info!(
        "Successfully received payment of {} {} for quote {}",
        amount,
        quote.unit,
        id
    );

    // Update quote state
    let _quote = state
        .db
        .update_quote_state(id, QuoteState::Paid)
        .map_err(|e| {
            tracing::error!("Failed to update quote state: {}", e);
            PosError::DatabaseError(e.to_string())
        })?;

    tracing::info!("Payment processing completed for quote {}", id);
    Ok(())
}
