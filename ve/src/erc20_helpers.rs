use casper_contract::{
    contract_api::{runtime}
};

use casper_types::{
    ContractHash,
    runtime_args,
    RuntimeArgs,
    Key,
    U256
};

pub fn get_total_supply(contract: Key) -> u128 {
    let total_supply : U256 = runtime::call_contract(
        ContractHash::new(contract.into_hash().unwrap()),
        "total_supply",
        runtime_args! {},
    );
    total_supply.as_u128()
}

pub fn get_balance(token: Key, user: Key) -> u128 {
    let b : U256 = runtime::call_contract(
        ContractHash::new(token.into_hash().unwrap()),
        "balance_of",
        runtime_args! {
            "address" => user
        },
    );
    b.as_u128()
}

pub fn transfer(contract_hash: Key, recipient: Key, amount: u128) {
    let _: () = runtime::call_contract(
        ContractHash::new(contract_hash.into_hash().unwrap()),
        "transfer",
        runtime_args! {
            "recipient" => recipient,
            "amount" => U256::from(amount)
        }
    );
}

pub fn transfer_from(contract_hash: Key, from: Key, recipient: Key, amount: u128) {
    let _: () = runtime::call_contract(
        ContractHash::new(contract_hash.into_hash().unwrap()),
        "transfer_from",
        runtime_args! {
            "owner" => from,
            "recipient" => recipient,
            "amount" => U256::from(amount)
        }
    );
}