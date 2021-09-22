use casper_contract::{contract_api::runtime, unwrap_or_revert::UnwrapOrRevert};
use casper_types::{bytesrepr::ToBytes, ApiError, Key, U256};
use cep47_logic::{CEP47Storage, Meta, TokenId};

use crate::data::{self, Issuers, Metadata, OwnedTokens, Owners};

#[derive(Default)]
pub struct CasperCEP47Storage {}
impl CasperCEP47Storage {
    pub fn new() -> CasperCEP47Storage {
        CasperCEP47Storage {}
    }
}

impl CEP47Storage for CasperCEP47Storage {
    fn name(&self) -> String {
        data::name()
    }

    fn symbol(&self) -> String {
        data::symbol()
    }

    fn meta(&self) -> Meta {
        data::meta()
    }

    fn balance_of(&self, owner: &Key) -> U256 {
        OwnedTokens::instance().get_balance(owner)
    }

    fn owner_of(&self, token_id: &TokenId) -> Option<Key> {
        Owners::instance().get(token_id)
    }

    fn total_supply(&self) -> U256 {
        data::total_supply()
    }

    fn token_meta(&self, token_id: &TokenId) -> Option<Meta> {
        Metadata::instance().get(token_id)
    }

    fn issuer_of(&self, token_id: &TokenId) -> Option<Key> {
        Issuers::instance().get(token_id)
    }

    fn is_paused(&self) -> bool {
        data::is_paused()
    }

    fn pause(&mut self) {
        data::pause();
    }

    fn unpause(&mut self) {
        data::unpause();
    }

    fn mint_many(
        &mut self,
        issuer: &Key,
        recipient: &Key,
        token_ids: &Vec<TokenId>,
        token_metas: &Vec<Meta>,
    ) {
        // Prepare dictionaries.
        let owners_dict = Owners::instance();
        let issuers_dict = Issuers::instance();
        let owned_tokens = OwnedTokens::instance();
        let metadata_dict = Metadata::instance();

        // Create new tokens.
        for (token_id, token_meta) in token_ids.iter().zip(token_metas) {
            // Set metadata.
            metadata_dict.set(token_id, token_meta.clone());

            // Set token owner.
            owners_dict.set(token_id, *recipient);

            // Set token issuer.
            issuers_dict.set(token_id, *issuer);

            // Update list of owned tokens.
            owned_tokens.set_token(recipient, token_id);
        }

        // Update recipient's balance.
        let new_tokens_count: U256 = token_ids.len().into();

        // Update total supply.
        let new_total_supply = data::total_supply() + new_tokens_count;
        data::update_total_supply(new_total_supply);
    }

    fn transfer_many(&mut self, sender: &Key, recipient: &Key, token_ids: &Vec<TokenId>) {
        // Prepare dictionaries.
        let owners_dict = Owners::instance();
        let owned_tokens = OwnedTokens::instance();

        // Update ownerships.
        for token_id in token_ids {
            owners_dict.set(token_id, *recipient);
            owned_tokens.remove_token(sender, token_id);
            owned_tokens.set_token(recipient, token_id);
        }
    }

    fn burn_many(&mut self, owner: &Key, token_ids: &Vec<TokenId>) {
        // Prepare dictionaries.
        let owners_dict = Owners::instance();
        let issuers_dict = Issuers::instance();
        let owned_tokens = OwnedTokens::instance();
        let metadata_dict = Metadata::instance();

        // Remove tokens.
        for token_id in token_ids {
            // Remove meta.
            metadata_dict.remove(token_id);

            // Remove ownership.
            owners_dict.remove(token_id);

            // Remove issuer info.
            issuers_dict.remove(token_id);

            // Remove owned tokens.
            owned_tokens.remove_token(owner, token_id);
        }

        // Decrement total supply.
        let amount: U256 = token_ids.len().into();
        let new_total_supply = data::total_supply() - amount;
        data::update_total_supply(new_total_supply);
    }

    fn update_token_metadata(&mut self, token_id: &TokenId, meta: Meta) {
        let metadata_dict = Metadata::instance();
        let current_meta = metadata_dict.get(token_id);
        match current_meta {
            None => runtime::revert(ApiError::None),
            Some(_) => metadata_dict.set(token_id, meta),
        };
    }

    fn gen_token_ids(&mut self, n: u32) -> Vec<TokenId> {
        let block_time = runtime::get_blocktime();
        let mut token_ids = Vec::new();
        let nonce = data::get_nonce();
        for i in nonce..nonce + n {
            let mut bytes: Vec<u8> = block_time.to_bytes().unwrap_or_revert();
            bytes.append(&mut i.to_bytes().unwrap_or_revert());
            let hash = runtime::blake2b(bytes);
            token_ids.push(hex::encode(hash));
        }
        data::set_nonce(nonce + n);
        token_ids
    }

    fn validate_token_ids(&self, token_ids: &Vec<TokenId>) -> bool {
        for token_id in token_ids {
            if self.owner_of(token_id).is_some() {
                return false;
            }
        }
        true
    }

    fn are_all_owner_tokens(&self, owner: &Key, token_ids: &Vec<TokenId>) -> bool {
        let owners_dict = Owners::instance();
        for token_id in token_ids.iter() {
            let token_owner = owners_dict.get(token_id);
            if let Some(token_owner) = token_owner {
                if &token_owner != owner {
                    return false;
                }
            }
        }
        true
    }

    fn emit(&mut self, event: cep47_logic::events::CEP47Event) {
        data::emit(&event)
    }

    fn contact_package_hash(&self) -> casper_types::ContractPackageHash {
        data::contract_package_hash()
    }
}