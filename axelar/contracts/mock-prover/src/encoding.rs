use crate::types::{BatchID, Command, CommandBatch, CommandType};
use connection_router::state::Message;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{HexBinary, StdError, Uint256};
use hex_literal::hex;
use sha3::{Digest, Keccak256};

#[cw_serde]
#[derive(Copy)]
pub enum Encoder {
    Abi,
    Bcs,
}

#[cw_serde]
pub struct Data {
    pub destination_chain_id: Uint256,
    pub commands: Vec<Command>,
}

impl Data {
    pub fn encode(&self, _encoder: Encoder) -> HexBinary {
        let mut bytes = Vec::new();
        for command in &self.commands {
            bytes.extend_from_slice(command.params.as_slice());
        }
        Keccak256::digest(bytes).as_slice().into()
    }
}

pub struct CommandBatchBuilder {
    message_ids: Vec<String>,
    // new_worker_set: Option<WorkerSet>,
    commands: Vec<Command>,
    destination_chain_id: Uint256,
    encoding: Encoder,
}

impl CommandBatchBuilder {
    pub fn new(destination_chain_id: Uint256, encoding: Encoder) -> Self {
        Self {
            message_ids: vec![],
            // new_worker_set: None,
            commands: vec![],
            destination_chain_id,
            encoding,
        }
    }

    pub fn add_message(&mut self, msg: Message) -> Result<(), StdError> {
        self.message_ids.push(msg.cc_id.to_string());
        self.commands.push(make_command(msg, self.encoding)?);
        Ok(())
    }

    pub fn build(self) -> Result<CommandBatch, StdError> {
        let data = Data {
            destination_chain_id: self.destination_chain_id,
            commands: self.commands,
        };

        let id = BatchID::new(&self.message_ids);

        Ok(CommandBatch {
            id,
            message_ids: self.message_ids,
            data,
            encoder: self.encoding,
        })
    }
}

fn make_command(msg: Message, _encoding: Encoder) -> Result<Command, StdError> {
    Ok(Command {
        ty: CommandType::ApproveContractCall,
        params: hex!("01020304").into(), // TODO: Mock prover uses constant data.
        id: command_id(msg.cc_id.to_string()),
    })
}

fn command_id(message_id: String) -> HexBinary {
    Keccak256::digest(message_id.as_bytes()).as_slice().into()
}
