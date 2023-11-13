use anchor_client::anchor_lang::AnchorSerialize;
use anchor_client::solana_sdk::commitment_config::CommitmentConfig;
use anchor_client::solana_sdk::pubkey::Pubkey;
use anchor_client::solana_sdk::signature::{read_keypair_file, Signer};
use anchor_client::{Client, ClientError, Cluster};
use clap::Parser;
use env_logger;
use gateway::accounts::CallContract;
use gateway::id as gateway_program_id;
use gateway::instruction as gateway_instruction;
use log::info;
use shellexpand::tilde;
use std::ops::Deref;
use std::rc::Rc;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Payload which will be sent to Solana Gateway e.g. "1, 2, 3, 4"
    #[arg(short, long)]
    payload: String,

    /// Symbol of destination chain e.g. "eth"
    #[arg(long)]
    destination_chain: String,

    /// Address of contract on destination chain e.g. "0x999991888887653456765445676544567654567765"
    #[arg(long)]
    destination_contract_address: String,

    /// Account address to pay for Axelar TXs
    #[arg(short, long)]
    solana_payer_path: String,
}

fn main() {
    env_logger::init();
    let args = Args::parse();

    let payer = read_keypair_file(&*tilde(&args.solana_payer_path)).unwrap();
    let payer = Rc::new(payer);
    let cluster = Cluster::Devnet;
    let client = Client::new_with_options(
        cluster.clone(),
        payer.clone(),
        CommitmentConfig::confirmed(),
    );

    // kick it
    let _result = gateway_call_contract(
        &client,
        gateway_program_id(),
        payer.pubkey(),
        args.destination_chain,
        args.destination_contract_address,
        args.payload.try_to_vec().unwrap(),
    )
    .unwrap();
    // TODO: error handling
}

fn gateway_call_contract<C: Deref<Target = impl Signer> + Clone>(
    client: &Client<C>,
    program_id: Pubkey,
    sender_account_info: Pubkey,
    destination_chain: String,
    destination_contract_addr: String,
    payload: Vec<u8>,
) -> Result<(), ClientError> {
    let program = client.program(program_id)?;
    let signature = program
        .request()
        .accounts(CallContract {
            sender: sender_account_info, // INFO: Perhaps could be ommited #TBD
        })
        .args(gateway_instruction::CallContract {
            destination_chain: destination_chain,
            destination_contract_address: destination_contract_addr,
            payload: payload,
        })
        .send()?;

    info!("sent - check relayer log | txid: {}", signature);

    Ok(())
}
