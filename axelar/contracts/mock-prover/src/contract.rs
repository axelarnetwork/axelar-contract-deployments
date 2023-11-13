use crate::{
    error::ContractError,
    execute,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query,
    state::{Config, CONFIG},
};
use connection_router::state::ChainName;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult,
};
use cw2::set_contract_version;
use std::str::FromStr;

// version info for migration info
const CONTRACT_NAME: &str = "axelar-solana:mock-prover";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = deps.api.addr_validate(&msg.admin_address)?;
    let gateway = deps.api.addr_validate(&msg.gateway_address)?;

    let config = Config {
        admin,
        gateway,
        destination_chain_id: msg.destination_chain_id,
        chain_name: ChainName::from_str(&msg.chain_name)
            .map_err(|_| ContractError::InvalidChainName)?,
        encoder: msg.encoder,
    };

    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, StdError> {
    match msg {
        ExecuteMsg::ConstructProof { message_ids } => {
            execute::construct_proof(deps, env, message_ids)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetProof {
            multisig_session_id,
        } => to_json_binary(&query::get_proof(deps, multisig_session_id)?),
    }
}

#[cfg(test)]
mod tests {
    use crate::encoding::Encoder;

    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, Addr, Attribute, Uint256};
    #[test]
    fn proper_initialization() {
        const CREATOR_ADDR: &str = "creator";
        const ADMIN_ADDR: &str = "admin";
        const GATEWAY_ADDR: &str = "gateway";
        const CHAIN_NAME: &str = "solana";
        const DEST_CHAIN_ID: Uint256 = Uint256::zero();
        const ENCODER: Encoder = Encoder::Bcs;

        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            admin_address: ADMIN_ADDR.into(),
            gateway_address: GATEWAY_ADDR.into(),
            destination_chain_id: DEST_CHAIN_ID,
            chain_name: CHAIN_NAME.into(),
            encoder: ENCODER,
        };
        let info = mock_info(CREATOR_ADDR, &coins(1000, "earth"));

        let response = instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        assert!(response
            .attributes
            .contains(&Attribute::new("method", "instantiate")));
        assert!(response
            .attributes
            .contains(&Attribute::new("owner", CREATOR_ADDR)));
        assert_eq!(
            CONFIG.load(&deps.storage).unwrap(),
            Config {
                admin: Addr::unchecked(ADMIN_ADDR),
                gateway: Addr::unchecked(GATEWAY_ADDR),
                destination_chain_id: DEST_CHAIN_ID,
                chain_name: CHAIN_NAME.parse().unwrap(),
                encoder: ENCODER,
            }
        )
    }
}
