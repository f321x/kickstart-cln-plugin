use super::*;

pub struct EcashWallet {
    cdk_wallet: Wallet,
}

#[derive(Debug, Clone)]
pub struct PaymentRequest {
    pub bolt11: String,
    pub mint_quote_id: String,
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
        Ok(Self { cdk_wallet })
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

    pub async fn create_lightning_invoice(&self, amount_sat: u64) -> Result<PaymentRequest> {
        let mint_quote = self
            .cdk_wallet
            .mint_quote(Amount::from(amount_sat), None)
            .await?;
        Ok(PaymentRequest {
            bolt11: mint_quote.request.clone(),
            mint_quote_id: mint_quote.id,
        })
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
