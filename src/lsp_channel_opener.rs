use std::time::Duration;

use super::*;

// currently using zeus olympus ( i think LSP1 spec)
// semi professional llm API implementation -> warn!("hackathon project")

const BASE_URL: &str = "https://mutinynet-lsps1.lnolymp.us";

#[derive(Debug, Deserialize)]
struct GetInfoResponse {
    max_channel_balance_sat: String,
    max_channel_expiry_blocks: u32,
    max_initial_client_balance_sat: String,
    max_initial_lsp_balance_sat: String,
    min_channel_balance_sat: String,
    min_funding_confirms_within_blocks: u32,
    min_initial_client_balance_sat: String,
    min_initial_lsp_balance_sat: String,
    min_onchain_payment_confirmations: Option<u32>,
    min_onchain_payment_size_sat: Option<String>,
    min_required_channel_confirmations: u32,
    supports_zero_channel_reserve: bool,
    uris: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CreateOrderRequest {
    lsp_balance_sat: String,
    client_balance_sat: String,
    required_channel_confirmations: u32,
    funding_confirms_within_blocks: u32,
    channel_expiry_blocks: u32,
    token: String,
    refund_onchain_address: String,
    announce_channel: bool,
    public_key: String,
}

#[derive(Debug, Deserialize)]
struct CreateOrderResponse {
    announce_channel: bool,
    channel: Option<serde_json::Value>,
    channel_expiry_blocks: u32,
    client_balance_sat: String,
    funding_confirms_within_blocks: u32,
    created_at: String,
    lsp_balance_sat: String,
    order_id: String,
    order_state: String,
    payment: Payment,
    token: String,
}

#[derive(Debug, Deserialize)]
struct Payment {
    bolt11: Bolt11,
}

#[derive(Debug, Deserialize)]
struct Bolt11 {
    order_total_sat: String,
    fee_total_sat: String,
    invoice: String,
    state: String,
    expires_at: String,
}

struct OlympusLspClient {
    client: reqwest::Client,
}

impl OlympusLspClient {
    fn new() -> Self {
        OlympusLspClient {
            client: reqwest::Client::new(),
        }
    }

    async fn get_info(&self) -> Result<GetInfoResponse> {
        let url = format!("{}/api/v1/get_info", BASE_URL);
        let response = self.client.get(&url).send().await?;
        let info: GetInfoResponse = response.json().await?;
        Ok(info)
    }

    async fn create_order(&self, request: CreateOrderRequest) -> Result<CreateOrderResponse> {
        let url = format!("{}/api/v1/create_order", BASE_URL);
        let response = self.client.post(&url).json(&request).send().await?;
        if response.status() != 200 {
            return Err(anyhow!(
                "Failed to create order: {:?}",
                response.text().await?
            ));
        }
        let order: CreateOrderResponse = response.json().await?;
        Ok(order)
    }

    async fn get_order(&self, order_id: &str) -> Result<CreateOrderResponse> {
        let url = format!("{}/api/v1/get_order?order_id={}", BASE_URL, order_id);
        let response = self.client.get(&url).send().await?;
        let order: CreateOrderResponse = response.json().await?;
        Ok(order)
    }

    async fn get_estimated_cost(&self, size_sat: u64, node_pk: &str) -> Result<u64> {
        let info = self.get_info().await?;
        let create_order_request = CreateOrderRequest {
            lsp_balance_sat: size_sat.to_string(),
            client_balance_sat: "0".to_string(),
            required_channel_confirmations: info.min_required_channel_confirmations,
            funding_confirms_within_blocks: info.min_funding_confirms_within_blocks,
            channel_expiry_blocks: info.max_channel_expiry_blocks,
            token: "".to_string(),
            refund_onchain_address: "".to_string(),
            announce_channel: true,
            public_key: node_pk.to_string(),
        };
        let create_order_response = self.create_order(create_order_request).await?;
        Ok(create_order_response
            .payment
            .bolt11
            .order_total_sat
            .parse::<u64>()?)
    }
}

async fn open_lsp_channel(
    size_sat: u64,
    public_key: String,
    ecash_wallet: Arc<Mutex<EcashWallet>>,
) -> Result<()> {
    let client = OlympusLspClient::new();

    // Get info
    let info = client.get_info().await?;
    debug!("Info: {:?}", info);
    if size_sat < info.min_initial_lsp_balance_sat.parse::<u64>()?
        || size_sat > info.max_initial_lsp_balance_sat.parse::<u64>()?
    {
        return Err(anyhow!("Requested amount not accepted"));
    }

    // Create order
    let create_order_request = CreateOrderRequest {
        lsp_balance_sat: size_sat.to_string(),
        client_balance_sat: "0".to_string(),
        required_channel_confirmations: info.min_required_channel_confirmations,
        funding_confirms_within_blocks: info.min_funding_confirms_within_blocks,
        channel_expiry_blocks: info.max_channel_expiry_blocks,
        token: "".to_string(),
        refund_onchain_address: "".to_string(),
        announce_channel: true,
        public_key,
    };
    let create_order_response = client.create_order(create_order_request).await?;
    debug!("Create Order Response: {:?}", create_order_response);
    if ecash_wallet.lock().await.get_total_balance().await?
        < create_order_response
            .payment
            .bolt11
            .order_total_sat
            .parse::<u64>()?
    {
        return Err(anyhow!("Insufficient balance"));
    }

    ecash_wallet
        .lock()
        .await
        .pay_lightning_invoice(create_order_response.payment.bolt11.invoice)
        .await?;
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Get order
    let order_id = &create_order_response.order_id;
    let get_order_response = client.get_order(order_id).await?;
    debug!("Get LSP order response: {:?}", get_order_response);
    Ok(())
}

pub async fn channel_manager(ecash_wallet: Arc<Mutex<EcashWallet>>) -> Result<()> {
    let lsp_client = OlympusLspClient::new();
    let lsp_info = lsp_client.get_info().await?;
    debug!("LSP Info: {:?}", lsp_info);
    // create dummy order to get rough estimate of the cost of opening a channel in our configured size
    let target_channel_size_sat = env::var("TARGET_CHANNEL_SIZE_SAT")
        .unwrap_or("1000000".to_string())
        .parse::<u64>()?;
    let lsp_addr = parse_lsp_host(lsp_info.uris.clone()).pop().unwrap_or((
        "031b301307574bbe9b9ac7b79cbe1700e31e544513eae0b5d7497483083f99e581".to_string(),
        "45.79.192.236".to_string(),
        9735,
    ));
    let node_pk = connect_and_get_pk(&lsp_addr.1, lsp_addr.2, &lsp_addr.0).await?;
    let estimated_cost = lsp_client
        .get_estimated_cost(target_channel_size_sat, &node_pk)
        .await?;
    debug!(
        "Estimated cost for {} sat channel opening: {}",
        target_channel_size_sat, estimated_cost
    );
    loop {
        let ecash_balance = ecash_wallet.lock().await.last_balance;
        trace!("Ecash balance in channel_manager loop: {}", ecash_balance);

        // check if balance is enough to open channel
        if ecash_balance as f64 * 0.9 > estimated_cost as f64 {
            trace!("Opening LSP channel...");
            // connect to LSP node and get our public key
            let node_pk = connect_and_get_pk(&lsp_addr.1, lsp_addr.2, &lsp_addr.0).await?;
            open_lsp_channel(target_channel_size_sat, node_pk, ecash_wallet.clone()).await?;
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    }
}

fn parse_lsp_host(addresses: Vec<String>) -> Vec<(String, String, u16)> {
    addresses
        .into_iter()
        .filter_map(|address| {
            let parts: Vec<&str> = address.split('@').collect();
            if parts.len() != 2 {
                return None;
            }

            let id = parts[0].to_string();
            let host_port: Vec<&str> = parts[1].split(':').collect();
            if host_port.len() != 2 {
                return None;
            }

            let host = host_port[0].to_string();
            let port: u16 = host_port[1].parse().ok()?;

            // Skip .onion addresses
            if host.ends_with(".onion") {
                return None;
            }

            Some((id, host, port))
        })
        .collect()
}

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lsp_interface() {}
}
