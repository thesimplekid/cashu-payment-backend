use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, bail};
use bip39::Mnemonic;
use cashu_pos::config::AppConfig;
use cashu_pos::create_cashu_pos_router;
use cashu_pos::db::Db;
use cashu_pos::types::CashuPosInfo;
use cdk::mint_url::MintUrl;
use cdk::nuts::CurrencyUnit;
use cdk::wallet::{MultiMintWallet, Wallet};
use tower_http::cors::CorsLayer;
use tracing_subscriber::EnvFilter;

fn main() -> anyhow::Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    let runtime = Arc::new(runtime);

    runtime.block_on(async {
        let work_dir = home::home_dir()
            .ok_or(anyhow!("Could not get home dir"))?
            .join(".cashu-pos");

        // Ensure work directory exists
        std::fs::create_dir_all(&work_dir)
            .map_err(|e| anyhow!("Failed to create work directory: {}", e))?;

        // Load configuration
        let config_path = work_dir.join("config.toml");
        let config = match AppConfig::new(Some(&config_path)) {
            Ok(config) => config,
            Err(e) => {
                eprintln!("Failed to load configuration: {}", e);
                eprintln!(
                    "An example configuration has been created at: {}",
                    work_dir.join("example.config.toml").display()
                );
                eprintln!(
                    "Please copy and modify this file to: {}",
                    config_path.display()
                );
                return Err(anyhow::anyhow!("Configuration error: {}", e));
            }
        };

        let default_filter = "debug";
        let sqlx_filter = "sqlx=warn";
        let hyper_filter = "hyper=warn";
        let h2_filter = "h2=warn";
        let rustls_filter = "rustls=warn";

        let env_filter = EnvFilter::new(format!(
            "{},{},{},{},{}",
            default_filter, sqlx_filter, hyper_filter, h2_filter, rustls_filter
        ));

        tracing_subscriber::fmt().with_env_filter(env_filter).init();

        let localstore = Arc::new(cdk_redb::WalletRedbDatabase::new(
            &work_dir.join("cdk-wallet.redb"),
        )?);

        let seed = Mnemonic::generate(12)?;

        let mut wallets = vec![];

        for mint in config.pos.accepted_mints.iter() {
            let wallet = Wallet::new(
                mint,
                CurrencyUnit::Usd,
                localstore.clone(),
                &seed.to_seed_normalized(""),
                None,
            )?;

            wallets.push(wallet);

            let wallet_sat = Wallet::new(
                mint,
                CurrencyUnit::Sat,
                localstore.clone(),
                &seed.to_seed_normalized(""),
                None,
            )?;
            wallets.push(wallet_sat);
        }

        let wallet = MultiMintWallet::new(wallets);

        let cdk_pos = cashu_pos::CashuPos::new(wallet)?;

        let cdk_pos = Arc::new(cdk_pos);

        // Configure POS server
        let cashu_pos_info = CashuPosInfo {
            accepted_mints: config
                .pos
                .accepted_mints
                .clone()
                .iter()
                .map(|s| MintUrl::from_str(s))
                .collect::<Result<Vec<MintUrl>, _>>()?,
        };

        let payment_url = config.pos.payment_url.clone();

        let db = Db::new(work_dir.join("cashu-lsp.redb"))?;

        let service =
            create_cashu_pos_router(Arc::clone(&cdk_pos), cashu_pos_info, payment_url, db).await?;

        let service = service.layer(CorsLayer::permissive());

        // Start POS HTTP server
        let socket_addr = SocketAddr::from_str(&format!(
            "{}:{}",
            config.pos.listen_host, config.pos.listen_port
        ))?;

        tracing::info!("Starting POS server on {}", socket_addr);

        let listener = tokio::net::TcpListener::bind(socket_addr).await?;

        let axum_result = axum::serve(listener, service).with_graceful_shutdown(shutdown_signal());

        match axum_result.await {
            Ok(_) => {
                tracing::info!("Axum server stopped with okay status");
            }
            Err(err) => {
                tracing::warn!("Axum server stopped with error");
                tracing::error!("{}", err);
                bail!("Axum exited with error")
            }
        }

        Ok(())
    })
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C handler");
    tracing::info!("Shutdown signal received");
}
