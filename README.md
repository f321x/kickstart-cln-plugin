# ```kickstart-cln``` [Core Lightning](https://github.com/ElementsProject/lightning) Plugin

This plugin intercepts the CLN [```createinvoice```](https://docs.corelightning.org/reference/lightning-createinvoice) RPC command and replaces the invoice with a 
[cashu ecash](https://cashu.space/) mint invoice in case the inbound [liquidity](https://bitcoin.design/guide/how-it-works/liquidity/) is too low to receive the payment directly.

Once enough satoshi are collected a lightning channel will be
requested from a specified [Lightning Service Provider (LSP)](https://thebitcoinmanual.com/articles/explained-lsp/).

This way new CLN users can instantly receive small payments, even if no funds to purchase a channel are available upfront.
If inbound liquidity is depleted again and no automatic swapping mechanism is set-up the plugin will also help to automatically get new inbound channels from the LSP.

### <u>Setup</u>
The plugin is able to read from the environment or a .env file 
in the plugins directory. The following variables are available:
* ```MINT_URL```: The ecash mint to use (e.g. ```https://mint.coinos.io```)
* ```CASHU_SEED```: 64 character (32 byte) hex encoded seed for the ecash wallet

The plugin has sane hardcoded values and can be used without setting any variables to allow for maximal simplicity.
If no seed is given the newly generated seed will be stored 
in a new (or existing) .env file.

### <u>Libraries</u>
The following bitcoin specific libraries were used:

* [cln-plugin](https://docs.rs/cln-plugin)
* [cdk](https://docs.rs/cdk) 

Thanks to the developers.