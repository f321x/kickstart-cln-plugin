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

#[tokio::main]
async fn main() -> Result<()> {
    // enable logging to stderr (stdout is used for plugin communication)
    env_logger::builder()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("kickstart-cln", log::LevelFilter::Trace)
        .target(Target::Stderr)
        .init();
    // load .env file
    dotenv().ok();
    // initialize ecash wallet
    let wallet = EcashWallet::new().await?;

    // run demo
    // demo(wallet).await?;

    // catch created invoice // hook
    // check inbound liquidity // lightning-listchannels RPC
    // if inbound liquidity is low, replace invoice with cashu invoice
    // check if balance is enough to open channel

    // if let Some(plugin) = Builder::new(tokio_stdin(), tokio_stdout())
    //     .dynamic()
    //     .option(TEST_OPTION)
    //     .option(TEST_OPTION_NO_DEFAULT)
    //     .option(test_dynamic_option)
    //     .setconfig_callback(setconfig_callback)
    //     .rpcmethod("testmethod", "This is a test", testmethod)
    //     .rpcmethod(
    //         "testoptions",
    //         "Retrieve options from this plugin",
    //         testoptions,
    //     )
    //     .rpcmethod(
    //         "test-custom-notification",
    //         "send a test_custom_notification event",
    //         test_send_custom_notification,
    //     )
    //     .subscribe("connect", connect_handler)
    //     .subscribe("test_custom_notification", test_receive_custom_notification)
    //     .hook("peer_connected", peer_connected_handler)
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

async fn demo(wallet: EcashWallet) -> Result<()> {
    let balance = wallet.get_total_balance().await?;
    println!("Total balance: {}", balance);
    if balance < 3 {
        let invoice = wallet.create_lightning_invoice(6 - balance).await?;
        println!("Invoice: {}", &invoice.bolt11);
        while !wallet.check_invoice_status(&invoice.mint_quote_id).await? {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }
        println!("Invoice paid");
        println!("Total balance: {}", wallet.get_total_balance().await?);
    }
    // read invoice from stdin
    let entered_lightning_invoice = {
        let mut input = String::new();
        println!("Enter lightning invoice: ");
        let mut stdin = tokio::io::BufReader::new(tokio_stdin());
        stdin.read_line(&mut input).await?;
        input.trim().to_string()
    };
    let payment_preimage = wallet
        .pay_lightning_invoice(entered_lightning_invoice)
        .await?;
    println!("Payment preimage: {}", payment_preimage);
    Ok(())
}
