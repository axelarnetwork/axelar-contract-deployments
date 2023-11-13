use super::*;
use shellexpand::tilde;
use std::{error::Error, rc::Rc};

pub fn setup_solana_client(payer_file_path: String) -> Result<Client<Rc<Keypair>>, Box<dyn Error>> {
    let payer = read_keypair_file(&*tilde(&payer_file_path))?;
    let payer = Rc::new(payer);
    let cluster = Cluster::Devnet;
    let client = Client::new_with_options(
        cluster.clone(),
        payer.clone(),
        CommitmentConfig::confirmed(),
    );
    Ok(client)
}
