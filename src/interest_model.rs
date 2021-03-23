// A mockup interest model
// TODO: make this in a separate contract and let someone manage this per each collaterized asset
// use cosmwasm_std::{Uint128};
// constant that needs to be managed
pub static  multiplier_per_block: u128 = 23; // 0.000000237823 * 10^8
pub static  base_rate_per_block: u128 = 0;
pub static  jump_multiplier_per_block: u128 = 51; // 0.000000518455 * 10^8;
pub static kink: u128 =  80_000_000; // 0.8 * 10^8
//pub static config_reserve_factor: u128 = 5_000_000; // 0.05 * 10^8

pub fn get_utilization_rate(cash: u128, borrows: u128, reserves: u128) -> u128 {
    if borrows == (0u128) {
        return 0;
    }
    borrows/((cash + borrows) - reserves)
}


pub fn get_borrow_rate(cash: u128, borrows: u128, reserves: u128) -> u128 {
    let util = get_utilization_rate(cash, borrows, reserves);

    if util <= kink {
        return util * multiplier_per_block + base_rate_per_block;
    } else {
        let normal_rate = kink * multiplier_per_block + base_rate_per_block;
        let excess_util = util - kink;
        return excess_util * jump_multiplier_per_block + normal_rate;
    }
}

pub fn get_supply_rate(cash: u128, borrows: u128, reserves: u128, reserve_factor: u128) -> u128 {
    let one_minus_reserve_factor = 1*10_i32.pow(8) as u128 - reserve_factor;
    let borrow_rate = get_borrow_rate(cash, borrows, reserves);
    let rate_to_pool = borrow_rate * one_minus_reserve_factor;
    get_utilization_rate(cash, borrows, reserves) * rate_to_pool
}
