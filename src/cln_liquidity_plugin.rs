use super::*;

/// fetches the available inbound liquidity in sat from CLN
async fn get_inbound_liquidity() -> Result<u64> {
    Ok(0)
}

/// createinvoice hook
/// if inbound liquidity is too low to receive the requested amount
/// the invoice is replaced with a cashu invoice
pub async fn createinvoice_hook() -> Result<()> {
    Ok(())
}

pub async fn rpc_command_handler(
    _p: Plugin<()>,
    v: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    log::info!("Got a connect hook call: {}", v);
    Ok(json!({"result": "continue"}))
}

// sample logs ----------------------
// Got a connect hook call: {"rpc_command":{"id":"init/offers:listconfigs#6","jsonrpc":"2.0","method":"listconfigs","params":{"config":"i-promise-to-fix-broken-api-user"}}}
// Got a connect hook call: {"rpc_command":{"id":"init/bookkeeper:listconfigs#0","jsonrpc":"2.0","method":"listconfigs","params":{"config":"i-promise-to-fix-broken-api-user"}}}
// Got a connect hook call: {"rpc_command":{"id":"recover:listpeerchannels#2","jsonrpc":"2.0","method":"listpeerchannels","params":{}}}
