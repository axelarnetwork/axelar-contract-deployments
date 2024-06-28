//! build.rs script to generate Rust types from the `devnet-amplifier.json`
//! script

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use quote::quote;
use serde_json::Value;

fn main() {
    let output_dir = std::env::var("OUT_DIR").unwrap();
    let output_file = PathBuf::from(output_dir).join("devnet_amplifier.rs");
    let amplifier_data = std::fs::read(
        workspace_root_dir()
            .join("xtask")
            .join("devnet-amplifier.json"),
    )
    .map(|x| serde_json::from_slice::<serde_json::Value>(&x))
    .expect("cannot read devnet amplifier data")
    .expect("cannot parse devnet amplifier data");

    let chains_tokens = generate_chains_tokens(&amplifier_data);
    let axelar_tokens = generate_axelar_tokens(&amplifier_data);

    let tokens = quote! {
        use std::collections::HashMap;

        #chains_tokens
        #axelar_tokens
    };

    let mut file = File::create(output_file).expect("Could not create output file");
    file.write_all(tokens.to_string().as_bytes())
        .expect("couldn't write parsed amplifier data");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=xtask/devnet-amplifier.json");
}

fn generate_chains_tokens(amplifier_data: &Value) -> proc_macro2::TokenStream {
    let chains = amplifier_data
        .get("chains")
        .expect("`chains` key must be present")
        .as_object()
        .expect("`chains` should be an object");

    let mut tokens = quote! {
        #[derive(Debug, Clone)]
        pub struct EvmChain {
            pub axelarId: String,
            pub chainId: u64,
            pub confirmations: u64,
            pub axelar_gateway: String,
            pub id: String,
            pub name: String,
            pub rpc: url::Url,
            pub tokenSymbol: String,
        }
    };
    let mut map = quote! {};

    for (chain_name, chain_data) in chains {
        let chain_name_lit = chain_name.as_str();
        let axelar_id = chain_data.get("axelarId").unwrap().as_str().unwrap();
        let chain_id = chain_data.get("chainId").unwrap().as_u64().unwrap();
        let confirmations = chain_data.get("confirmations").unwrap().as_u64().unwrap();
        let axelar_gateway = chain_data
            .get("contracts")
            .unwrap()
            .get("AxelarGateway")
            .unwrap()
            .get("address")
            .unwrap()
            .as_str()
            .unwrap();
        let id = chain_data.get("id").unwrap().as_str().unwrap();
        let name = chain_data.get("name").unwrap().as_str().unwrap();
        let rpc = chain_data.get("rpc").unwrap().as_str().unwrap();
        let token_symbol = chain_data.get("tokenSymbol").unwrap().as_str().unwrap();

        map = quote! {
            #map
            map.insert(
                #chain_name_lit,
                EvmChain {
                    axelarId: #axelar_id.to_string(),
                    chainId: #chain_id,
                    confirmations: #confirmations,
                    axelar_gateway: #axelar_gateway.to_string(),
                    id: #id.to_string(),
                    name: #name.to_string(),
                    rpc: url::Url::parse(#rpc).unwrap(),
                    tokenSymbol: #token_symbol.to_string(),
                },
            );
        };
    }

    tokens = quote! {
        #tokens
        pub fn get_chains() -> HashMap<&'static str, EvmChain> {
            let mut map = HashMap::new();
            #map
            map
        }
    };

    tokens
}

#[allow(clippy::too_many_lines)]
fn generate_axelar_tokens(amplifier_data: &Value) -> proc_macro2::TokenStream {
    let axelar = amplifier_data
        .get("axelar")
        .expect("`axelar` key must be present")
        .as_object()
        .expect("`axelar` should be an object");

    let contracts = axelar.get("contracts").unwrap().as_object().unwrap();
    let rpc = axelar.get("rpc").unwrap().as_str().unwrap();
    let lcd = axelar.get("lcd").unwrap().as_str().unwrap();
    let grpc = axelar.get("grpc").unwrap().as_str().unwrap();
    let token_symbol = axelar.get("tokenSymbol").unwrap().as_str().unwrap();
    let gas_price = axelar.get("gasPrice").unwrap().as_str().unwrap();
    let gas_limit = axelar.get("gasLimit").unwrap().as_str().unwrap();

    let mut multisig_prover_map = quote! {};
    let mut voting_verifier_map = quote! {};
    let mut gateway_map = quote! {};

    let multisig_prover = contracts.get("MultisigProver").unwrap();
    for (chain, data) in multisig_prover.as_object().unwrap() {
        if chain != "codeId" {
            let governance_address = data.get("governanceAddress").unwrap().as_str().unwrap();
            let destination_chain_id = data.get("destinationChainID").unwrap().as_str().unwrap();
            let service_name = data.get("serviceName").unwrap().as_str().unwrap();
            let encoder = data.get("encoder").unwrap().as_str().unwrap();
            let address = data.get("address").unwrap().as_str().unwrap();
            let domain_separator = data.get("domainSeparator").unwrap().as_str().unwrap();
            let key_type = data.get("keyType").unwrap().as_str().unwrap();

            multisig_prover_map = quote! {
                #multisig_prover_map
                multisig_prover_map.insert(
                    #chain.to_string(),
                    MultisigProver {
                        governance_address: #governance_address.to_string(),
                        destination_chain_id: #destination_chain_id.to_string(),
                        service_name: #service_name.to_string(),
                        encoder: #encoder.to_string(),
                        address: #address.to_string(),
                        domain_separator: #domain_separator.to_string(),
                        key_type: #key_type.to_string(),
                    },
                );
            };
        }
    }

    let voting_verifier = contracts.get("VotingVerifier").unwrap();
    for (chain, data) in voting_verifier.as_object().unwrap() {
        if chain != "codeId" {
            let governance_address = data.get("governanceAddress").unwrap().as_str().unwrap();
            let source_gateway_address =
                data.get("sourceGatewayAddress").unwrap().as_str().unwrap();
            let address = data.get("address").unwrap().as_str().unwrap();
            let msg_id_format = data.get("msgIdFormat").unwrap().as_str().unwrap();

            voting_verifier_map = quote! {
                #voting_verifier_map
                voting_verifier_map.insert(
                    #chain.to_string(),
                    VotingVerifier {
                        governance_address: #governance_address.to_string(),
                        source_gateway_address: #source_gateway_address.to_string(),
                        address: #address.to_string(),
                        msg_id_format: #msg_id_format.to_string(),
                    },
                );
            };
        }
    }

    let gateway = contracts.get("Gateway").unwrap();
    for (chain, data) in gateway.as_object().unwrap() {
        if chain != "codeId" {
            let address = data.get("address").unwrap().as_str().unwrap();

            gateway_map = quote! {
                #gateway_map
                gateway_map.insert(
                    #chain.to_string(),
                    Contract {
                        address: #address.to_string(),
                    },
                );
            };
        }
    }

    quote! {
        #[derive(Debug, Clone)]
        pub struct Contract {
            pub address: String,
        }

        #[derive(Debug, Clone)]
        pub struct Axelar {
            pub multisig_prover: HashMap<String, MultisigProver>,
            pub voting_verifier: HashMap<String, VotingVerifier>,
            pub gateway: HashMap<String, Contract>,
            pub rpc: url::Url,
            pub lcd: String,
            pub grpc: url::Url,
            pub tokenSymbol: String,
            pub gasPrice: String,
            pub gasLimit: String,
        }

        #[derive(Debug, Clone)]
        pub struct MultisigProver {
            pub governance_address: String,
            pub destination_chain_id: String,
            pub service_name: String,
            pub encoder: String,
            pub address: String,
            pub domain_separator: String,
            pub key_type: String,
        }

        #[derive(Debug, Clone)]
        pub struct VotingVerifier {
            pub governance_address: String,
            pub source_gateway_address: String,
            pub address: String,
            pub msg_id_format: String,
        }

        pub fn get_axelar() -> Axelar {
            let mut multisig_prover_map = HashMap::new();
            #multisig_prover_map

            let mut voting_verifier_map = HashMap::new();
            #voting_verifier_map

            let mut gateway_map = HashMap::new();
            #gateway_map

            Axelar {
                multisig_prover: multisig_prover_map,
                voting_verifier: voting_verifier_map,
                gateway: gateway_map,
                rpc: url::Url::parse(#rpc).unwrap(),
                lcd: #lcd.to_string(),
                grpc: url::Url::parse(#grpc).unwrap(),
                tokenSymbol: #token_symbol.to_string(),
                gasPrice: #gas_price.to_string(),
                gasLimit: #gas_limit.to_string(),
            }
        }
    }
}

fn workspace_root_dir() -> PathBuf {
    let dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| env!("CARGO_MANIFEST_DIR").to_owned());
    PathBuf::from(dir).parent().unwrap().to_owned()
}
