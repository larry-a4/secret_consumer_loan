use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{HumanAddr, Uint128};
//use cosmwasm_std::{Binary, CosmosMsg, HumanAddr, Querier, StdResult, Uint128};

//use secret_toolkit::snip20::{register_receive_msg, token_info_query, transfer_msg, TokenInfo};

//use crate::contract::BLOCK_SIZE;


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InitMsg {
    pub name: String,
    pub total_supply: Uint128,
    pub decimals: u8,
    pub symbol: String,
    pub initial_exchange_rate: Uint128,
    pub reserve_factor: Uint128,
    pub borrow_index: Uint128,
    pub max_borrow_rate: Uint128,
    pub denom: String
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleMsg {
    Mint {},
    Redeem {
        redeem_tokens_in: Uint128
    },
    Borrow {
        borrow_amount: Uint128
    },
    RepayBorrow {},
    Approve {
        spender: HumanAddr,
        amount: Uint128,
    },
    Transfer {
        recipient: HumanAddr,
        amount: Uint128,
    },
    TransferFrom {
        owner: HumanAddr,
        recipient: HumanAddr,
        amount: Uint128,
    },
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HandleAnswer {

}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Balance {
        address: HumanAddr,
    },
    Allowance {
        owner: HumanAddr,
        spender: HumanAddr,
    },
}


/// responses to queries
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryAnswer {
    /// Config query response
    ConfigResponse {
        name: String,
        total_supply: Uint128,
        decimals: u8,
        symbol: String,
        intital_exchange_rate: Uint128,
        reserve_factor: Uint128,
        borrow_index: Uint128,
        denom: String
    },
    /// Balance query response
    BalanceResponse {
        balance: Uint128,
    },
    /// Allowance query response
    AllowanceResponse {
        allowance: Uint128,
    },
}

/// success or failure response
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub enum ResponseStatus {
    Success,
    Failure,
}

