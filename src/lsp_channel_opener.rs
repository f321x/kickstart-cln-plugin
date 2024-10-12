use super::*;

// currently using zeus olympus ( i think LSP1 spec)
// this API implementation is pure LLM gore -> warn!("hackathon project")

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
    client: Client,
}

impl OlympusLspClient {
    fn new() -> Self {
        OlympusLspClient {
            client: Client::new(),
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
        let order: CreateOrderResponse = response.json().await?;
        Ok(order)
    }

    async fn get_order(&self, order_id: &str) -> Result<CreateOrderRequest> {
        let url = format!("{}/api/v1/get_order?order_id={}", BASE_URL, order_id);
        let response = self.client.get(&url).send().await?;
        let order: GetOrderResponse = response.json().await?;
        Ok(order)
    }
}

pub async fn open_lsp_channel() -> Result<()> {
    let client = OlympusLspClient::new();

    // Get info
    let info = client.get_info().await?;
    debug!("Info: {:?}", info);

    // Create order
    let create_order_request = CreateOrderRequest {
        lsp_balance_sat: "10000000".to_string(),
        client_balance_sat: "0".to_string(),
        required_channel_confirmations: 8,
        funding_confirms_within_blocks: 6,
        channel_expiry_blocks: 13000,
        token: "".to_string(),
        refund_onchain_address: "".to_string(),
        announce_channel: false,
        public_key: "025b7a68b4cd85668e65db6a343a4c607a462cdd010daa793f82be561a3316c5b1"
            .to_string(),
    };
    let create_order_response = client.create_order(create_order_request).await?;
    debug!("Create Order Response: {:?}", create_order_response);

    // Get order
    let order_id = &create_order_response.order_id;
    let get_order_response = client.get_order(order_id).await?;
    debug!("Get Order Response: {:?}", get_order_response);
}
