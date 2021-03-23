//use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Api, Env, Extern, HandleResponse, HumanAddr,
    InitResponse, InitResult, Querier, QueryResult, StdResult, Storage, Uint128,
};

//use std::collections::HashSet;

//use serde_json_wasm as serde_json;

use secret_toolkit::utils::{pad_handle_result, pad_query_result};

use crate::msg::{
    HandleMsg, InitMsg, QueryAnswer, QueryMsg,
};
use crate::state::{
    save, get_allowance, get_balance ,get_config, Config, State,
    CONFIG_KEY, STATE_KEY,
};

use crate::{collateral, token};



/// pad handle responses and log attributes to blocks of 256 bytes to prevent leaking info based on
/// response size
pub const BLOCK_SIZE: usize = 256;

pub fn init<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: InitMsg,
) -> InitResult {
    let init_config = Config {
        name: msg.name,
        total_supply: msg.total_supply.u128(),
        decimals: msg.decimals,
        symbol: msg.symbol,
        denom: msg.denom,
        initial_exchange_rate: msg.initial_exchange_rate.u128(),
        reserve_factor: msg.reserve_factor.u128(),
        max_borrow_rate: msg.max_borrow_rate.u128(),
        borrow_index:msg.borrow_index.u128(),
    };
    save(&mut deps.storage, CONFIG_KEY, &init_config)?;

    let init_state = State {
        cash: 0u128,
        block_number: env.block.height,
        total_reserves: 0u128,
        total_borrows: 0u128,
        exchange_rate: init_config.initial_exchange_rate,
        reserve_factor: init_config.reserve_factor,
        max_borrow_rate: init_config.max_borrow_rate,
        borrow_index: init_config.borrow_index,
    };
    save(&mut deps.storage, STATE_KEY, &init_state)?;

    Ok(InitResponse::default())
}

pub fn handle<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    msg: HandleMsg,
) -> StdResult<HandleResponse> {
    let response = match msg {
        HandleMsg::Mint {} => collateral::try_mint(deps, env),
        HandleMsg::Redeem { redeem_tokens_in } => {
            collateral::try_redeem(deps, env, redeem_tokens_in)
        },
        HandleMsg::Borrow { borrow_amount } => {
            collateral::try_borrow(deps, env, borrow_amount)
        },
        HandleMsg::RepayBorrow {} => collateral::try_repay_borrow(deps, env),
        HandleMsg::Approve { spender,amount } => {
            token::try_approve(deps, env, &spender, &amount)
        },
        HandleMsg::Transfer { recipient, amount } => {
            token::try_transfer(deps, env, &recipient, &amount)
        },
        HandleMsg::TransferFrom { owner, recipient, amount } => {
            token::try_transfer_from(deps, env, &owner, &recipient, &amount)
        },
    };
    pad_handle_result(response, BLOCK_SIZE)
}

pub fn query<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>, msg: QueryMsg) -> QueryResult {
    let response = match msg {
        QueryMsg::Config {} => try_query_config(deps),
        QueryMsg::Balance {
            address,
        } => try_query_balance(deps, &address),
        QueryMsg::Allowance {
            owner,
            spender,
        } => try_query_allowance(deps, &owner, &spender),
    };
    pad_query_result(response, BLOCK_SIZE)
}

fn try_query_config<S: Storage, A: Api, Q: Querier>(deps: &Extern<S, A, Q>) -> QueryResult {
    let config: Config = get_config(&deps.storage)?;
    to_binary(&QueryAnswer::ConfigResponse {
        name: config.name,
        total_supply: Uint128::from(config.total_supply),
        decimals: config.decimals,
        symbol: config.symbol,
        intital_exchange_rate: Uint128::from(config.initial_exchange_rate),
        reserve_factor: Uint128::from(config.reserve_factor),
        borrow_index: Uint128::from(config.borrow_index),
        denom: config.denom,
    })
}

fn try_query_balance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    address: &HumanAddr,
) -> QueryResult {
    let address_key = deps.api.canonical_address(address)?;
    let balance: u128 = get_balance(&deps.storage, &address_key)?;
    to_binary(&QueryAnswer::BalanceResponse{
        balance: Uint128::from(balance)
    })
}

fn try_query_allowance<S: Storage, A: Api, Q: Querier>(
    deps: &Extern<S, A, Q>,
    owner: &HumanAddr,
    spender: &HumanAddr,
) -> QueryResult {
    let owner_key = deps.api.canonical_address(&owner)?;
    let spender_key = deps.api.canonical_address(&spender)?;
    let allowance = get_allowance(&deps.storage, &owner_key, &spender_key)?;
    to_binary(&QueryAnswer::AllowanceResponse {
        allowance: Uint128::from(allowance),
    })
}
