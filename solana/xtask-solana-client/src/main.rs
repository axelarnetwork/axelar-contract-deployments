use std::error::Error;
use std::path::PathBuf;

use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::keypair::read_keypair_file;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Payload which will be sent to Solana Gateway e.g. "1, 2, 3, 4"
    #[arg(short, long)]
    payload: String,

    /// Symbol of destination chain e.g. "eth"
    #[arg(long)]
    destination_chain: String,

    /// Address of contract on destination chain e.g.
    /// "0x999991888887653456765445676544567654567765"
    #[arg(long)]
    destination_contract_address: String,

    /// Account address to pay for Axelar TXs
    #[arg(short, long)]
    solana_payer_path: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let args = Args::parse();
    let payer = read_keypair_file(&args.solana_payer_path).unwrap();
    let devnet_cluster = "https://api.devnet.solana.com";
    let client = RpcClient::new(devnet_cluster);

    // kick it
    gateway_call_contract(
        &client,
        &payer,
        &args.destination_chain,
        &args.destination_contract_address,
        args.payload.as_ref(),
    )
}

fn gateway_call_contract(
    _client: &RpcClient,
    _payer: &Keypair,
    _destination_chain: &str,
    _destination_contract_address: &str,
    _payload: &[u8],
) -> Result<(), Box<dyn Error>> {
    unimplemented!("gateway_call_contract");
    // let _ix = gateway::instruction::call_contract(
    //     &gateway::id(),
    //     &payer.pubkey(),
    //     destination_chain,
    //     destination_contract_address,
    //     payload,
    // )?;
    // let latest_blockhash = client.get_latest_blockhash()?;

    // let tx = Transaction::new_signed_with_payer(
    //     &[ix],
    //     Some(&payer.pubkey()),
    //     &[payer],
    //     latest_blockhash,
    // );

    // let signature = client.send_and_confirm_transaction(&tx)?;

    // info!("sent - check relayer log | txid: {}", signature);
    // Ok(())
}
