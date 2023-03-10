use crate::cep47::NFTToken;
use crate::data;
use crate::dict::Dict;
use crate::error::VeError;
use crate::lock::{self, *};
use crate::utils::{self, require};
use crate::utils::{get_key, set_key};
use crate::{erc20_helpers, CEP47, I128::*, TokenId};
use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use casper_contract::{
    contract_api::{
        runtime::{self, get_blocktime},
        runtime::print,
        storage,
    },
    unwrap_or_revert::UnwrapOrRevert,
};
use crate::cep47::Error;
use casper_types::{
    bytesrepr::{self, FromBytes, ToBytes},
    CLType, CLTyped, CLValue, EntryPoint, EntryPointAccess, EntryPointType, EntryPoints, Key,
    Parameter, U128, U256,
};
use serde::{Deserialize, Serialize};

pub const MAX_TOTAL_TOKEN_SUPPLY: u64 = 1_000_000u64;
pub const ARG_TOKEN_ID: &str = "token_id";

pub const POINT_HISTORY: &str = "point_history";
pub const IS_LOCKED: &str = "is_locked";
pub const TOKEN_CONTRACT_HASH: &str = "token_contract_hash";
pub const ART_PROXY_CONTRACT_HASH: &str = "art_proxy_contract_hash";
pub const VOTER: &str = "voter";
pub const VOTED: &str = "voted";
pub const TEAM: &str = "team";
pub const USER_POINT_EPOCH: &str = "user_point_epoch";
pub const USER_POINT_HISTORY: &str = "user_point_history";
pub const LOCKED: &str = "locked";
pub const EPOCH: &str = "epoch";
pub const SLOPE_CHANGES: &str = "slope_changes";
pub const SUPPLY: &str = "supply";
pub const EPOCH_INDEX: &str = "epoch_index";
pub const VE_SUPPLY: &str = "ve_supply";
pub const ARG_AMOUNT: &str = "amount";
pub const ARG_LOCK_DURATION: &str = "lock_duration";
pub const DELEGATES: &str = "delegates";
pub const CHECKPOINTS: &str = "checkpoints";
pub const NUM_CHECKPOINTS: &str = "num_checkpoints";
pub const NONCES: &str = "nonces";
pub const ARG_ADDRESS: &str = "address";
pub const EPOCH_TIME: &str = "epoch_time";
pub const BLOCK: &str = "block";
pub const ARG_T: &str = "t";
pub const ATTACHMENTS: &str = "attachments";
pub const ARG_FROM: &str = "from";
pub const ARG_TO: &str = "to";
pub const DELEGATOR: &str = "delegator";
pub const ARG_TIMESTAMP: &str = "timestamp";

pub const DEPOSIT_FOR_TYPE: u8 = 0;
pub const CREATE_LOCK_TYPE: u8 = 1;
pub const INCREASE_LOCK_AMOUNT: u8 = 2;
pub const INCREASE_UNLOCK_TIME: u8 = 3;
pub const MERGE_TYPE: u8 = 4;
pub const WEEK: u128 = 86400 * 7;
pub const MAXTIME: u128 = 26 * 86400 * 7;
pub const I_MAXTIME: i128 = 26 * 86400 * 7;
pub const MULTIPLIER: u128 = 1_000_000_000_000_000_000;
pub const MAX_DELEGATES: u64 = 1024;

pub fn current_block_timestamp_seconds() -> u64 {
    u64::from(get_blocktime()).checked_rem(u64::MAX).unwrap() / 1000
}

pub fn current_block_number() -> u64 {
    100
}

#[derive(Serialize, Deserialize, Clone)]
pub struct LockedBalance {
    pub amount: u128,
    pub end: u64,
}

impl Default for LockedBalance {
    fn default() -> Self {
        LockedBalance { amount: 0, end: 0 }
    }
}

impl ToBytes for LockedBalance {
    fn to_bytes(&self) -> Result<Vec<u8>, casper_types::bytesrepr::Error> {
        let mut result = bytesrepr::allocate_buffer(self)?;
        result.extend(U128::from(self.amount).to_bytes()?);
        result.extend(self.end.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        U128::from(self.amount).serialized_length() + self.end.serialized_length()
    }
}

impl FromBytes for LockedBalance {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), casper_types::bytesrepr::Error> {
        let (amount, remainder) = U128::from_bytes(bytes)?;
        let amount = amount.as_u128();
        let (end, remainder) = u64::from_bytes(remainder)?;
        Ok((LockedBalance { amount, end }, remainder))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Point {
    pub bias: i128,
    pub slope: i128,
    pub ts: u64,
    pub blk: u64,
}

impl Default for Point {
    fn default() -> Self {
        Point {
            bias: 0,
            slope: 0,
            ts: 0,
            blk: 0,
        }
    }
}

impl ToBytes for Point {
    fn to_bytes(&self) -> Result<Vec<u8>, casper_types::bytesrepr::Error> {
        let mut result = bytesrepr::allocate_buffer(self)?;
        result.extend(I128::from(self.bias).to_bytes()?);
        result.extend(I128::from(self.slope).to_bytes()?);
        result.extend(self.ts.to_bytes()?);
        result.extend(self.blk.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        I128::from(self.bias).serialized_length() * 2 + self.ts.serialized_length() * 2
    }
}

impl FromBytes for Point {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), casper_types::bytesrepr::Error> {
        let (bias, remainder) = I128::from_bytes(bytes)?;
        let bias = bias.as_i128();
        let (slope, remainder) = I128::from_bytes(remainder)?;
        let slope = slope.as_i128();
        let (ts, remainder) = u64::from_bytes(remainder)?;
        let (blk, remainder) = u64::from_bytes(remainder)?;
        Ok((
            Point {
                bias,
                slope,
                ts,
                blk,
            },
            remainder,
        ))
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct Checkpoint {
    pub timestamp: u128,
    pub token_ids: Vec<u64>,
}

impl Default for Checkpoint {
    fn default() -> Self {
        Checkpoint {
            timestamp: 0,
            token_ids: vec![],
        }
    }
}

impl ToBytes for Checkpoint {
    fn to_bytes(&self) -> Result<Vec<u8>, casper_types::bytesrepr::Error> {
        let mut result = bytesrepr::allocate_buffer(self)?;
        result.extend(U128::from(self.timestamp).to_bytes()?);
        result.extend(self.token_ids.to_bytes()?);
        Ok(result)
    }

    fn serialized_length(&self) -> usize {
        U128::from(self.timestamp).serialized_length() + self.token_ids.serialized_length()
    }
}

impl FromBytes for Checkpoint {
    fn from_bytes(bytes: &[u8]) -> Result<(Self, &[u8]), casper_types::bytesrepr::Error> {
        let (timestamp, remainder) = U128::from_bytes(bytes)?;
        let timestamp = timestamp.as_u128();
        let (token_ids, remainder) = Vec::<u64>::from_bytes(remainder)?;
        Ok((
            Checkpoint {
                timestamp,
                token_ids,
            },
            remainder,
        ))
    }
}

impl CLTyped for Checkpoint {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl CLTyped for Point {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

impl CLTyped for LockedBalance {
    fn cl_type() -> CLType {
        CLType::Any
    }
}

pub fn initialize(token_contract: Key, art_proxy_contract: Key) {
    runtime::print("initialize");
    lock::init();
    let caller = utils::get_immediate_caller_key();
    set_key(TOKEN_CONTRACT_HASH, token_contract);
    set_key(ART_PROXY_CONTRACT_HASH, art_proxy_contract);
    set_key(TEAM, caller);
    set_key(VOTER, caller);

    storage::new_dictionary(POINT_HISTORY).unwrap_or_revert_with(VeError::FailedToCreateDictionary);

    let point = Point {
        bias: 0,
        slope: 0,
        ts: current_block_timestamp_seconds(),
        blk: current_block_number(),
    };
    let dict = Dict::instance(POINT_HISTORY);
    dict.set(&0u128.to_string(), point);

    escrow_init();
    voting_logic_init();
    dao_voting_storage_init();
}

#[no_mangle]
pub extern "C" fn set_team() {
    let current_team: Key = get_key(TEAM).unwrap();
    let caller = utils::get_immediate_caller_key();
    require(caller == current_team, VeError::NOTTEAM);
    let new_team: Key = runtime::get_named_arg("new_team");
    set_key(TEAM, new_team);
}

#[no_mangle]
pub extern "C" fn set_art_proxy() {
    let current_team: Key = get_key(TEAM).unwrap();
    let caller = utils::get_immediate_caller_key();
    require(caller == current_team, VeError::NOTTEAM);
    let new_ap: Key = runtime::get_named_arg("new_art_proxy");
    set_key(ART_PROXY_CONTRACT_HASH, new_ap);
}

////////////////////////////////////////////////////////////////
//                             ESCROW
//////////////////////////////////////////////////////////////*/
fn escrow_init() {
    storage::new_dictionary(USER_POINT_EPOCH)
        .unwrap_or_revert_with(VeError::FailedToCreateDictionary);

    storage::new_dictionary(USER_POINT_HISTORY)
        .unwrap_or_revert_with(VeError::FailedToCreateDictionary);

    storage::new_dictionary(LOCKED).unwrap_or_revert_with(VeError::FailedToCreateDictionary);

    storage::new_dictionary(SLOPE_CHANGES).unwrap_or_revert_with(VeError::FailedToCreateDictionary);

    set_key(EPOCH, 0u64);
    set_key(VE_SUPPLY, U128::from(0));
}

pub fn get_locked_balance(token_id: u64) -> LockedBalance {
    let dict = Dict::instance(LOCKED);
    let locked = dict.get::<LockedBalance>(&token_id.to_string());
    if locked.is_some() {
        let locked_balance: LockedBalance = locked.unwrap();
        return locked_balance;
    }
    LockedBalance::default()
}

pub fn get_slope_changes(time: u64) -> i128 {
    let dict = Dict::instance(SLOPE_CHANGES);
    let sc: I128 = dict.get(&time.to_string()).unwrap_or(I128::from(0));
    sc.bits
}

pub fn save_slope_changes(time: u64, val: i128) {
    let dict = Dict::instance(SLOPE_CHANGES);
    dict.set(&time.to_string(), I128 { bits: val })
}

pub fn get_user_point(token_id: u64, uepoch: u64) -> Point {
    let dict = Dict::instance(USER_POINT_HISTORY);
    dict.get::<Point>(&(token_id.to_string() + &uepoch.to_string()))
        .unwrap_or(Point::default())
}

pub fn get_point(uepoch: u128) -> Point {
    let dict = Dict::instance(POINT_HISTORY);
    dict.get::<Point>(&uepoch.to_string())
        .unwrap_or(Point::default())
}

#[no_mangle]
pub extern "C" fn get_last_user_slope() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let dict = Dict::instance(USER_POINT_EPOCH);
    let uepoch: u64 = dict.get(&token_id.to_string()).unwrap_or(0);
    let point = get_user_point(token_id, uepoch);
    let slope = I128 { bits: point.slope };
    runtime::ret(CLValue::from_t(slope).unwrap_or_revert());
}

/// @notice Get the timestamp for checkpoint `_idx` for `_tokenId`
/// @param _tokenId token of the NFT
/// @param _idx User epoch number
/// @return Epoch time of the checkpoint
#[no_mangle]
pub extern "C" fn user_point_history__ts() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let epoch_index: u64 = runtime::get_named_arg(EPOCH_INDEX);
    let point = get_user_point(token_id, epoch_index);

    runtime::ret(CLValue::from_t(U128::from(point.ts)).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn locked_end() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let locked_balance = get_locked_balance(token_id);

    runtime::ret(CLValue::from_t(locked_balance.end).unwrap_or_revert());
}

/// @notice Record global and per-user data to checkpoint
/// @param _tokenId NFT token ID. No user checkpoint if 0
/// @param old_locked Pevious locked amount / end lock time for the user
/// @param new_locked New locked amount / end lock time for the user
pub fn _check_point(token_id: u64, old_locked: &LockedBalance, new_locked: &LockedBalance) {
    let mut u_old = Point::default();
    let mut u_new = Point::default();
    let mut old_dslope = 0i128;
    let mut new_dslope = 0i128;
    let _epoch: u64 = get_key(EPOCH).unwrap();
    let mut _epoch = _epoch as u128;
    let ts = current_block_timestamp_seconds();
    let block_number = current_block_number();
    if token_id != 0 {
        if old_locked.end > ts && old_locked.amount > 0 {
            u_old.slope = (old_locked.amount / I_MAXTIME.unsigned_abs()) as i128;
            u_old.bias = u_old.slope * ((old_locked.end - ts) as i128);
        }

        if new_locked.end > ts && new_locked.amount > 0 {
            u_new.slope = (new_locked.amount / I_MAXTIME.unsigned_abs()) as i128;
            u_new.bias = u_new.slope * ((new_locked.end - ts) as i128);
        }

        // Read values of scheduled changes in the slope
        // old_locked.end can be in the past and in the future
        // new_locked.end can ONLY by in the FUTURE unless everything expired: than zeros
        old_dslope = get_slope_changes(old_locked.end);
        if new_locked.end != 0 {
            if new_locked.end == old_locked.end {
                new_dslope = old_dslope;
            } else {
                new_dslope = get_slope_changes(new_locked.end);
            }
        }
    }

    let mut last_point = Point {
        bias: 0,
        slope: 0,
        ts: ts,
        blk: block_number,
    };

    if _epoch > 0 {
        last_point = get_point(_epoch);
    }
    let dict = Dict::instance(POINT_HISTORY);

    let mut last_checkpoint = last_point.ts;
    let initial_last_point = last_point.clone();
    let mut block_slope = 0u128; // dblock/dt
    if ts > last_point.ts {
        block_slope = (MULTIPLIER * (block_number as u128 - last_point.blk as u128))
            / (ts as u128 - last_point.ts as u128);
    }

    {
        let mut t_i = (last_checkpoint as u128 / WEEK) * WEEK;
        for _i in 0..255u128 {
            // Hopefully it won't happen that this won't get used in 27 weeks!
            // If it does, users will be able to withdraw but vote weight will be broken
            t_i = t_i + WEEK as u128;
            let mut d_slope = 0i128;
            if t_i > ts as u128 {
                t_i = ts as u128;
            } else {
                d_slope = get_slope_changes(t_i as u64);
            }
            last_point.bias =
                last_point.bias - last_point.slope * ((t_i - last_checkpoint as u128) as i128);
            last_point.slope = last_point.slope + d_slope;
            if last_point.bias < 0 {
                // This can happen
                last_point.bias = 0;
            }
            if last_point.slope < 0 {
                // This cannot happen - just in case
                last_point.slope = 0;
            }
            last_checkpoint = t_i as u64;
            last_point.ts = t_i as u64;
            last_point.blk = initial_last_point.blk
                + ((block_slope as u128 * (t_i - initial_last_point.ts as u128)) / MULTIPLIER)
                    as u64;
            _epoch = _epoch + 1;
            if t_i == ts as u128 {
                last_point.blk = block_number;
                break;
            } else {
                // set point history
                dict.set(&_epoch.to_string(), last_point.clone());
                // point_history[_epoch] = last_point;
            }
        }
    }

    // update epoch
    set_key(EPOCH, _epoch as u64);

    if token_id != 0 {
        last_point.slope = last_point.slope + u_new.slope - u_old.slope;
        last_point.bias = last_point.bias + u_new.bias - u_old.bias;
        if last_point.slope < 0 {
            last_point.slope = 0;
        }
        if last_point.bias < 0 {
            last_point.bias = 0;
        }
    }
    // Record the changed point into history
    dict.set(&_epoch.to_string(), last_point);

    if token_id != 0 {
        if old_locked.end > ts {
            old_dslope = old_dslope + u_old.slope;
            if new_locked.end == old_locked.end {
                old_dslope = old_dslope - u_new.slope; // It was a new deposit, not extension
            }
            save_slope_changes(old_locked.end, old_dslope);
        }

        if new_locked.end > ts {
            if new_locked.end > old_locked.end {
                new_dslope = new_dslope - u_new.slope; // old slope disappeared at this point
                save_slope_changes(new_locked.end, new_dslope);
            }
            // else: we recorded it already in old_dslope
        }

        // Now handle user history
        let dict = Dict::instance(USER_POINT_EPOCH);
        let user_epoch: u64 = dict.get(&token_id.to_string()).unwrap_or(0);
        let user_epoch = user_epoch + 1;
        dict.set(&token_id.to_string(), user_epoch);

        u_new.ts = ts;
        u_new.blk = block_number;
        let dict_uph = Dict::instance(USER_POINT_HISTORY);
        dict_uph.set(&(token_id.to_string() + &user_epoch.to_string()), u_new);
    }
}

/// @notice Deposit and lock tokens for a user
/// @param _tokenId NFT that holds lock
/// @param _value Amount to deposit
/// @param unlock_time New time when to unlock the tokens, or 0 if unchanged
/// @param locked_balance Previous locked amount / timestamp
/// @param deposit_type The type of deposit
fn _deposit_for(
    token_id: u64,
    value: u128,
    unlock_time: u64,
    locked_balance: &LockedBalance,
    deposit_type: u8,
) {
    let mut __locked = locked_balance.clone();
    let supply_before: U128 = get_key(VE_SUPPLY).unwrap();
    let supply_before = supply_before.as_u128();

    set_key(VE_SUPPLY, U128::from(supply_before + value));
    let mut old_locked = LockedBalance::default();
    old_locked.amount = __locked.amount;
    old_locked.end = __locked.end;
    // Adding to existing lock, or if a lock is expired - creating a new one
    __locked.amount = __locked.amount + value;
    if unlock_time != 0 {
        __locked.end = unlock_time;
    }
    let dict_locked = Dict::instance(LOCKED);
    dict_locked.set(&token_id.to_string(), __locked.clone());

    // Possibilities:
    // Both old_locked.end could be current or expired (>/< block.timestamp)
    // value == 0 (extend lock) or value > 0 (add to lock or extend lock)
    // _locked.end > block.timestamp (always)
    _check_point(token_id, &old_locked, &__locked);

    let from = utils::get_immediate_caller_key();
    let token: Key = get_key(TOKEN_CONTRACT_HASH).unwrap();
    if value != 0 && deposit_type != MERGE_TYPE {
        erc20_helpers::transfer_from(token, from, utils::get_self_key(), value);
    }
    // TODO
    // emit Deposit(from, _tokenId, _value, __locked.end, deposit_type, block.timestamp);
    // emit Supply(supply_before, supply_before + _value);
}

#[no_mangle]
pub extern "C" fn check_point() {
    _check_point(0, &LockedBalance::default(), &LockedBalance::default());
}

#[no_mangle]
pub extern "C" fn deposit_for() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let amount: U128 = runtime::get_named_arg(ARG_AMOUNT);
    let amount = amount.as_u128();
    let locked_balance = get_locked_balance(token_id);

    require(amount > 0, VeError::InvalidAmount);
    require(locked_balance.amount > 0, VeError::NoExistingLock);
    require(
        locked_balance.end > current_block_timestamp_seconds(),
        VeError::CannotAddToExpiredLock,
    );

    _deposit_for(token_id, amount, 0, &locked_balance, DEPOSIT_FOR_TYPE);
}

pub fn _create_lock(value: u128, lock_duration: u64, to: Key) -> u64 {
    let ts = current_block_timestamp_seconds();
    let unlock_time = (ts + lock_duration) / (WEEK as u64) * (WEEK as u64); // Locktime is rounded down to weeks
    require(value > 0, VeError::InvalidAmount);
    require(unlock_time > ts, VeError::CanOnlyLockTillTimeInFuture);
    require(
        unlock_time <= ts + MAXTIME as u64,
        VeError::VotingLockMax26Weeks,
    );

    let minted_tokens_count = data::total_supply().as_u64();
    let token_id = minted_tokens_count + 1;
    runtime::print("minting token");
    runtime::print(&token_id.to_string());

    NFTToken::default()
        .mint(
            to,
            vec![U256::from(token_id)],
            vec![BTreeMap::<String, String>::new()],
        ).unwrap_or_revert();

    _move_token_delegates(utils::null_key(), _delegates(to), token_id);

    _deposit_for(
        token_id,
        value,
        unlock_time,
        &get_locked_balance(token_id),
        CREATE_LOCK_TYPE,
    );
    token_id
}

#[no_mangle]
pub extern "C" fn create_lock() {
    runtime::print("here");
    let amount: U128 = runtime::get_named_arg(ARG_AMOUNT);
    let lock_duration: u64 = runtime::get_named_arg(ARG_LOCK_DURATION);
    when_not_locked();
    lock_contract();
    _create_lock(
        amount.as_u128(),
        lock_duration,
        utils::get_immediate_caller_key(),
    );
    unlock_contract();
}

#[no_mangle]
pub extern "C" fn create_lock_for() {
    let amount: U128 = runtime::get_named_arg(ARG_AMOUNT);
    let lock_duration: u64 = runtime::get_named_arg(ARG_LOCK_DURATION);
    let to: Key = runtime::get_named_arg(ARG_TO);

    when_not_locked();
    lock_contract();
    _create_lock(amount.as_u128(), lock_duration, to);
    unlock_contract();
}

#[no_mangle]
pub extern "C" fn increase_amount() {
    let amount: U128 = runtime::get_named_arg(ARG_AMOUNT);
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let caller: Key = utils::get_immediate_caller_key();
    require(
        NFTToken::default().is_approved_or_owner(token_id.into(), caller),
        VeError::NotOwnerOrApproved,
    );

    when_not_locked();
    lock_contract();

    let ts = current_block_timestamp_seconds();
    let __locked = get_locked_balance(token_id);
    require(amount.as_u128() > 0, VeError::InvalidAmount);
    require(__locked.amount > 0, VeError::NoExistingLock);
    require(__locked.end > ts, VeError::CannotAddToExpiredLock);

    _deposit_for(
        token_id,
        amount.as_u128(),
        0,
        &__locked,
        INCREASE_LOCK_AMOUNT,
    );

    unlock_contract();
}

#[no_mangle]
pub extern "C" fn increase_unlock_time() {
    let lock_duration: u64 = runtime::get_named_arg(ARG_LOCK_DURATION);
    let token_id: u64 = runtime::get_named_arg(ARG_TOKEN_ID);
    let caller: Key = utils::get_immediate_caller_key();
    require(
        NFTToken::default().is_approved_or_owner(token_id.into(), caller),
        VeError::NotOwnerOrApproved,
    );

    when_not_locked();
    lock_contract();

    let ts = current_block_timestamp_seconds();
    let __locked = get_locked_balance(token_id);
    let unlock_time = (ts + lock_duration) / (WEEK as u64) * (WEEK as u64); // Locktime is rounded down to weeks

    require(__locked.end > ts, VeError::CannotAddToExpiredLock);
    require(__locked.amount > 0, VeError::NoExistingLock);
    require(unlock_time > __locked.end, VeError::CanOnlyIncreaseLock);
    require(
        unlock_time <= ts + MAXTIME as u64,
        VeError::VotingLockMax26Weeks,
    );

    _deposit_for(token_id, 0, unlock_time, &__locked, INCREASE_UNLOCK_TIME);

    unlock_contract();
}

fn _burn_nft(token_id: u64) {
    let caller: Key = utils::get_immediate_caller_key();
    require(
        NFTToken::default().is_approved_or_owner(token_id.into(), caller),
        VeError::NotOwnerOrApproved,
    );
    let owner = NFTToken::default().owner_of(token_id.into()).unwrap();
    NFTToken::default()
        .burn(owner, vec![U256::from(token_id)])
        .unwrap_or_revert();
    _move_token_delegates(owner, utils::null_key(), token_id);
    _move_token_delegates(_delegates(owner), utils::null_key(), token_id);
}

#[no_mangle]
pub extern "C" fn withdraw() {}

////////////////////////////////////////////////////////////////
//                             GAUGE VOTING STORAGE
//////////////////////////////////////////////////////////////*/
/// @notice Binary search to estimate timestamp for block number
/// @param _block Block to find
/// @param max_epoch Don't go beyond this epoch
/// @return Approximate timestamp for block
fn _find_block_epoch(_block: u64, max_epoch: u64) -> u64 {
    // Binary search
    let mut _min = 0u64;
    let mut _max = max_epoch;
    for _i in 0..128 {
        // Will be always enough for 128-bit numbers
        if _min >= _max {
            break;
        }
        let _mid = (_min + _max + 1) / 2;
        if get_point(_mid.into()).blk <= _block {
            _min = _mid;
        } else {
            _max = _mid - 1;
        }
    }
    _min
}

/// @notice Get the current voting power for `_tokenId`
/// @dev Adheres to the ERC20 `balanceOf` interface for Aragon compatibility
/// @param _tokenId NFT for lock
/// @param _t Epoch time to return voting power at
/// @return User voting power
fn _balance_of_nft(token_id: u64, t: u64) -> u128 {
    let dict = Dict::instance(USER_POINT_EPOCH);
    let _epoch: u64 = dict.get(&token_id.to_string()).unwrap_or(0);

    if _epoch == 0 {
        return 0;
    } else {
        let mut last_point = get_user_point(token_id, _epoch);
        last_point.bias =
            last_point.bias - last_point.slope * ((t as i128) - (last_point.ts as i128));
        if last_point.bias < 0 {
            last_point.bias = 0;
        }
        return last_point.bias as u128;
    }
}

#[no_mangle]
pub extern "C" fn balance_of_nft() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    runtime::ret(
        CLValue::from_t(U128::from(_balance_of_nft(
            token_id,
            current_block_timestamp_seconds(),
        )))
        .unwrap_or_revert(),
    );
}

#[no_mangle]
pub extern "C" fn balance_of_nft_at() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let epoch_time: u64 = runtime::get_named_arg(EPOCH_TIME);

    runtime::ret(
        CLValue::from_t(U128::from(_balance_of_nft(token_id, epoch_time))).unwrap_or_revert(),
    );
}

/// @notice Measure voting power of `_tokenId` at block height `_block`
/// @dev Adheres to MiniMe `balanceOfAt` interface: https://github.com/Giveth/minime
/// @param _tokenId User's wallet NFT
/// @param _block Block to calculate the voting power at
/// @return Voting power
fn _balance_of_at_nft(token_id: u64, block: u64) -> u128 {
    let block_number = current_block_number();
    let ts = current_block_timestamp_seconds();
    require(block <= block_number, VeError::InvalidBlock);

    // Binary search
    let mut _min = 0u64;
    let dict = Dict::instance(USER_POINT_EPOCH);
    let mut _max = dict.get(&token_id.to_string()).unwrap_or(0);
    for _i in 0..128 {
        // Will be always enough for 128-bit numbers
        if _min >= _max {
            break;
        }
        let _mid = (_min + _max + 1) / 2;
        if get_user_point(token_id, _mid).blk <= block {
            _min = _mid;
        } else {
            _max = _mid - 1;
        }
    }

    let mut upoint = get_user_point(token_id, _min);

    let max_epoch: u64 = get_key(EPOCH).unwrap();
    let _epoch = _find_block_epoch(block, max_epoch);
    let point_0 = get_point(_epoch as u128);
    let d_block;
    let d_t;
    if _epoch < max_epoch {
        let point_1 = get_point(_epoch as u128 + 1);
        d_block = point_1.blk - point_0.blk;
        d_t = point_1.ts - point_0.ts;
    } else {
        d_block = block_number - point_0.blk;
        d_t = ts - point_0.ts;
    }
    let mut block_time = point_0.ts;
    if d_block != 0 {
        block_time = block_time + (d_t * (block - point_0.blk)) / d_block;
    }

    upoint.bias = upoint.bias - upoint.slope * (block_time as i128 - upoint.ts as i128);
    if upoint.bias >= 0 {
        return upoint.bias as u128;
    } else {
        return 0;
    }
}

#[no_mangle]
pub extern "C" fn balance_of_at_nft() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let block: u64 = runtime::get_named_arg(BLOCK);

    runtime::ret(
        CLValue::from_t(U128::from(_balance_of_at_nft(token_id, block))).unwrap_or_revert(),
    );
}

/// @notice Calculate total voting power at some point in the past
/// @param _block Block to calculate the total voting power at
/// @return Total voting power at `_block`
#[no_mangle]
pub extern "C" fn total_supply_at() {
    let block: u64 = runtime::get_named_arg(BLOCK);
    let block_number = current_block_number();
    let ts = current_block_timestamp_seconds();

    require(block <= block_number, VeError::InvalidBlock);
    let _epoch: u64 = get_key(EPOCH).unwrap_or(0);
    let target_epoch = _find_block_epoch(block, _epoch);

    let point = get_point(target_epoch as u128);
    let mut dt = 0u64;
    if target_epoch < _epoch {
        let point_next = get_point(target_epoch as u128 + 1);
        if point.blk != point_next.blk {
            dt = ((block - point.blk) * (point_next.ts - point.ts)) / (point_next.blk - point.blk);
        }
    } else {
        if point.blk != block_number {
            dt = ((block - point.blk) * (ts - point.ts)) / (block_number - point.blk);
        }
    }
    // Now dt contains info on how far are we beyond point
    runtime::ret(
        CLValue::from_t(U128::from(_supply_at(point.clone(), point.ts + dt))).unwrap_or_revert(),
    );
}

/// @notice Calculate total voting power at some point in the past
/// @param point The point (bias/slope) to start search from
/// @param t Time to calculate the total voting power at
/// @return Total voting power at that time
fn _supply_at(point: Point, t: u64) -> u128 {
    let mut last_point = point;
    let mut t_i = (last_point.ts / WEEK as u64) * WEEK as u64;
    for _i in 0..255 {
        t_i = t_i + WEEK as u64;
        let mut d_slope = 0i128;
        if t_i > t {
            t_i = t;
        } else {
            d_slope = get_slope_changes(t_i);
        }
        last_point.bias =
            last_point.bias - last_point.slope * (t_i as i128 - last_point.ts as i128);
        if t_i == t {
            break;
        }
        last_point.slope = last_point.slope + d_slope;
        last_point.ts = last_point.ts + t_i;
    }

    if last_point.bias < 0 {
        last_point.bias = 0;
    }
    return last_point.bias as u128;
}

fn _total_supply_at_t(t: u64) -> u128 {
    let epoch: u64 = get_key(EPOCH).unwrap();
    let last_point = get_point(epoch as u128);
    _supply_at(last_point, t)
}

#[no_mangle]
pub extern "C" fn ve_total_supply() {
    runtime::ret(
        CLValue::from_t(U128::from(_total_supply_at_t(
            current_block_timestamp_seconds(),
        )))
        .unwrap_or_revert(),
    );
}

#[no_mangle]
pub extern "C" fn total_supply_at_t() {
    let t: u64 = runtime::get_named_arg(ARG_T);
    runtime::ret(CLValue::from_t(U128::from(_total_supply_at_t(t))).unwrap_or_revert());
}

////////////////////////////////////////////////////////////////
//                             GAUGE VOTING LOGIC
//////////////////////////////////////////////////////////////*/
fn voting_logic_init() {
    storage::new_dictionary(ATTACHMENTS).unwrap_or_revert_with(VeError::FailedToCreateDictionary);
    storage::new_dictionary(VOTED).unwrap_or_revert_with(VeError::FailedToCreateDictionary);
}

fn get_attachments(token_id: u64) -> u64 {
    let dict = Dict::instance(ATTACHMENTS);
    let r: u64 = dict.get(&token_id.to_string()).unwrap_or(0);
    r
}

fn get_voted(token_id: u64) -> bool {
    let dict = Dict::instance(VOTED);
    let r: bool = dict.get(&token_id.to_string()).unwrap_or(false);
    r
}

fn only_voter() {
    let caller = utils::get_immediate_caller_key();
    let voter: Key = get_key(VOTER).unwrap();
    require(caller == voter, VeError::NotVoter);
}

#[no_mangle]
pub extern "C" fn set_voter() {
    let voter: Key = runtime::get_named_arg(VOTER);
    only_voter();
    set_key(VOTER, voter);
}

#[no_mangle]
pub extern "C" fn voting() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    only_voter();
    let dict = Dict::instance(VOTED);
    dict.set(&token_id.to_string(), true);
}

#[no_mangle]
pub extern "C" fn abstain() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    only_voter();
    let dict = Dict::instance(VOTED);
    dict.set(&token_id.to_string(), false);
}

#[no_mangle]
pub extern "C" fn attach() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    only_voter();
    let dict = Dict::instance(ATTACHMENTS);
    dict.set(&token_id.to_string(), token_id + 1);
}

#[no_mangle]
pub extern "C" fn detach() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    only_voter();
    let dict = Dict::instance(ATTACHMENTS);
    dict.set(&token_id.to_string(), token_id - 1);
}

#[no_mangle]
pub extern "C" fn merge() {
    let from: u64 = runtime::get_named_arg::<U256>(ARG_FROM).as_u64();
    let to: u64 = runtime::get_named_arg::<U256>(ARG_TO).as_u64();
    require(from != to, VeError::FromMustNotTo);

    let caller = utils::get_immediate_caller_key();
    require(
        NFTToken::default().is_approved_or_owner(from.into(), caller),
        VeError::NotOwnerOrApproved,
    );
    require(
        NFTToken::default().is_approved_or_owner(to.into(), caller),
        VeError::NotOwnerOrApproved,
    );

    let locked0 = get_locked_balance(from);
    let locked1 = get_locked_balance(to);
    let value0 = locked0.amount as u128;
    let end = if locked0.end >= locked1.end {
        locked0.end
    } else {
        locked1.end
    };

    let dict = Dict::instance(LOCKED);
    dict.set(&from.to_string(), LockedBalance::default());
    _check_point(from, &locked0, &LockedBalance::default());
    _burn_nft(from);
    _deposit_for(to, value0, end, &locked1, MERGE_TYPE);
}

////////////////////////////////////////////////////////////////
//                             DAO VOTING STORAGE
//////////////////////////////////////////////////////////////*/
fn dao_voting_storage_init() {
    storage::new_dictionary(DELEGATES).unwrap_or_revert_with(VeError::FailedToCreateDictionary);
    storage::new_dictionary(CHECKPOINTS).unwrap_or_revert_with(VeError::FailedToCreateDictionary);
    storage::new_dictionary(NUM_CHECKPOINTS)
        .unwrap_or_revert_with(VeError::FailedToCreateDictionary);
    storage::new_dictionary(NONCES).unwrap_or_revert_with(VeError::FailedToCreateDictionary);
}

fn get_delegate(a: Key) -> Key {
    runtime::print("get_delegate reading dict");
    let k = utils::key_to_str(&a);
    let dict = Dict::instance(DELEGATES);
    runtime::print("get_delegate");
    dict.get(&k).unwrap_or(utils::null_key())
}

fn set_delegate(a: Key, d: Key) {
    let k = utils::key_to_str(&a);
    let dict = Dict::instance(DELEGATES);
    dict.set(&k, d);
}

fn get_check_point_key(a: Key, index: u64) -> String {
    let k = a.to_bytes().unwrap();

    let mut preimage = Vec::new();
    preimage.append(&mut k.to_vec());
    preimage.append(&mut index.to_le_bytes().to_vec());

    let key_bytes = runtime::blake2b(&preimage);
    let k = hex::encode(&key_bytes);
    k
}

fn get_check_point(a: Key, index: u64) -> Checkpoint {
    let k = get_check_point_key(a, index);
    let dict = Dict::instance(CHECKPOINTS);
    dict.get(&k).unwrap_or_default()
}

fn set_check_point(a: Key, index: u64, cp: &Checkpoint) {
    let k = get_check_point_key(a, index);
    let dict = Dict::instance(CHECKPOINTS);
    dict.set(&k, (*cp).clone());
}

fn get_num_checkpoints(a: Key) -> u64 {
    let k = utils::key_to_str(&a);
    let dict = Dict::instance(NUM_CHECKPOINTS);
    dict.get(&k).unwrap_or_default()
}

fn set_num_checkpoints(a: Key, n: u64) {
    let k = utils::key_to_str(&a);
    let dict = Dict::instance(NUM_CHECKPOINTS);
    dict.set(&k, n);
}

fn get_nonces(a: Key) -> u64 {
    let k = utils::key_to_str(&a);
    let dict = Dict::instance(NONCES);
    dict.get(&k).unwrap_or_default()
}

fn set_nonces(a: Key, n: u64) {
    let k = utils::key_to_str(&a);
    let dict = Dict::instance(NONCES);
    dict.set(&k, n);
}

fn _delegates(delegator: Key) -> Key {
    runtime::print("reading delegate");
    let current = get_delegate(delegator);
    runtime::print("after reading delegate");
    if utils::is_null(current) {
        runtime::print("is null");
        return delegator;
    }
    runtime::print("is not null");
    current
}

#[no_mangle]
pub extern "C" fn delegates() {
    let delegator: Key = runtime::get_named_arg(DELEGATOR);
    runtime::ret(CLValue::from_t(delegator).unwrap_or_revert());
}

/**
* @notice Gets the current votes balance for `account`
* @param account The address to get votes balance
* @return The number of current votes for `account`
*/
#[no_mangle]
pub extern "C" fn get_votes() {
    let account: Key = runtime::get_named_arg(ARG_ADDRESS);
    let n_checkpoints = get_num_checkpoints(account);
    let mut ret = 0u128;
    let ts = current_block_timestamp_seconds();
    if n_checkpoints == 0 {
        ret = 0;
    } else {
        let _token_ids = get_check_point(account, n_checkpoints).token_ids;
        for i in 0.._token_ids.len() {
            let id = _token_ids[i];
            ret = ret + _balance_of_nft(id, ts);
        }
    }

    runtime::ret(CLValue::from_t(U128::from(ret)).unwrap_or_revert());
}

fn _get_past_votes_index(account: Key, timestamp: u64) -> u64 {
    let n_checkpoints = get_num_checkpoints(account);
    if n_checkpoints == 0 {
        return 0;
    } else {
        let cp = get_check_point(account, n_checkpoints - 1);
        // First check most recent balance
        if cp.timestamp <= timestamp.into() {
            return n_checkpoints - 1;
        } else if get_check_point(account, 0).timestamp > timestamp.into() {
            return 0;
        } else {
            let mut lower = 0u64;
            let mut upper = n_checkpoints - 1;
            while upper > lower {
                let center = upper - (upper - lower) / 2;
                let cp = get_check_point(account, center);
                if cp.timestamp == timestamp.into() {
                    return center;
                } else if cp.timestamp < timestamp.into() {
                    lower = center;
                } else {
                    upper = upper - 1;
                }
            }
            return lower;
        }
    }
}

#[no_mangle]
pub extern "C" fn get_past_votes_index() {
    let account: Key = runtime::get_named_arg(ARG_ADDRESS);
    let timestamp: u64 = runtime::get_named_arg(ARG_TIMESTAMP);
    let ret = _get_past_votes_index(account, timestamp);
    runtime::ret(CLValue::from_t(ret).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_past_votes() {
    let account: Key = runtime::get_named_arg(ARG_ADDRESS);
    let timestamp: u64 = runtime::get_named_arg(ARG_TIMESTAMP);
    let _check_index = _get_past_votes_index(account, timestamp);
    // Sum votes
    let _token_ids = get_check_point(account, _check_index).token_ids;
    let mut votes = 0u128;
    for tid in _token_ids {
        // Use the provided input timestamp here to get the right decay
        votes = votes + _balance_of_nft(tid, timestamp);
    }
    runtime::ret(CLValue::from_t(U128::from(votes)).unwrap_or_revert());
}

#[no_mangle]
pub extern "C" fn get_past_total_supply() {
    let timestamp: u64 = runtime::get_named_arg(ARG_TIMESTAMP);
    let ret = _total_supply_at_t(timestamp);
    runtime::ret(CLValue::from_t(U128::from(ret)).unwrap_or_revert());
}

////////////////////////////////////////////////////////////////
//                             DAO VOTING LOGIC
//////////////////////////////////////////////////////////////*/
pub(crate) fn _move_token_delegates(src: Key, dst: Key, token_id: u64) {
    runtime::print("hehre");
    if src != dst && token_id > 0 {
        if utils::is_not_null(src) {
            let src_rep_num = get_num_checkpoints(src);
            let cp = if src_rep_num > 0 {
                get_check_point(src, src_rep_num - 1)
            } else {
                get_check_point(src, 0)
            };

            let next_src_rep_num = _find_what_checkpoint_to_write(src);
            let mut cp_new = get_check_point(src, next_src_rep_num);
            for i in 0..cp.token_ids.len() {
                let id = cp.token_ids[i];
                if id != token_id {
                    cp_new.token_ids.push(id);
                }
            }
            set_check_point(src, next_src_rep_num, &cp_new);
            set_num_checkpoints(src, src_rep_num + 1);
        }

        if utils::is_not_null(dst) {
            let dst_rep_num = get_num_checkpoints(dst);
            let cp = if dst_rep_num > 0 {
                get_check_point(dst, dst_rep_num - 1)
            } else {
                get_check_point(dst, 0)
            };

            require(cp.token_ids.len() + 1 <= MAX_DELEGATES as usize, VeError::TooManyTokenIds);

            let next_dst_rep_num = _find_what_checkpoint_to_write(dst);
            let mut cp_new = get_check_point(dst, next_dst_rep_num);
            for i in 0..cp.token_ids.len() {
                let id = cp.token_ids[i];
                if id != token_id {
                    cp_new.token_ids.push(id);
                }
            }
            set_check_point(dst, next_dst_rep_num, &cp_new);
            set_num_checkpoints(dst, dst_rep_num + 1);
        }
    }
}

fn _find_what_checkpoint_to_write(account: Key) -> u64 {
    let _timestamp = current_block_timestamp_seconds();
    let n_checkpoints = get_num_checkpoints(account);
    if n_checkpoints > 0
        && get_check_point(account, n_checkpoints - 1).timestamp as u64 == _timestamp
    {
        return n_checkpoints - 1;
    } else {
        return n_checkpoints;
    }
}

fn _move_all_delegates(owner: Key, src: Key, dst: Key) {
    if src != dst {
        if utils::is_not_null(src) {
            let src_rep_num = get_num_checkpoints(src);
            let src_rep_old = if src_rep_num > 0 {
                get_check_point(src, src_rep_num - 1)
            } else {
                get_check_point(src, 0)
            };

            let next_src_rep_num = _find_what_checkpoint_to_write(src);
            let mut src_rep_new = get_check_point(src, next_src_rep_num);

            for tid in &src_rep_old.token_ids {
                if NFTToken::default().owner_of((*tid).into()).unwrap() != owner {
                    src_rep_new.token_ids.push(*tid);
                }
            }
            set_check_point(src, next_src_rep_num, &src_rep_new);
            set_num_checkpoints(src, src_rep_num + 1);
        }

        if utils::is_not_null(dst) {
            let dst_rep_num = get_num_checkpoints(dst);
            let dst_rep_old = if dst_rep_num > 0 {
                get_check_point(dst, dst_rep_num - 1)
            } else {
                get_check_point(dst, 0)
            };
            let next_dst_rep_num = _find_what_checkpoint_to_write(dst);
            let mut dst_rep_new = get_check_point(dst, next_dst_rep_num);
            let owner_token_count = NFTToken::default().balance_of(owner).as_usize();
            require(dst_rep_old.token_ids.len() + owner_token_count <= MAX_DELEGATES as usize, VeError::TooManyTokenIds);
            for tid in &dst_rep_old.token_ids {
                dst_rep_new.token_ids.push(*tid);
            }

            for i in 0..owner_token_count {
                let tid = NFTToken::default().get_token_by_index(owner, U256::from(i)).unwrap().as_u64();
                dst_rep_new.token_ids.push(tid);
            }
            set_check_point(dst, next_dst_rep_num, &dst_rep_new);
            set_num_checkpoints(dst, dst_rep_num + 1);
        }
    }
}

fn _delegate(delegator: Key, delegatee: Key) {
    let current_delegate = get_delegate(delegator);
    set_delegate(delegator, delegatee);

    _move_all_delegates(delegator, current_delegate, delegatee);
}

#[no_mangle]
pub extern "C" fn delegate() {
    let delegatee: Key = runtime::get_named_arg("delegatee");
    let caller = utils::get_immediate_caller_key();
    _delegate(caller, delegatee);
}

#[no_mangle]
pub extern "C" fn delegate_by_sig() {}

#[no_mangle]
pub extern "C" fn increase_amount_for() {
    let token_id: u64 = runtime::get_named_arg::<U256>(ARG_TOKEN_ID).as_u64();
    let amount: u128 = runtime::get_named_arg::<U128>(ARG_AMOUNT).as_u128();

    let caller = utils::get_immediate_caller_key();
    require(caller == get_key::<Key>(TEAM).unwrap(), VeError::NOTTEAM);

    let locked = get_locked_balance(token_id);
    require(amount > 0, VeError::InvalidAmount);

    require(locked.amount > 0, VeError::NoExistingLock);
    require(locked.end > current_block_timestamp_seconds(), VeError::CannotAddToExpiredLock);

    _deposit_for(token_id, amount, 0, &locked, INCREASE_LOCK_AMOUNT);
}

pub fn get_entry_points() -> EntryPoints {
    let mut entry_points = EntryPoints::new();
    entry_points.add_entry_point(EntryPoint::new(
        "increase_amount_for",
        vec![
            Parameter::new(ARG_TOKEN_ID, U256::cl_type()),
            Parameter::new(ARG_AMOUNT, U128::cl_type()),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "get_past_votes_index",
        vec![
            Parameter::new(ARG_ADDRESS, Key::cl_type()),
            Parameter::new(ARG_TIMESTAMP, u64::cl_type()),
        ],
        CLType::U64,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "get_past_votes",
        vec![
            Parameter::new(ARG_ADDRESS, Key::cl_type()),
            Parameter::new(ARG_TIMESTAMP, u64::cl_type()),
        ],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "get_past_total_supply",
        vec![
            Parameter::new(ARG_TIMESTAMP, u64::cl_type()),
        ],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "delegate",
        vec![Parameter::new("delegatee", Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "get_votes",
        vec![Parameter::new(ARG_ADDRESS, Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "delegates",
        vec![Parameter::new(DELEGATOR, Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "merge",
        vec![
            Parameter::new(ARG_FROM, U256::cl_type()),
            Parameter::new(ARG_TO, U256::cl_type()),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "detach",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "attach",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "abstain",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "voting",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "set_voter",
        vec![Parameter::new(VOTER, Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "total_supply_at_t",
        vec![Parameter::new(ARG_T, u64::cl_type())],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "ve_total_supply",
        vec![],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "total_supply_at",
        vec![Parameter::new(BLOCK, u64::cl_type())],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "balance_of_at_nft",
        vec![
            Parameter::new(ARG_TOKEN_ID, U256::cl_type()),
            Parameter::new(BLOCK, u64::cl_type()),
        ],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "balance_of_nft_at",
        vec![
            Parameter::new(ARG_TOKEN_ID, U256::cl_type()),
            Parameter::new(EPOCH_TIME, u64::cl_type()),
        ],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "balance_of_nft",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    //TODO
    entry_points.add_entry_point(EntryPoint::new(
        "withdraw",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "increase_unlock_time",
        vec![
            Parameter::new(ARG_TOKEN_ID, U256::cl_type()),
            Parameter::new(ARG_LOCK_DURATION, u64::cl_type()),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "increase_amount",
        vec![
            Parameter::new(ARG_TOKEN_ID, U256::cl_type()),
            Parameter::new(ARG_AMOUNT, U128::cl_type()),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "create_lock_for",
        vec![
            Parameter::new(ARG_LOCK_DURATION, u64::cl_type()),
            Parameter::new(ARG_AMOUNT, U128::cl_type()),
            Parameter::new(ARG_TO, Key::cl_type()),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "create_lock",
        vec![
            Parameter::new(ARG_LOCK_DURATION, u64::cl_type()),
            Parameter::new(ARG_AMOUNT, U128::cl_type()),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "deposit_for",
        vec![
            Parameter::new(ARG_TOKEN_ID, U256::cl_type()),
            Parameter::new(ARG_AMOUNT, U128::cl_type()),
        ],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "check_point",
        vec![],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "locked_end",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        CLType::U64,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "user_point_history__ts",
        vec![
            Parameter::new(ARG_TOKEN_ID, U256::cl_type()),
            Parameter::new(EPOCH_INDEX, u64::cl_type()),
        ],
        CLType::U128,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "get_last_user_slope",
        vec![Parameter::new(ARG_TOKEN_ID, U256::cl_type())],
        I128::cl_type(),
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "set_art_proxy",
        vec![Parameter::new("new_art_proxy", Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points.add_entry_point(EntryPoint::new(
        "set_team",
        vec![Parameter::new("new_team", Key::cl_type())],
        CLType::Unit,
        EntryPointAccess::Public,
        EntryPointType::Contract,
    ));

    entry_points
}
