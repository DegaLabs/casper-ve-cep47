use crate::{error::VeError, utils::require, vedata::IS_LOCKED};
use casper_contract::contract_api::{runtime, storage};
use crate::utils::{get_key, set_key};

pub fn when_not_locked() {
    let locked: bool = get_key(IS_LOCKED).unwrap();
    require(!locked, VeError::ContractLocked);
}

pub fn lock_contract() {
    set_key(IS_LOCKED, true);
}

pub fn unlock_contract() {
    set_key(IS_LOCKED, false);
}

pub fn init() {
    runtime::put_key(
        IS_LOCKED,
        storage::new_uref(false).into(),
    );
}