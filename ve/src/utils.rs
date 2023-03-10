use crate::error::VeError;
use alloc::string::String;
use alloc::string::{ToString};
use core::convert::TryInto;
use casper_contract::{
    contract_api::{runtime, storage},
    unwrap_or_revert::UnwrapOrRevert,
};
use casper_types::{Key, account::AccountHash, bytesrepr::{FromBytes, ToBytes}, CLTyped, ApiError};
use casper_types::{system::CallStackElement};

pub fn get_key<T: FromBytes + CLTyped>(name: &str) -> Option<T> {
    match runtime::get_key(name) {
        None => None,
        Some(value) => {
            let key = value.try_into().unwrap_or_revert();
            let result = storage::read(key).unwrap_or_revert().unwrap_or_revert();
            Some(result)
        }
    }
}

pub fn set_key<T: ToBytes + CLTyped>(name: &str, value: T) {
    match runtime::get_key(name) {
        Some(key) => {
            let key_ref = key.try_into().unwrap_or_revert();
            storage::write(key_ref, value);
        }
        None => {
            let key = storage::new_uref(value).into();
            runtime::put_key(name, key);
        }
    }
}

// Helper functions
pub fn get_self_key() -> Key {
    get_last_call_stack_item()
        .map(call_stack_element_to_key).unwrap_or_revert()
}

fn get_last_call_stack_item() -> Option<CallStackElement> {
    let call_stack = runtime::get_call_stack();
    call_stack.into_iter().rev().nth(0)
}

/// Gets the immediate call stack element of the current execution.
fn get_immediate_call_stack_item() -> Option<CallStackElement> {
    let call_stack = runtime::get_call_stack();
    call_stack.into_iter().rev().nth(1)
}

/// Returns address based on a [`CallStackElement`].
///
/// For `Session` and `StoredSession` variants it will return account hash, and for `StoredContract`
/// case it will use contract hash as the address.
fn call_stack_element_to_key(call_stack_element: CallStackElement) -> Key {
    match call_stack_element {
        CallStackElement::Session { account_hash } => Key::from(account_hash),
        CallStackElement::StoredSession { account_hash, .. } => {
            // Stored session code acts in account's context, so if stored session wants to interact
            // with an ERC20 token caller's address will be used.
            Key::from(account_hash)
        }
        CallStackElement::StoredContract {
            contract_package_hash,
            ..
        } => Key::from(contract_package_hash),
    }
}

pub fn get_immediate_caller_key() -> Key {
    get_immediate_call_stack_item()
        .map(call_stack_element_to_key)
        .unwrap_or_revert()
}

pub fn require(v: bool, e: VeError) {
    if !v {
        runtime::revert(e);
    }
}

pub fn is_null(k: Key) -> bool {
    let null_bytes: [u8; 32] = vec![0u8; 32].try_into().unwrap();
    k.to_bytes().unwrap() == null_bytes
}

pub fn null_key() -> Key {
    let null_bytes: [u8; 32] = vec![0u8; 32].try_into().unwrap();
    Key::from(AccountHash::new(null_bytes))
}

pub fn is_not_null(k: Key) -> bool {
    !is_null(k)
}

pub fn key_to_str(key: &Key) -> String {
    match key {
        Key::Account(account) => account.to_string(),
        Key::Hash(package) => hex::encode(package),
        _ => runtime::revert(ApiError::UnexpectedKeyVariant),
    }
}

pub fn keys_to_str(key_a: &Key, key_b: &Key) -> String {
    let mut bytes_a = key_a.to_bytes().unwrap_or_revert();
    let mut bytes_b = key_b.to_bytes().unwrap_or_revert();

    bytes_a.append(&mut bytes_b);

    let bytes = runtime::blake2b(bytes_a);
    hex::encode(bytes)
}

pub fn key_and_value_to_str<T: CLTyped + ToBytes>(key: &Key, value: &T) -> String {
    let mut bytes_a = key.to_bytes().unwrap_or_revert();
    let mut bytes_b = value.to_bytes().unwrap_or_revert();

    bytes_a.append(&mut bytes_b);

    let bytes = runtime::blake2b(bytes_a);
    hex::encode(bytes)
}