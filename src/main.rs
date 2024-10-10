mod cln_liquidity_plugin;
mod ecash_wallet;

use anyhow::{anyhow, Result};
use cdk::*;
use cln_plugin::Builder;
use dotenvy::dotenv;
use ecash_wallet::EcashWallet;
use env_logger::Target;
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};
use rand::Rng;
use std::{env, fs::OpenOptions, io::Write, path::Path, sync::Arc};
use tokio::io::{stdin as tokio_stdin, stdout as tokio_stdout, AsyncBufReadExt};

// disclaimer: started hacking on this on Thursday (and some research before)
#[tokio::main]
async fn main() -> Result<()> {
    // enable logging to stderr (stdout is used for plugin communication)
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("kickstart_cln", log::LevelFilter::Trace)
        .target(Target::Stderr)
        .init();
    // load .env file
    dotenv().ok();
    // initialize ecash wallet
    let wallet = EcashWallet::new().await?;

    // run demo
    // _demo(wallet).await?;

    // catch created invoice // hook @ lightning-invoice
    // check inbound liquidity // lightning-listchannels RPC
    // if inbound liquidity is low, replace invoice with cashu invoice
    // check if balance is enough to open channel

    // if let Some(plugin) = Builder::new(tokio_stdin(), tokio_stdout())
    //     // .option(TEST_OPTION)  // used to accept cli input from user to plugin
    //     .subscribe("connect", connect_handler)
    //     .subscribe("test_custom_notification", test_receive_custom_notification)
    //     .hook("rpc_command", peer_connected_handler)
    //     .notification(messages::NotificationTopic::new(TEST_NOTIF_TAG))
    //     .start(state)
    //     .await?
    // {
    //     plugin.join().await
    // } else {
    //     Ok(())
    // }

    Ok(())
}

async fn _demo(wallet: EcashWallet) -> Result<()> {
    let balance = wallet.get_total_balance().await?;
    debug!("Total balance: {}", balance);
    if balance < 3 {
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
