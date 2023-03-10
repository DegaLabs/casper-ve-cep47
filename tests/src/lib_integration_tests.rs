use casper_engine_test_support::{
    ExecuteRequestBuilder, InMemoryWasmTestBuilder, DEFAULT_RUN_GENESIS_REQUEST,
    DEFAULT_ACCOUNT_ADDR, MINIMUM_ACCOUNT_CREATION_BALANCE,
};

use casper_types::{
    account::AccountHash, bytesrepr::FromBytes, CLTyped, runtime_args, system::mint,
    ContractHash, ContractPackageHash, Key, PublicKey, RuntimeArgs, crypto::SecretKey, U256, U128
};
use std::collections::BTreeMap;
use std::convert::TryInto;

const EXAMPLE_ERC20_TOKEN: &str = "erc20_token.wasm";
const TEST_SESSION: &str = "test-session.wasm";
const VE_CONTRACT: &str = "ve.wasm";
const ARG_NAME: &str = "name";
const ARG_SYMBOL: &str = "symbol";
const ARG_DECIMALS: &str = "decimals";
const ARG_TOTAL_SUPPLY: &str = "total_supply";
const ARG_NEW_MINTER: &str = "new_minter";
const RESULT_KEY: &str = "result";
const TOKEN_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000_000_000_000_000;
const ERC20_TOKEN_CONTRACT_KEY: &str = "erc20_token_contract";

fn get_token_key_name(symbol: String) -> String {
    ERC20_TOKEN_CONTRACT_KEY.to_owned() + "_" + &symbol
}

fn get_account1_addr() -> AccountHash {
    let sk: SecretKey = SecretKey::secp256k1_from_bytes(&[221u8; 32]).unwrap();
    let pk: PublicKey = PublicKey::from(&sk);
    let a: AccountHash = pk.to_account_hash();
    a
}

fn get_account2_addr() -> AccountHash {
    let sk: SecretKey = SecretKey::secp256k1_from_bytes(&[212u8; 32]).unwrap();
    let pk: PublicKey = PublicKey::from(&sk);
    let a: AccountHash = pk.to_account_hash();
    a
}

fn get_test_result<T: FromBytes + CLTyped>(
    builder: &mut InMemoryWasmTestBuilder,
    test_session: ContractPackageHash,
) -> T {
    let contract_package = builder
        .get_contract_package(test_session)
        .expect("should have contract package");
    let enabled_versions = contract_package.enabled_versions();
    let (_version, contract_hash) = enabled_versions
        .iter()
        .rev()
        .next()
        .expect("should have latest version");

    builder.get_value(*contract_hash, RESULT_KEY)
}

fn call_and_get<T: FromBytes + CLTyped>(
    builder: &mut InMemoryWasmTestBuilder,
    func_name: &str,
    args: RuntimeArgs
) -> T {
    let test_session = get_test_session(builder);
    let exec_request = ExecuteRequestBuilder::versioned_contract_call_by_hash(
        *DEFAULT_ACCOUNT_ADDR,
        test_session,
        None,
        func_name,
        args,
    )
    .build();
    builder.exec(exec_request).expect_success().commit();

    get_test_result(builder, test_session)
}

#[derive(Copy, Clone)]
struct TestContext {
    ve_contract_hash: ContractHash,
    token: ContractHash,
    ve_contract_package_hash: ContractPackageHash
}

fn exec_call(builder: &mut InMemoryWasmTestBuilder, account_hash: AccountHash, contract_hash: ContractHash, fun_name: &str, args: RuntimeArgs, expect_success: bool) {
    let request = ExecuteRequestBuilder::contract_call_by_hash(
        account_hash,
        contract_hash,
        fun_name,
        args
    ).build();
    if expect_success {
        builder.exec(request).expect_success().commit();
    } else {
        builder.exec(request).expect_failure();
    }
}

fn get_test_session(builder: &mut InMemoryWasmTestBuilder) -> ContractPackageHash {
    let install_test_session = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        TEST_SESSION,
        runtime_args! {}
    )
    .build();

    builder.exec(install_test_session).expect_success().commit();    

    let account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account");

    let test_session = account
        .named_keys()
        .get("test_session")
        .and_then(|key| key.into_hash())
        .map(ContractPackageHash::new)
        .expect("should have contract hash");
    test_session
}

fn setup() -> (InMemoryWasmTestBuilder, TestContext) {
    let mut builder = InMemoryWasmTestBuilder::default();
    builder.run_genesis(&*DEFAULT_RUN_GENESIS_REQUEST);

    let id: Option<u64> = None;
    let transfer_1_args = runtime_args! {
        mint::ARG_TARGET => get_account1_addr(),
        mint::ARG_AMOUNT => MINIMUM_ACCOUNT_CREATION_BALANCE,
        mint::ARG_ID => id,
    };
    let transfer_2_args = runtime_args! {
        mint::ARG_TARGET => get_account2_addr(),
        mint::ARG_AMOUNT => MINIMUM_ACCOUNT_CREATION_BALANCE,
        mint::ARG_ID => id,
    };

    let transfer_request_1 =
        ExecuteRequestBuilder::transfer(*DEFAULT_ACCOUNT_ADDR, transfer_1_args).build();
    let transfer_request_2 =
        ExecuteRequestBuilder::transfer(*DEFAULT_ACCOUNT_ADDR, transfer_2_args).build();

    let deploy_erc20 = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        EXAMPLE_ERC20_TOKEN,
        runtime_args! {
            ARG_NAME => "USDC Faucet".to_string(),
            ARG_SYMBOL => "USDC".to_string(),
            ARG_DECIMALS => 18u8,
            ARG_TOTAL_SUPPLY => U256::from(TOKEN_TOTAL_SUPPLY),
            ARG_NEW_MINTER => Key::from(*DEFAULT_ACCOUNT_ADDR)
        },
    )
    .build();

    builder.exec(transfer_request_1).expect_success().commit();
    builder.exec(transfer_request_2).expect_success().commit();
    builder.exec(deploy_erc20).expect_success().commit();

    let account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account");

    let usdc_token = account
        .named_keys()
        .get(&get_token_key_name("USDC".to_string()))
        .and_then(|key| key.into_hash())
        .map(ContractHash::new)
        .expect("should have contract hash");

    let deploy_ve = ExecuteRequestBuilder::standard(
        *DEFAULT_ACCOUNT_ADDR,
        VE_CONTRACT,
        runtime_args! {
            ARG_NAME => "USDC-VE".to_string(),
            ARG_SYMBOL => "USDC".to_string(),
            "meta" => BTreeMap::<String, String>::new(),
            "token_contract_hash" => Key::from(usdc_token),
            "art_proxy_contract_hash" => Key::from(usdc_token),
            "contract_name" => "ve".to_string()
        },
    )
    .build();
    builder.exec(deploy_ve).expect_success().commit();    

    let account = builder
        .get_account(*DEFAULT_ACCOUNT_ADDR)
        .expect("should have account");

    let ve_contract_hash = account
        .named_keys()
        .get(&"ve_contract_hash".to_string())
        .and_then(|key| key.into_hash())
        .map(ContractHash::new)
        .expect("should have contract hash");

    let ve_contract_package_hash = account
        .named_keys()
        .get(&"ve_contract_package_hash".to_string())
        .and_then(|key| key.into_hash())
        .map(ContractPackageHash::new)
        .expect("should have contract package hash");

    // update lp
    exec_call(&mut builder, *DEFAULT_ACCOUNT_ADDR, usdc_token, "approve", runtime_args! {
        "spender" => Key::from(ve_contract_package_hash),
        "amount" => U256::from(TOKEN_TOTAL_SUPPLY)
    }, true);
    println!(
        "approve cost {:?}",
        builder.last_exec_gas_cost()
    );

    let tc = TestContext {
        token: usdc_token,
        ve_contract_hash,
        ve_contract_package_hash
    };

    (builder, tc)
}

#[test]
fn test_create_lock() {
    let (mut builder, tc) = setup();
    let lock_duration: u64 = 7 * 24 * 3600;
    let balance: U256 = call_and_get(&mut builder, "get_balance", runtime_args! {
        "contract_hash" => tc.ve_contract_hash,
        "address" => Key::from(*DEFAULT_ACCOUNT_ADDR)
    });
    assert_eq!(0u128, balance.as_u128());
    exec_call(&mut builder, *DEFAULT_ACCOUNT_ADDR, tc.ve_contract_hash, "create_lock", runtime_args! {
        "amount" => U128::from(1_000_000_000_000_000_000_000u128),
        "lock_duration" => lock_duration
    }, true);

    let owner_of: Key = call_and_get(&mut builder, "owner_of", runtime_args! {
        "contract_hash" => tc.ve_contract_hash,
        "token_id" => U256::from(1)
    });
    assert_eq!(owner_of, Key::from(*DEFAULT_ACCOUNT_ADDR));

    let balance_of: U256 = call_and_get(&mut builder, "get_balance", runtime_args! {
        "contract_hash" => tc.ve_contract_hash,
        "address" => Key::from(*DEFAULT_ACCOUNT_ADDR)
    });
    assert_eq!(balance_of.as_u64(), 1);
    // exec_call(&mut builder, *DEFAULT_ACCOUNT_ADDR, tc.dex_contract, "add_liquidity", runtime_args! {
    //     "amounts" => vec![U128::from(1_000_000_000_000_000_000u128), U128::from(1_000_000_000_000_000_000u128)],
    //     "min_to_mint" => U128::from(0),
    //     "deadline" => 99999999999999u64
    // }, false);

    // exec_call(&mut builder, *DEFAULT_ACCOUNT_ADDR, tc.dex_contract, "set_paused", runtime_args! {
    //     "paused" => false
    // }, true);

    // exec_call(&mut builder, *DEFAULT_ACCOUNT_ADDR, tc.dex_contract, "add_liquidity", runtime_args! {
    //     "amounts" => vec![U128::from(1_000_000_000_000_000_000u128), U128::from(3_000_000_000_000_000_000u128)],
    //     "min_to_mint" => U128::from(0),
    //     "deadline" => 99999999999999u64
    // }, true);
}


