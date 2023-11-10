use crate::{
    msg::{GetProofResponse, ProofStatus},
    state::{COMMANDS_BATCH, CONFIG, MULTISIG_SESSION_BATCH},
};
use cosmwasm_std::{Deps, StdResult, Uint64};

pub fn get_proof(deps: Deps, multisig_session_id: Uint64) -> StdResult<GetProofResponse> {
    let config = CONFIG.load(deps.storage)?;
    let batch_id = MULTISIG_SESSION_BATCH.load(deps.storage, multisig_session_id.u64())?;
    let batch = COMMANDS_BATCH.load(deps.storage, &batch_id)?;
    // TODO: assert_eq!(batch.encoder, config.encoder);

    let status = ProofStatus::Completed {
        execute_data: batch.data.encode(config.encoder),
    };

    Ok(GetProofResponse {
        multisig_session_id,
        message_ids: batch.message_ids,
        data: batch.data,
        status,
    })
}
