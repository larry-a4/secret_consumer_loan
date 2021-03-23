use std::{any::type_name};
use std::convert::TryFrom;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use cosmwasm_std::{CanonicalAddr, ReadonlyStorage, StdError, StdResult, Storage};
use cosmwasm_storage::{ReadonlyPrefixedStorage, PrefixedStorage, Bucket, ReadonlyBucket};

use secret_toolkit::serialization::{Bincode2, Serde};
//use secret_toolkit::storage::{AppendStore, AppendStoreMut, TypedStore, TypedStoreMut};

/// storage key for contract state
pub const CONFIG_KEY: &[u8] = b"config";
pub const STATE_KEY: &[u8] = b"state";
pub const ALLOWANCE_PREFIX: &[u8] = b"allowance";
pub const BALANCE_PREFIX: &[u8] = b"balance";
pub const BORROW_PREFIX: &[u8] = b"borrow";

/// Config struct
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Config {
    pub name: String,
    pub total_supply: u128,
    pub decimals: u8,
    pub symbol: String,
    pub initial_exchange_rate: u128,
    pub reserve_factor: u128,
    pub borrow_index: u128,
    pub max_borrow_rate: u128,
    pub denom: String,
}

/// state of the auction
#[derive(Serialize, Deserialize, Clone)]
pub struct State {
    pub cash: u128,
    pub block_number: u64,
    pub total_reserves: u128,
    pub total_borrows: u128,
    pub exchange_rate: u128,
    pub reserve_factor: u128,
    pub max_borrow_rate: u128,
    pub borrow_index: u128,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct BorrowSnapshot {
    pub principal: u128,
    pub interest_index: u128
}

/// Returns StdResult<()> resulting from saving an item to storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item should go to
/// * `key` - a byte slice representing the key to access the stored item
/// * `value` - a reference to the item to store
pub fn save<T: Serialize, S: Storage>(storage: &mut S, key: &[u8], value: &T) -> StdResult<()> {
    storage.set(key, &Bincode2::serialize(value)?);
    Ok(())
}

/// Removes an item from storage
///
/// # Arguments
///
/// * `storage` - a mutable reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn remove<S: Storage>(storage: &mut S, key: &[u8]) {
    storage.remove(key);
}

/// Returns StdResult<T> from retrieving the item with the specified key.  Returns a
/// StdError::NotFound if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn load<T: DeserializeOwned, S: ReadonlyStorage>(storage: &S, key: &[u8]) -> StdResult<T> {
    Bincode2::deserialize(
        &storage
            .get(key)
            .ok_or_else(|| StdError::not_found(type_name::<T>()))?,
    )
}

/// Returns StdResult<Option<T>> from retrieving the item with the specified key.
/// Returns Ok(None) if there is no item with that key
///
/// # Arguments
///
/// * `storage` - a reference to the storage this item is in
/// * `key` - a byte slice representing the key that accesses the stored item
pub fn may_load<T: DeserializeOwned, S: ReadonlyStorage>(
    storage: &S,
    key: &[u8],
) -> StdResult<Option<T>> {
    match storage.get(key) {
        Some(value) => Bincode2::deserialize(&value).map(Some),
        None => Ok(None),
    }
}

/// Config singleton initialization
/*
pub fn config<S: Storage>(storage: &mut S) -> Singleton<S, Config> {
    singleton(storage, CONFIG_PREFIX)
}
 */

/// Get config
pub fn get_config<S: Storage>(storage: &S) -> StdResult<Config> {
    load(storage, CONFIG_KEY)
}

/// Set config
pub fn set_config<S: Storage>(storage: &mut S, config: &Config) -> StdResult<()> {
    save(storage, CONFIG_KEY, config)
}

/// Get exchange rate
pub fn get_state<S: Storage>(storage: &S) -> StdResult<State> {
    load(storage, STATE_KEY)
}

/// Set exchange rate
pub fn set_state<S: Storage>(storage: &mut S, state: &State) -> StdResult<()> {
    save(storage, STATE_KEY, state)
}

pub fn get_balance<S: Storage>(store: &S, owner: &CanonicalAddr) -> StdResult<u128> {
    let balance_store = ReadonlyPrefixedStorage::new(BALANCE_PREFIX, store);
    load(&balance_store, owner.as_slice())
}

pub fn set_balance<S: Storage>(store: &mut S, owner: &CanonicalAddr, balance: u128) -> StdResult<()> {
    let mut balance_store = PrefixedStorage::new(BALANCE_PREFIX, store);
    save(&mut balance_store, owner.as_slice(), &balance)
}

pub fn get_allowance<S: Storage>(
    store: &S,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr
) -> StdResult<u128> {
    let owner_store =
        ReadonlyPrefixedStorage::multilevel(&[ALLOWANCE_PREFIX, owner.as_slice()], store);
    load(&owner_store, spender.as_slice())
}


/// Set allowance from address
pub fn set_allowance<S: Storage>(
    store: &mut S,
    owner: &CanonicalAddr,
    spender: &CanonicalAddr,
    amount: u128,
) -> StdResult<()> {
    let mut owner_store =
        PrefixedStorage::multilevel(&[ALLOWANCE_PREFIX, owner.as_slice()], store);
    save(&mut owner_store, spender.as_slice(), &amount)
}

pub fn get_borrow_balance<S: Storage>(store: &S, owner: &CanonicalAddr) -> Option<BorrowSnapshot> {
    match ReadonlyBucket::new(BORROW_PREFIX, store).may_load(owner.as_slice()) {
        Ok(Some(wrapped_reserves)) => Some(wrapped_reserves),
        _ => None,
    }
}

pub fn set_borrow_balance<S: Storage>(
    store: &mut S,
    owner: &CanonicalAddr,
    snapshot: Option<BorrowSnapshot>,
) -> StdResult<()> {
    match Bucket::new(BORROW_PREFIX, store).save(owner.as_slice(), &snapshot) {
        Ok(_) => Ok(()),
        Err(_) => Err(StdError::generic_err(format!(
            "Failed to write to the borrow_balance. key: {:?}, value: {:?}",
            owner, snapshot
        ))),
    }
}

// Helpers

/// Converts 16 bytes value into u128
/// Errors if data found that is not 16 bytes
fn bytes_to_u128(data: &[u8]) -> StdResult<u128> {
    match <[u8; 16]>::try_from(data) {
        Ok(bytes) => Ok(u128::from_be_bytes(bytes)),
        Err(_) => Err(StdError::generic_err(
            "Corrupted data found. 16 byte expected.",
        )),
    }
}
