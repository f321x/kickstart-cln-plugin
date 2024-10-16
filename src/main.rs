mod cln_liquidity_plugin;
mod ecash_wallet;
mod lsp_channel_opener;

use anyhow::{anyhow, Error, Result};
use cdk::{
    amount::{Amount, SplitTarget},
    error,
    nuts::{CurrencyUnit, MeltQuoteState},
    wallet::Wallet,
};
use cln_liquidity_plugin::{connect_and_get_pk, rpc_command_handler};
use cln_plugin::{Builder, Plugin};
use cln_rpc::{
    model::{
        requests::{ConnectRequest, GetinfoRequest, ListfundsRequest},
        Request, Response,
    },
    ClnRpc,
};
use dotenvy::dotenv;
use ecash_wallet::{mint_pending_mint_requests, EcashWallet};
use env_logger::Target;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use lsp_channel_opener::channel_manager;
// use lsp_channel_opener::channel_manager;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    env, fs::OpenOptions, io::Write, path::Path, sync::Arc, thread::AccessError, time::Duration,
};
use tokio::{
    io::{stdin as tokio_stdin, stdout as tokio_stdout, AsyncBufReadExt},
    sync::Mutex,
};

// disclaimer: started hacking on this on Thursday (some research, ecash functions and part of the readme)
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // enable logging to stderr (stdout is used for plugin communication)
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("kickstart_cln", log::LevelFilter::Trace)
        .target(Target::Stderr)
        .init();
    // load .env file
    dotenv().ok();

    warn!("This is a hackathon project, usage is definitely reckless!");

    // initialize ecash wallet
    let wallet = Arc::new(Mutex::new(EcashWallet::new().await?));
    let minting_wallet = Arc::clone(&wallet);
    let channel_manager_wallet = Arc::clone(&wallet);
    tokio::task::spawn(async move { mint_pending_mint_requests(minting_wallet).await });

    tokio::task::spawn(async move {
        let err = channel_manager(channel_manager_wallet).await;
        error!("Channel manager exited: {:?}", err);
        tokio::time::sleep(Duration::from_secs(60)).await;
    });

    // run ecash wallet demo
    // _demo(&mut *wallet.lock().await).await?;

    // catch created invoice // hook @ lightning-invoice
    // check inbound liquidity // lightning-listchannels RPC
    // if inbound liquidity is low, replace invoice with cashu invoice
    // check if balance is enough to open channel
    trace!("Starting cln plugin...");
    let state = wallet;
    if let Some(plugin) = Builder::new(tokio_stdin(), tokio_stdout())
        .hook("rpc_command", rpc_command_handler)
        .with_logging(false)
        .start(state)
        .await?
    {
        info!("Plugin initiated successfully, running...");
        plugin.join().await?;
    }

    warn!("Plugin exited");
    Ok(())
}

// demo to test the ecash wallet functionality (mint/melt/balance)
async fn _demo(wallet: &mut EcashWallet) -> Result<()> {
    let balance = wallet.get_total_balance().await?;
    debug!("Total balance: {}", balance);
    if balance < 7 {
        let invoice = wallet.create_lightning_invoice(6).await?;
        debug!("Invoice: {}", &invoice.bolt11);
        while !wallet.check_invoice_status(&invoice.mint_quote_id).await? {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        debug!("Invoice paid");
        debug!("Total balance: {}", wallet.get_total_balance().await?);
    }
    // read invoice from stdin
    let entered_lightning_invoice = {
        let mut input = String::new();
        eprintln!("Enter lightning invoice: ");
        let mut stdin = tokio::io::BufReader::new(tokio_stdin());
        stdin.read_line(&mut input).await?;
        input.trim().to_string()
    };
    let payment_preimage = wallet
        .pay_lightning_invoice(entered_lightning_invoice)
        .await?;
    debug!("Payment preimage: {}", payment_preimage);
    Ok(())
}
