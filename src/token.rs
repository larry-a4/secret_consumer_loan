use cosmwasm_std::{
    log, Api, Binary, CanonicalAddr, Env, Extern, HandleResponse, HumanAddr, InitResponse, Querier,
    ReadonlyStorage, StdError, StdResult, Storage, Uint128,
};
use cosmwasm_storage::{PrefixedStorage, ReadonlyPrefixedStorage};

//use std::convert::TryInto;

use crate::state::{
    get_allowance, get_balance, set_allowance, set_balance
};

pub fn try_transfer<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    recipient: &HumanAddr,
    amount: &Uint128,
) -> StdResult<HandleResponse> {
    let sender_address_raw = deps.api.canonical_address(recipient)?;
    let recipient_address_raw = deps.api.canonical_address(recipient)?;
    let amount_raw = amount.u128();

    perform_transfer(
        &mut deps.storage,
        &sender_address_raw,
        &recipient_address_raw,
        amount_raw,
    )?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "transfer"),
            log("sender", env.message.sender.as_str()),
            log("recipient", recipient.as_str()),
        ],
        data: None,
    };
    Ok(res)
}

pub fn try_transfer_from<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    owner: &HumanAddr,
    recipient: &HumanAddr,
    amount: &Uint128,
) -> StdResult<HandleResponse> {
    let spender_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let owner_address_raw = deps.api.canonical_address(owner)?;
    let recipient_address_raw = deps.api.canonical_address(recipient)?;
    let amount_raw = amount.u128();

    let mut allowance = get_allowance(&deps.storage, &owner_address_raw, &spender_address_raw)?;
    if allowance < amount_raw {
        return Err(StdError::generic_err(format!(
            "Insufficient allowance: allowance={}, required={}",
            allowance, amount_raw
        )));
    }
    allowance -= amount_raw;
    set_allowance(
        &mut deps.storage,
        &owner_address_raw,
        &spender_address_raw,
        allowance,
    )?;
    perform_transfer(
        &mut deps.storage,
        &owner_address_raw,
        &recipient_address_raw,
        amount_raw,
    )?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "transfer_from"),
            log("spender", env.message.sender.as_str()),
            log("sender", owner.as_str()),
            log("recipient", recipient.as_str()),
        ],
        data: None,
    };
    Ok(res)
}

pub fn try_approve<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    spender: &HumanAddr,
    amount: &Uint128,
) -> StdResult<HandleResponse> {
    let owner_address_raw = deps.api.canonical_address(&env.message.sender)?;
    let spender_address_raw = deps.api.canonical_address(spender)?;
    set_allowance(
        &mut deps.storage,
        &owner_address_raw,
        &spender_address_raw,
        amount.u128(),
    )?;
    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "approve"),
            log("owner", env.message.sender.as_str()),
            log("spender", spender.as_str()),
        ],
        data: None,
    };
    Ok(res)
}

fn perform_transfer<T: Storage>(
    store: &mut T,
    from: &CanonicalAddr,
    to: &CanonicalAddr,
    amount: u128,
) -> StdResult<()> {
    let mut from_balance = get_balance(store, from);
    if from_balance < amount {
        return Err(StdError::generic_err(format!(
            "Insufficient funds: sender={}, balance={}, required={}",
            HumanAddr::from(from.to_string()),
            from_balance,
            amount
        )));
    }
    from_balance -= amount;
    set_balance(store, from, from_balance);

    let mut to_balance = get_balance(store, to);
    to_balance += amount;
    set_balance(store, to, to_balance);

    Ok(())
}

pub fn mint_tokens<T: Storage>(
    store: &mut T,
    to: &CanonicalAddr,
    amount: u128,
) -> StdResult<()> {
    let mut to_balance = get_balance(store, to);
    to_balance += amount;
    set_balance(store, to, to_balance);

    Ok(())
}


pub fn burn_tokens<T: Storage>(
    store: &mut T,
    to: &CanonicalAddr,
    amount: u128,
) -> StdResult<()> {
    let mut to_balance = get_balance(store, to);
    to_balance -= amount;
    set_balance(store, to, to_balance);

    Ok(())
}
