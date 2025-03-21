use cdk::mint_url::MintUrl;
use cdk::nuts::CurrencyUnit;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize)]
pub struct QuoteInfo {
    pub id: Uuid,
    pub amount: u64,
    pub state: QuoteState,
    pub unit: CurrencyUnit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelQuoteRequest {
    pub amount: u64,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum QuoteState {
    Unpaid,
    Paid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CashuPosInfo {
    pub accepted_mints: Vec<MintUrl>,
}
