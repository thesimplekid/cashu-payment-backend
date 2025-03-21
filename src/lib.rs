use cdk::wallet::MultiMintWallet;

pub mod config;
pub mod db;
pub mod error;
pub mod pos_server;
pub mod types;

pub use pos_server::create_cashu_pos_router;

pub struct CashuPos {
    wallet: MultiMintWallet,
}

impl CashuPos {
    pub fn new(wallet: MultiMintWallet) -> anyhow::Result<Self> {
        Ok(Self { wallet })
    }
}
