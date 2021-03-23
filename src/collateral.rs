use cosmwasm_std::{
    log, Api, Env, Extern, HandleResponse,  Querier,
    StdError, StdResult, Storage, Uint128, BankMsg, CosmosMsg, Coin
};

use std::convert::TryInto;

use crate::state::{get_state, set_state, get_config, set_config, set_borrow_balance, get_borrow_balance, BorrowSnapshot};

use crate::interest_model::{get_borrow_rate};
use crate::exponential::truncate;
use crate::token::{mint_tokens, burn_tokens};

pub fn try_repay_borrow<S: Storage, A: Api, Q: Querier>(
    _deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {
    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "repay_borrow"),
            log("sender", env.message.sender.as_str()),
        ],
        data: None,
    };
    Ok(res)
}

pub fn try_borrow<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    borrow_amount: Uint128
) -> StdResult<HandleResponse> {

    accrue_interest(deps, env.clone())?;

    let current_block = env.block.height;
    let state = get_state(&deps.storage)?;
    if current_block != state.block_number {
        return Err(StdError::generic_err(format!(
            "Market is not fresh: current_block: {}, market_block: {}",
            current_block, state.block_number)
        )
        );
    }

    // TODO: get query from controller contract whether the sender is allowed to borrow

    // Check if the pool has enough balance to lend to the sender
    if state.cash < borrow_amount.u128() {
        return Err(StdError::generic_err(format!(
            "The lending pool has insufficient cash: redeem_amount: {}, pool_reserve: {}",
            borrow_amount, state.cash)
        )
        );
    }

    // Get borrow balance of the sender
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    // get borrow balance
    let account_borrow = get_account_borrow(deps, env.clone())?;
    let new_account_borrow = account_borrow + borrow_amount.u128();


    // Set new cash amount for contract
    let mut new_state = get_state(&deps.storage)?;
    new_state.cash = new_state.cash - borrow_amount.u128();
    new_state.total_borrows += borrow_amount.u128();
    set_state(&mut deps.storage, &new_state)?;

    // Set new borrow balance for the sender
    let new_borrow_balance = BorrowSnapshot {
        principal: new_account_borrow,
        interest_index: new_state.borrow_index
    };
    set_borrow_balance(&mut deps.storage, &sender_raw, Some(new_borrow_balance))?;

    // Transfer native token to the user
    // TODO: in this case it is hard coded to luna, include denom in contract config
    let native_transfer: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: env.message.sender.clone(),
        amount: vec![Coin {
            denom: "uluna".to_string(),
            amount: borrow_amount.clone(),
        }],
    });

    let res = HandleResponse {
        messages: vec![native_transfer],
        log: vec![
            log("action", "borrow"),
            log("sender", env.message.sender.as_str()),
            log("new_account_borrow", new_account_borrow.clone()),
            log("new_total_borrows", new_state.clone().total_borrows)
        ],
        data: None,
    };
    Ok(res)
}

pub fn try_mint<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
) -> StdResult<HandleResponse> {

    accrue_interest(deps, env.clone())?;

    let current_block = env.block.height;
    let state = get_state(&deps.storage)?;
    if current_block != state.block_number {
        return Err(StdError::generic_err(format!(
            "Market is not fresh: current_block: {}, market_block: {}",
            current_block, state.block_number)
        )
        );
    }


    // TODO: get query from controller contract whether the sender is allowed to borrow

    // Check native currency transfer
    let mint_amount = env.message.sent_funds[0].amount.u128();

    // Get exchange rate derived from borrow and reserve
    let exchange_rate = get_exchange_rate(deps, env.clone())?;

    let token_mint_amount = truncate(mint_amount * exchange_rate);

    // Set new config
    let mut new_config = get_config(&deps.storage)?;
    new_config.total_supply += token_mint_amount;

    // Set new cash amount for contract
    let mut new_state = get_state(&deps.storage)?;
    new_state.cash += mint_amount;
    set_state(&mut deps.storage, &new_state)?;

    set_config(&mut deps.storage, &new_config)?;

    // Mint token to the sender
    let recipient_address_raw = deps.api.canonical_address(&env.message.sender)?;
    mint_tokens(
        &mut deps.storage,
        &recipient_address_raw,
        token_mint_amount,
    )?;

    let res = HandleResponse {
        messages: vec![],
        log: vec![
            log("action", "mint"),
            log("sender", env.message.sender.as_str()),
            log("minted_amount", token_mint_amount.clone())
        ],
        data: None,
    };
    Ok(res)
}

pub fn try_redeem<S: Storage, A: Api, Q: Querier>(
    deps: &mut Extern<S, A, Q>,
    env: Env,
    redeem_tokens_in: Uint128
) -> StdResult<HandleResponse> {
    accrue_interest(deps, env.clone())?;

    let current_block = env.block.height;
    let state = get_state(&deps.storage)?;
    if current_block != state.block_number {
        return Err(StdError::generic_err(format!(
            "Market is not fresh: current_block: {}, market_block: {}",
            current_block, state.block_number)
        )
        );
    }

    // TODO: get query from controller contract whether the sender is allowed to borrow

    // Get exchange rate derived from borrow and reserve
    let exchange_rate = get_exchange_rate(deps, env.clone())?;

    let redeem_native_in = env.message.sent_funds[0].amount.u128();
    if redeem_tokens_in.u128() != 0 && redeem_native_in != 0 {
        return Err(StdError::generic_err(format!(
            "one of redeeming tokens or asset must be 0: redeeming_cw20: {}, redeeming_native_currency: {}",
            redeem_tokens_in.u128(), redeem_native_in)
        )
        );
    }
    // Calculate redeem amount
    let (redeem_native, redeem_tokens, new_state) = match redeem_tokens_in.u128() {
        x if x > 0 => {
            // Set new cash amount for contract
            let mut new_state = state.clone();
            let redeem_native = truncate(exchange_rate * 100_000_000 * redeem_tokens_in.u128());
            new_state.cash = new_state.cash - redeem_native;
            (redeem_native, redeem_tokens_in.u128(), new_state)
        },
        _ => {
            // Set new cash amount for contract
            let mut new_state = state.clone();
            new_state.cash +=redeem_native_in;
            (redeem_native_in, truncate(redeem_native_in / (100_000_000 * exchange_rate)), new_state)
        }
    };

    // Set new state for cash
    set_state(&mut deps.storage, &new_state)?;

    // Set new config
    let mut new_config = get_config(&deps.storage)?;
    new_config.total_supply = new_config.total_supply - redeem_tokens;
    set_config(&mut deps.storage, &new_config)?;



    // Burn token to the sender
    let recipient_address_raw = deps.api.canonical_address(&env.message.sender)?;
    burn_tokens(
        &mut deps.storage,
        &recipient_address_raw,
        redeem_tokens,
    )?;

    // Check if the pool has enough balance
    if state.cash < redeem_native {
        return Err(StdError::generic_err(format!(
            "The lending pool has insufficient cash: redeem_amount: {}, pool_reserve: {}",
            redeem_native, state.cash)
        )
        );
    }

    // Transfer native token to the user
    // TODO: in this case it is hard coded to luna, include denom in contract config
    let native_transfer: CosmosMsg = CosmosMsg::Bank(BankMsg::Send {
        from_address: env.contract.address.clone(),
        to_address: env.message.sender.clone(),
        amount: vec![Coin {
            denom: "uluna".to_string(),
            amount: Uint128::from(redeem_native.clone()),
        }],
    });


    // TODO: write defense hook

    let res = HandleResponse {
        messages: vec![
            native_transfer
        ],
        log: vec![
            log("action", "redeem"),
            log("sender", env.message.sender.as_str()),
            log("redeem_tokens", redeem_tokens.clone()),
            log("redeem_native", redeem_native.clone())
        ],
        data: None,
    };
    Ok(res)
}

fn accrue_interest<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>, env: Env) -> StdResult<()>  {
    let prior_state = get_state(&deps.storage)?;

    let borrow_rate = get_borrow_rate(prior_state.cash, prior_state.total_borrows, prior_state.total_reserves);

    if borrow_rate > prior_state.max_borrow_rate {
        return Err(StdError::generic_err(format!(
            "borrow rate is absurdly high: borrow_rate: {}, max_borrow_rate: {}",
            borrow_rate, prior_state.max_borrow_rate)
        )
        );
    }

    let current_block = env.block.height;

    let block_delta: u128 = (prior_state.block_number - current_block).try_into().unwrap();

    // Calculate the interest accumulated into borrows and reserves and the new index:
    let simple_interest_factor = borrow_rate * block_delta;

    let accumulated_interest = truncate(simple_interest_factor * prior_state.total_borrows);
    let new_total_borrows = accumulated_interest + prior_state.total_borrows;
    let new_total_reserves = truncate(accumulated_interest * prior_state.reserve_factor) + prior_state.total_reserves;
    let new_borrow_index = truncate(simple_interest_factor * prior_state.borrow_index) + prior_state.borrow_index;

    // Set new state
    let mut new_state = get_state(&deps.storage)?;
    new_state.block_number = env.block.height;
    new_state.borrow_index = new_borrow_index;
    new_state.total_borrows = new_total_borrows;
    new_state.total_reserves = new_total_reserves;

    set_state(&mut deps.storage, &new_state)?;


    Ok(())
}

fn get_exchange_rate<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>, _env: Env) -> StdResult<u128> {
    let config = get_config(&deps.storage)?;

    // if total supply is zero
    if config.total_supply == 0u128 {
        return Ok(config.initial_exchange_rate);
    }
    // else calculate exchange rate
    let prior_state = get_state(&deps.storage)?;

    let total_cash = prior_state.cash;

    let cash_plus_borrows_minus_reserves = total_cash + prior_state.total_borrows - prior_state.total_reserves;

    let exchange_rate = cash_plus_borrows_minus_reserves / config.total_supply * 100_000_000;


    Ok(exchange_rate)
}


fn get_account_borrow<S: Storage, A: Api, Q: Querier>(deps: &mut Extern<S, A, Q>, env: Env) -> StdResult<u128> {
    let sender_raw = deps.api.canonical_address(&env.message.sender)?;
    let snapshot = get_borrow_balance(&deps.storage, &sender_raw);

    let borrow_snapshot = match snapshot {
        Some(s) => {
            s
        },
        None => {
            let init_borrow_snapshot = BorrowSnapshot {
                principal: 0u128,
                interest_index: 0u128
            };
            set_borrow_balance(&mut deps.storage, &sender_raw, Some(init_borrow_snapshot.clone()))?;
            init_borrow_snapshot
        }
    };

    if borrow_snapshot.principal == 0u128 {
        // if borrow balance is 0, then borrow index is likey also 0.
        // Hence, return 0 in this case.
        return Ok(0)
    }

    let state = get_state(&deps.storage)?;
    let principal_time_index = borrow_snapshot.principal * state.borrow_index;
    return Ok(principal_time_index / borrow_snapshot.interest_index);
}
