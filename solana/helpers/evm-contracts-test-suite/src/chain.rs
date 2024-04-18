//! Test EVM blockchain for testing purposes.
use std::sync::Arc;
use std::time::Duration;

use ethers::middleware::SignerMiddleware;
use ethers::providers::{Http, Provider};
use ethers::signers::LocalWallet;
use ethers::utils::{Anvil, AnvilInstance};

/// A test blockchain environment for testing EVM contracts.
/// It uses `anvil` under the hood.
pub struct TestBlockchain {
    /// An instance of Anvil, a local Ethereum testnet.
    pub anvil: AnvilInstance,
    /// A shared provider for accessing the blockchain.
    pub provider: Arc<Provider<Http>>,
}

impl Default for TestBlockchain {
    fn default() -> Self {
        Self::new()
    }
}

impl TestBlockchain {
    /// Creates a new `TestBlockchain` instance with a specified AnvilInstance.
    /// This allows for the creation of a test blockchain environment with a
    /// predefined Anvil setup.
    pub fn new_with_anvil(anvil: AnvilInstance) -> Self {
        let provider = Provider::<Http>::try_from(anvil.endpoint())
            .expect("Valid URL")
            .interval(Duration::from_millis(100));
        let provider = Arc::new(provider);
        TestBlockchain { provider, anvil }
    }

    /// Creates a new `TestBlockchain` instance with a default Anvil setup.
    /// This is useful for quickly setting up a test blockchain environment
    /// without needing to configure Anvil manually.
    ///
    /// # Returns
    ///
    /// Returns a new instance of `TestBlockchain` with a default Anvil
    /// configuration.
    pub fn new() -> Self {
        let mnemonic = "abstract vacuum mammal awkward pudding scene penalty purchase dinner depart evoke puzzle";
        let anvil = Anvil::new().mnemonic(mnemonic).spawn();
        Self::new_with_anvil(anvil)
    }

    /// Constructs a provider wrapped with a signer from the test blockchain's
    /// Anvil instance. This allows for transactions to be signed with the
    /// private key associated with the specified index.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the key (derived from the mnemonic) to be used
    ///   for signing transactions.
    ///
    /// # Returns
    ///
    /// Returns an instance of `EvmSigner` which contains the signer and wallet
    /// information for interacting with the blockchain.
    pub fn construct_provider_with_signer(&self, idx: usize) -> crate::EvmSigner {
        use ethers::signers::Signer;

        let provider = self.provider.clone();
        let wallet: LocalWallet = self.anvil.keys()[idx].clone().into();
        let client = SignerMiddleware::new(
            provider,
            wallet.clone().with_chain_id(self.anvil.chain_id()),
        );
        let client = Arc::new(client);

        crate::EvmSigner {
            signer: client,
            walelt: wallet,
        }
    }
}
