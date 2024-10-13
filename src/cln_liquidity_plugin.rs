use super::*;

#[derive(Debug, Serialize, Deserialize)]
struct InvoiceParams {
    #[serde(rename = "0")]
    amount_msat: u64,
    #[serde(rename = "1")]
    description: String,
    #[serde(rename = "2")]
    label: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RpcCommand {
    id: String,
    jsonrpc: String,
    method: String,
    params: InvoiceParams,
}

#[derive(Debug, Serialize, Deserialize)]
struct ConnectHookCall {
    rpc_command: RpcCommand,
}

// main handler that hooks into the lightning-invoice RPC command
pub async fn rpc_command_handler(
    p: Plugin<Arc<Mutex<EcashWallet>>>,
    v: serde_json::Value,
) -> Result<serde_json::Value, Error> {
    let rpc_command: Option<ConnectHookCall> = serde_json::from_value(v).ok();
    // we continue if it is the "invoice" command else we return
    if let Some(rpc_call) = rpc_command {
        debug!("Got a invoice hook call: {:?}", rpc_call.rpc_command.params);

        // fetch the balances
        let inbound_liq_msat = (get_available_inbound_liquidity().await? as f64 * 0.9) as u64; // 0.9 is a buffer factor
        let ecash_balance_sat = p.state().lock().await.get_total_balance().await?;
        p.state().lock().await.last_balance = ecash_balance_sat; // https://www.youtube.com/watch?v=dQw4w9WgXcQ
        debug!(
            "Inbound liquidity: {} | Ecash balance: {}",
            inbound_liq_msat, ecash_balance_sat
        );

        if inbound_liq_msat < rpc_call.rpc_command.params.amount_msat {
            // replace invoice with cashu invoice
            let cashu_invoice = match p
                .state()
                .lock()
                .await
                .create_lightning_invoice(rpc_call.rpc_command.params.amount_msat / 1000)
                .await
            {
                Ok(request) => request,
                Err(e) => {
                    error!("Error creating cashu invoice (mint probably doesn't like high amounts): {}", e);
                    return Ok(json!({"return": {
                        "error": {
                            "message": "Error creating cashu invoice and you have too low inbound liquidity,
                            probably too low/high amount for mint (use msat!)",
                            "code": 1
                        }
                    }}));
                }
            };
            debug!("Cashu invoice: {}", cashu_invoice.bolt11);
            return Ok(json!({
                "return": {
                    "result": {
                        "bolt11": cashu_invoice.bolt11,
                        "created_index": 999,  // dummy
                        "expires_at": cashu_invoice.expiry,
                        "payment_hash": "0b02490f82526f7ea932b8c2cd265be6e9d6a0eb043bd560efe989cd28e4717",  // dummy -> "cashu"
                        "paymetn_secret": "000" // dummy
                    }
                }
            }));
        }
    }
    Ok(json!({"result": "continue"}))
}

// fetches the total available inbound liquidity in msat
async fn get_available_inbound_liquidity() -> Result<u64> {
    let request = ListfundsRequest { spent: None };
    let response: Response = send_rpc_request(request.into()).await?;
    let listfunds_channels = match response {
        Response::ListFunds(funds) => funds.channels,
        _ => return Err(anyhow!("Unexpected response")),
    };

    let total_inbound_liquidity: u64 = listfunds_channels
        .iter()
        .map(|channel| {
            if channel.connected {
                channel.amount_msat.msat() - channel.our_amount_msat.msat()
            } else {
                0
            }
        })
        .sum();

    Ok(total_inbound_liquidity)
}

// connects to the LSP node as we don't have a public IP to connect to and
// returns our nodes public key for the LSP to open a channel to
pub async fn connect_and_get_pk(lsp_host: &str, lsp_port: u16, lsp_id: &str) -> Result<String> {
    // request our own public key
    let request = GetinfoRequest {};
    let response: Response = send_rpc_request(request.into()).await?;
    let info = match response {
        Response::Getinfo(info) => info,
        _ => return Err(anyhow!("Unexpected response")),
    };
    let public_key = info.id.to_string();

    // connect to the LSP node
    let request = ConnectRequest {
        host: Some(lsp_host.to_string()),
        port: Some(lsp_port),
        id: lsp_id.to_string(),
    };
    let response: Response = send_rpc_request(request.into()).await?;
    match response {
        Response::Connect(response) => {
            debug!("Connected to LSP node: {}", response.id);
        }
        _ => return Err(anyhow!("Unexpected response")),
    };
    Ok(public_key)
}

// can't init rpc client upfront because the socket is only available after plugin setup
async fn send_rpc_request(request: Request) -> Result<Response> {
    let path = Path::new("./lightning-rpc");
    let mut rpc = ClnRpc::new(path).await?;
    let response = rpc.call(request).await?;
    Ok(response)
}

// sample logs ----------------------
// Got a connect hook call: {"rpc_command":{"id":"init/offers:listconfigs#6","jsonrpc":"2.0","method":"listconfigs","params":{"config":"i-promise-to-fix-broken-api-user"}}}
// Got a connect hook call: {"rpc_command":{"id":"init/bookkeeper:listconfigs#0","jsonrpc":"2.0","method":"listconfigs","params":{"config":"i-promise-to-fix-broken-api-user"}}}
// Got a connect hook call: {"rpc_command":{"id":"recover:listpeerchannels#2","jsonrpc":"2.0","method":"listpeerchannels","params":{}}}

// root@0a6d5dfbe81c:/# lightning-cli invoice 1000 desc lab
// {
//    "payment_hash": "01be6793873d9bbb41e06727f45dec42e90fc385c0a1c224a516e973571d6385",
//    "expires_at": 1729325931,
//    "bolt11": "lntbs10n1pns5v8tsp549dcrpnlxd2yk244sg65ml2eaqc248sv4mfy4vmu7nvjt68qh2yqpp5qxlx0yu88kdmks0qvunlgh0vgt5slsu9czsuyf99zm5hx4cavwzsdq9d3skyxqyjw5qcqp29qxpqysgq4maskqpj6js4y997n9zuf5af6ysv9nzhvy9sq75zxt9mtugh20wr3n66d8wm02xr9a2xdzqthqmgm6dsz3t2ghzfdv0ezhhgy25csxqqm2vl0l",
//    "payment_secret": "a95b81867f33544b2ab582354dfd59e830aa9e0caed24ab37cf4d925e8e0ba88",
//    "created_index": 2,
//    "warning_capacity": "Insufficient incoming channel capacity to pay invoice"
// }

// Got a connect hook call: {"rpc_command":{"id":"cli:invoice#2985","jsonrpc":"2.0","method":"invoice","params":[1000,"desc","lab"]}}
