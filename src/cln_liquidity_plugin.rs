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
