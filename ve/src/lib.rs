#![no_std]
#[macro_use]
extern crate alloc;

mod cep47;
pub mod data;
pub mod event;
pub mod vedata;
pub mod I128;
pub mod error;
pub mod utils;
pub mod erc20_helpers;
pub mod lock;
pub mod dict;

pub use cep47::{Error, CEP47, NFTToken};

use alloc::{collections::BTreeMap, string::String};
use casper_types::U256;
pub type TokenId = U256;
pub type Meta = BTreeMap<String, String>;


