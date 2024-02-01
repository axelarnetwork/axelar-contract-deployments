use borsh::BorshDeserialize;
use gateway::accounts::GatewayConfig;
use solana_program::hash::Hash;
use solana_program::pubkey::Pubkey;
use solana_program_test::{processor, BanksClient, ProgramTest, ProgramTestBanksClientExt};
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use solana_sdk::transaction::Transaction;

pub fn program_test() -> ProgramTest {
    // Add other programs here as needed

    let mut pt = ProgramTest::new(
        &env!("CARGO_PKG_NAME").replace('-', "_"),
        interchain_token_service::id(),
        processor!(interchain_token_service::processor::Processor::process_instruction),
    );
    pt.add_program(
        "gateway",
        gateway::id(),
        processor!(gateway::processor::Processor::process_instruction),
    );
    pt.add_program(
        "gas_service",
        gas_service::id(),
        processor!(gas_service::processor::Processor::process_instruction),
    );

    pt
}
pub struct TestFixture {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub recent_blockhash: Hash,
}

impl TestFixture {
    pub async fn new() -> TestFixture {
        let (banks_client, payer, recent_blockhash) = program_test().start().await;
        TestFixture {
            banks_client,
            payer,
            recent_blockhash,
        }
    }

    pub async fn refresh_blockhash(&mut self) -> Hash {
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();
        self.recent_blockhash
    }

    pub async fn init_gas_service(&mut self) -> Pubkey {
        let (root_pda_address, _) = gas_service::get_gas_service_root_pda();
        let ix =
            gas_service::instruction::create_initialize_root_pda_ix(self.payer.pubkey()).unwrap();
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();

        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );

        self.banks_client.process_transaction(tx).await.unwrap();

        let root_pda_data = self
            .banks_client
            .get_account(root_pda_address)
            .await
            .unwrap()
            .unwrap();
        let root_pda_data =
            gas_service::accounts::GasServiceRootPDA::try_from_slice(root_pda_data.data.as_slice())
                .unwrap();

        assert!(root_pda_data.check_authority(self.payer.pubkey().into()));

        root_pda_address
    }

    pub async fn initialize_gateway_config_account(
        &mut self,
        gateway_config: GatewayConfig,
    ) -> Pubkey {
        self.recent_blockhash = self
            .banks_client
            .get_new_latest_blockhash(&self.recent_blockhash)
            .await
            .unwrap();
        let (gateway_config_pda, _bump) = GatewayConfig::pda();

        let ix =
            gateway::instructions::initialize_config(self.payer.pubkey(), gateway_config.clone())
                .unwrap();
        let tx = Transaction::new_signed_with_payer(
            &[ix],
            Some(&self.payer.pubkey()),
            &[&self.payer],
            self.recent_blockhash,
        );

        self.banks_client.process_transaction(tx).await.unwrap();

        let account = self
            .banks_client
            .get_account(gateway_config_pda)
            .await
            .unwrap()
            .expect("metadata");

        assert_eq!(account.owner, gateway::id());
        let deserialized_gateway_config: GatewayConfig = borsh::from_slice(&account.data).unwrap();
        assert_eq!(deserialized_gateway_config, gateway_config);

        gateway_config_pda
    }
}
