use std::time::Duration;

use cdk::nuts::SpendingConditions;

use super::*;

pub struct EcashWallet {
    cdk_wallet: Wallet,
    pending_mint_requests: Vec<PaymentRequest>,
}

#[derive(Debug, Clone)]
pub struct PaymentRequest {
    pub bolt11: String,
    pub mint_quote_id: String,
    pub minted: bool,
    pub expiry: u64,
}

impl EcashWallet {
    pub async fn new() -> Result<Self> {
        let (seed, newly_generated) = gen_or_read_seed()?;
        let mint_url = match env::var("MINT_URL") {
            Ok(url) if url.len() > 1 => url,
            _ => {
                warn!("MINT_URL not set, using default (mint.coinos.io)");
                "https://mint.coinos.io".to_string()
            }
        };
        // let database = cdk_sqlite::WalletSqliteDatabase::new(Path::new("cashu_wallet.db")).await?;
        // database.migrate().await;
        let database = cdk_redb::WalletRedbDatabase::new(Path::new("cashu_wallet.db"))?;
        let cdk_wallet = Wallet::new(
            &mint_url,
            CurrencyUnit::Sat,
            Arc::new(database),
            &seed,
            None,
        )?;
        let existing_balance = cdk_wallet.total_balance().await?;
        if existing_balance == Amount::from(0) && !newly_generated {
            warn!("Found no balance in database on already existing secret, scanning for existing proofs...");
            let restored_amount: Amount = cdk_wallet.restore().await?;
            warn!("Restored balance: {}", restored_amount);
        }
        Ok(Self {
            cdk_wallet,
            pending_mint_requests: Vec::new(),
        })
    }

    pub async fn get_total_balance(&self) -> Result<u64> {
        Ok(self.cdk_wallet.total_balance().await?.into())
    }

    pub async fn pay_lightning_invoice(&self, bolt11_invoice: String) -> Result<String> {
        // get melt quote for invoice
        let melt_quote = self.cdk_wallet.melt_quote(bolt11_invoice, None).await?;
        // pay invoice
        let melted = self.cdk_wallet.melt(&melt_quote.id).await?; // blocking till paid
        if melted.state != MeltQuoteState::Paid {
            return Err(anyhow!("Invoice not paid, Status: {:?}", melted.state));
        }
        Ok(melted.preimage.unwrap_or(String::new()))
    }

    pub async fn create_lightning_invoice(&mut self, amount_sat: u64) -> Result<PaymentRequest> {
        let mint_quote = self
            .cdk_wallet
            .mint_quote(Amount::from(amount_sat), None)
            .await?;
        debug!("Mint quote: {:?}", mint_quote);
        let paymet_request = PaymentRequest {
            bolt11: mint_quote.request.clone(),
            mint_quote_id: mint_quote.id.clone(),
            expiry: mint_quote.expiry,
            minted: false,
        };
        self.pending_mint_requests.push(paymet_request.clone());
        Ok(paymet_request)
    }

    pub async fn check_invoice_status(&self, mint_quote_id: &str) -> Result<bool> {
        let status = self
            .cdk_wallet
            .mint_quote_state(mint_quote_id)
            .await?
            .paid
            .unwrap_or(false);
        if status {
            self.cdk_wallet
                .mint(mint_quote_id, SplitTarget::None, None)
                .await?;
        }
        Ok(status)
    }
}

pub async fn mint_pending_mint_requests(wallet: Arc<Mutex<EcashWallet>>) -> Result<()> {
    loop {
        if !wallet.lock().await.pending_mint_requests.is_empty() {
            trace!("Checking pending mint requests...");
            let mut wallet = wallet.lock().await;

            wallet.pending_mint_requests.retain(|quote| {
                if quote.expiry < unix_time() {
                    debug!("Quote expired: {}", quote.mint_quote_id);
                    false
                } else {
                    true
                }
            });
            let mut status_vec: Vec<bool> = Vec::new();
            for quote in &wallet.pending_mint_requests {
                status_vec.push(wallet.check_invoice_status(&quote.mint_quote_id).await?);
            }
            // remove paid invoices
            wallet.pending_mint_requests.retain(|quote| {
                if status_vec.pop().unwrap() {
                    debug!("Quote paid: {}", quote.mint_quote_id);
                    false
                } else {
                    true
                }
            });
        }
        tokio::time::sleep(Duration::from_secs(10)).await; // low timeout for demo
    }
}

/// Get the current unix time
fn unix_time() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards")
        .as_secs()
}

/// load hex seed from env
/// or generate a new one and save it in .env file
fn gen_or_read_seed() -> Result<([u8; 32], bool)> {
    match env::var("CASHU_SEED") {
        Ok(seed) if seed.len() == 64 => {
            trace!("Found existing seed in env, loading...");
            Ok((
                hex::decode(seed)?
                    .try_into()
                    .map_err(|_| anyhow!("Invalid seed in env"))?,
                false,
            ))
        }
        _ => {
            warn!("No seed found in env, generating and saving new seed...");
            // generate new seed
            let seed = rand::thread_rng().gen::<[u8; 32]>();
            // write newly generated seed into .env file (and create file if it doesn't exist)
            write_seed_to_env_file(seed)?;
            Ok((seed, true))
        }
    }
}

fn write_seed_to_env_file(seed: [u8; 32]) -> Result<()> {
    info!("Writing new seed to .env file");
    let seed = hex::encode(seed);
    let mut file = OpenOptions::new().create(true).append(true).open(".env")?;
    file.write_all(format!("CASHU_SEED={}\n", seed).as_bytes())?;
    Ok(())
}
