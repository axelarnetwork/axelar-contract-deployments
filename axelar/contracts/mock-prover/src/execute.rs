use crate::{
    encoding::CommandBatchBuilder,
    events::ProverEvent,
    multisig_imports::{KeyID, MsgToSign, MultiSigEvent},
    state::{COMMANDS_BATCH, CONFIG, MULTISIG_SESSION_BATCH},
    types::BatchID,
};

use connection_router::state::{ChainName, CrossChainId, Message};
use cosmwasm_std::{
    to_json_binary, Addr, DepsMut, Env, Event, QuerierWrapper, QueryRequest, Response, StdError,
    Uint64, WasmQuery,
};
use std::collections::HashMap;

pub fn construct_proof(
    deps: DepsMut,
    env: Env,
    message_ids: Vec<String>,
) -> Result<Response, StdError> {
    let config = CONFIG.load(deps.storage)?;
    let batch_id = BatchID::new(&message_ids);

    let messages = get_messages(
        deps.querier,
        message_ids,
        config.gateway.clone(),
        config.chain_name.clone(),
    )?;

    // For debugging purposes, we will also Emit events with the the message ids.
    let message_id_events: Vec<Event> = messages
        .iter()
        .map(|msg| Event::new("message_from_gateway").add_attribute("id", msg.cc_id.to_string()))
        .collect();

    let session_id = Uint64::zero();
    MULTISIG_SESSION_BATCH.save(deps.storage, session_id.into(), &batch_id)?;

    // This would be used in the multisig submessage reply.
    let _command_batch = match COMMANDS_BATCH.may_load(deps.storage, &batch_id)? {
        Some(batch) => batch,
        None => {
            let mut builder = CommandBatchBuilder::new(config.destination_chain_id, config.encoder);

            for msg in messages {
                builder.add_message(msg)?;
            }
            let batch = builder.build()?;

            COMMANDS_BATCH.save(deps.storage, &batch.id, &batch)?;

            batch
        }
    };

    // TODO: The referential prover contract would encode the command batch and send a submessage to
    // the multisig contract to start a signign session. Since we are skipping the multisig in this
    // implementation, we will instead emit [multisig::{SigningStarted, SigningCompleted}] events
    // that would be emitted by the referential multisig contract.
    // The main difference is that the [multisig::SigningCompleted] event is expected to be found at
    // the results of a transaction sent to the multisig contract, not to this one.
    let signing_started_event = MultiSigEvent::SigningStarted {
        session_id,
        key_id: KeyID {
            owner: Addr::unchecked("foo"),
            subkey: "bar".into(),
        },
        pub_keys: HashMap::new(),
        msg: MsgToSign::unchecked(batch_id.inner().clone()),
    };

    let proof_under_construction_event = ProverEvent::ProofUnderConstruction {
        command_batch_id: batch_id,
        multisig_session_id: session_id,
    };

    let signing_completed_event = MultiSigEvent::SigningCompleted {
        session_id,
        completed_at: env.block.height,
    };

    Ok(Response::new()
        .add_event(signing_started_event.into())
        .add_event(proof_under_construction_event.into())
        .add_event(signing_completed_event.into())
        .add_events(message_id_events))
}

/// Copied from the referential multisig-prover contract.
fn get_messages(
    querier: QuerierWrapper,
    message_ids: Vec<String>,
    gateway: Addr,
    chain_name: ChainName,
) -> Result<Vec<Message>, StdError> {
    let length = message_ids.len();

    let ids = message_ids
        .into_iter()
        .map(|id| {
            id.parse::<CrossChainId>()
                .expect("ids should have correct format")
        })
        .collect::<Vec<_>>();
    let query = gateway::msg::QueryMsg::GetMessages { message_ids: ids };
    let messages: Vec<Message> = querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: gateway.into(),
        msg: to_json_binary(&query)?,
    }))?;

    assert!(
        messages.len() == length,
        "violated invariant: returned gateway messages count mismatch"
    );

    if messages
        .iter()
        .any(|msg| msg.destination_chain != chain_name)
    {
        panic!("violated invariant: messages from different chain found");
    }

    Ok(messages)
}
