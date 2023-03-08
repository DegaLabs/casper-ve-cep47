//! Implementation of an `Address` which refers either an account hash, or a contract hash.
use alloc::vec::Vec;
use casper_types::{
    bytesrepr::{self, FromBytes, ToBytes},
    CLType, CLTyped
};
use core::convert::TryInto;

/// An enum representing an [`AccountHash`] or a [`ContractPackageHash`].
#[derive(PartialOrd, Ord, PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct I128 {
    pub bits: i128
}

impl I128 {
    /// Returns the inner account hash if `self` is the `Account` variant.
    pub fn as_i128(&self) -> i128 {
        self.bits
    }

    /// Returns the inner contract hash if `self` is the `Contract` variant.
    pub fn as_u128(&self) -> Option<u128> {
        if self.as_i128() >= 0 {
            Some(self.bits.unsigned_abs())
        } else {
            None
        }
    }
}

impl From<i128> for I128 {
    fn from(x: i128) -> Self {
        I128 { bits: x }
    }
}


impl CLTyped for I128 {
    fn cl_type() -> casper_types::CLType {
        CLType::Any
    }
}

impl ToBytes for I128 {
    fn to_bytes(&self) -> Result<Vec<u8>, bytesrepr::Error> {
        Ok(self.bits.to_le_bytes().to_vec())
    }

    fn serialized_length(&self) -> usize {
        16
    }
}

impl FromBytes for I128 {
    fn from_bytes(b: &[u8]) -> Result<(Self, &[u8]), bytesrepr::Error> {
        let bytes: [u8; 16] = b[0..16].try_into().unwrap();
        let x = i128::from_le_bytes(bytes);

        Ok((I128 { bits: x }, &b[16..b.len()]))
    }
}