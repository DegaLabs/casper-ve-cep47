#![no_std]
#![no_main]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
    boxed::Box
};

use casper_contract::{
    self,
    contract_api::{runtime, storage},
};

use casper_types::{
    bytesrepr::ToBytes, runtime_args, CLTyped, ContractHash, EntryPoint, EntryPointAccess,
    EntryPointType, EntryPoints, Key, Parameter, RuntimeArgs, U256, U128, CLType
};

const RESULT_KEY: &str = "result";
const TEST_SESSION: &str = "test_session";

fn store_result<T: CLTyped + ToBytes>(result: T) {
    match runtime::get_key(RESULT_KEY) {
        Some(Key::URef(uref)) => storage::write(uref, result),
        Some(_) => unreachable!(),
        None => {
            let new_uref = storage::new_uref(result);
            runtime::put_key(RESULT_KEY, new_uref.into());
        }
    }
}

#[no_mangle]
extern "C" fn get_balance() {
    let token_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let owner: Key = runtime::get_named_arg("address");

    let b: U256 = runtime::call_contract(
        token_contract,
        "balance_of",
        runtime_args! {
            "address" => owner,
            "owner" => owner
        },
    );
    store_result(b);
}

#[no_mangle]
extern "C" fn get_allowance() {
    let token_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let owner: Key = runtime::get_named_arg("owner");
    let spender: Key = runtime::get_named_arg("spender");

    let b: U256 = runtime::call_contract(
        token_contract,
        "allowance",
        runtime_args! {
            "owner" => owner,
            "spender" => spender
        },
    );
    store_result(U128::from(b.as_u128()));
}

#[no_mangle]
extern "C" fn get_total_supply() {
    let token_contract: ContractHash = runtime::get_named_arg("contract_hash");

    let b: U256 = runtime::call_contract(
        token_contract,
        "total_supply",
        runtime_args! {
        },
    );
    store_result(U128::from(b.as_u128()));
}

#[no_mangle]
extern "C" fn get_token_balance() {
    let dex_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let index: u64 = runtime::get_named_arg("index");

    let b: U128 = runtime::call_contract(
        dex_contract,
        "get_token_balance",
        runtime_args! {
            "index" => index
        },
    );

    store_result(b);
}

#[no_mangle]
extern "C" fn get_virtual_price() {
    let dex_contract: ContractHash = runtime::get_named_arg("contract_hash");

    let b: U128 = runtime::call_contract(
        dex_contract,
        "get_virtual_price",
        runtime_args! {
        },
    );

    store_result(b);
}

#[no_mangle]
extern "C" fn owner_of() {
    let ve_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let token_id: U256 = runtime::get_named_arg("token_id");
    runtime::print("calling owner_of");
    let b: Option<Key> = runtime::call_contract(
        ve_contract,
        "owner_of",
        runtime_args! {
            "token_id" => token_id
        },
    );
    runtime::print("get_result");
    store_result(b.unwrap());
}

#[no_mangle]
extern "C" fn calculate_swap() {
    let dex_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let token_index_from: u64 = runtime::get_named_arg("token_index_from");
    let token_index_to: u64 = runtime::get_named_arg("token_index_to");
    let dx: U128 = runtime::get_named_arg("dx");

    let b: U128 = runtime::call_contract(
        dex_contract,
        "calculate_swap",
        runtime_args! {
            "token_index_from" => token_index_from,
            "token_index_to" => token_index_to,
            "dx" => dx
        },
    );

    store_result(b);
}


#[no_mangle]
extern "C" fn calculate_token_amount() {
    let dex_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let amounts: Vec<U128> = runtime::get_named_arg("amounts");
    let deposit: bool = runtime::get_named_arg("deposit");

    let b: U128 = runtime::call_contract(
        dex_contract,
        "calculate_token_amount",
        runtime_args! {
            "amounts" => amounts,
            "deposit" => deposit
        },
    );

    store_result(b);
}

#[no_mangle]
extern "C" fn calculate_remove_liquidity_one_token() {
    let dex_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let token_amount: U128 = runtime::get_named_arg("token_amount");
    let token_index: bool = runtime::get_named_arg("token_index");

    let b: U128 = runtime::call_contract(
        dex_contract,
        "calculate_remove_liquidity_one_token",
        runtime_args! {
            "token_amount" => token_amount,
            "token_index" => token_index
        },
    );

    store_result(b);
}

#[no_mangle]
extern "C" fn get_admin_balance() {
    let dex_contract: ContractHash = runtime::get_named_arg("contract_hash");
    let token_index: bool = runtime::get_named_arg("token_index");

    let b: U128 = runtime::call_contract(
        dex_contract,
        "get_admin_balance",
        runtime_args! {
            "token_index" => token_index
        },
    );

    store_result(b);
}

#[no_mangle]
pub extern "C" fn call() {
    let mut entry_points = EntryPoints::new();
    let get_balance_entrypoint = EntryPoint::new(
        String::from("get_balance"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("address", Key::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );
    let get_allowance_entrypoint = EntryPoint::new(
        String::from("get_allowance"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("spender", Key::cl_type()),
            Parameter::new("owner", Key::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );
    let get_supply_entrypoint = EntryPoint::new(
        String::from("get_total_supply"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    let get_token_balance_entrypoint = EntryPoint::new(
        String::from("get_token_balance"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("index", CLType::U64)
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    let get_virtual_price_entrypoint = EntryPoint::new(
        String::from("get_virtual_price"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    let get_owner_of = EntryPoint::new(
        String::from("owner_of"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("token_id", U256::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    let calculate_swap_entrypoint = EntryPoint::new(
        String::from("calculate_swap"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("token_index_from", CLType::U64),
            Parameter::new("token_index_to", CLType::U64),
            Parameter::new("dx", U128::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    let calculate_token_amount_entrypoint = EntryPoint::new(
        String::from("calculate_token_amount"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("amounts", CLType::List(Box::new(CLType::U128))),
            Parameter::new("deposit", CLType::Bool),
            Parameter::new("dx", U128::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    let calculate_remove_liquidity_one_token_entrypoint = EntryPoint::new(
        String::from("calculate_remove_liquidity_one_token"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("token_index", CLType::U64),
            Parameter::new("token_amount", U128::cl_type())
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    let get_admin_balance_entrypoint = EntryPoint::new(
        String::from("get_admin_balance"),
        vec![
            Parameter::new("contract_hash", ContractHash::cl_type()),
            Parameter::new("token_index", CLType::U64),
        ],
        <()>::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    );

    entry_points.add_entry_point(get_admin_balance_entrypoint);
    entry_points.add_entry_point(calculate_remove_liquidity_one_token_entrypoint);
    entry_points.add_entry_point(calculate_swap_entrypoint);
    entry_points.add_entry_point(get_virtual_price_entrypoint);
    entry_points.add_entry_point(get_token_balance_entrypoint);
    entry_points.add_entry_point(get_supply_entrypoint);
    entry_points.add_entry_point(get_allowance_entrypoint);
    entry_points.add_entry_point(get_balance_entrypoint);
    entry_points.add_entry_point(calculate_token_amount_entrypoint);
    entry_points.add_entry_point(get_owner_of);

    let (_contract_hash, _version) = storage::new_contract(
        entry_points,
        None,
        Some(TEST_SESSION.to_string()),
        None,
    );
}
