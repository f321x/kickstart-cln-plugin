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
use tokio::io::{stdin as tokio_stdin, stdout as tokio_stdout};

#[tokio::main]
async fn main() -> Result<()> {
    // enable logging to stderr (stdout is used for plugin communication)
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .target(Target::Stderr)
        .init();
    // load .env file
    dotenv().ok();

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
