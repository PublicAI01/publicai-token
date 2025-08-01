use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::{
    FungibleToken, FungibleTokenCore, FungibleTokenResolver,
};
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::borsh::BorshSerialize;
use near_sdk::collections::LazyOption;
use near_sdk::json_types::U128;
use near_sdk::{
    assert_one_yocto, env, log, near, require, AccountId, BorshStorageKey, NearToken,
    PanicOnDefault, PromiseOrValue,
};

#[derive(PanicOnDefault)]
#[near(contract_state)]
pub struct Contract {
    owner_id: AccountId,
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}
#[derive(BorshSerialize, BorshStorageKey)]
#[borsh(crate = "near_sdk::borsh")]
enum StorageKey {
    FungibleToken,
    Metadata,
}

#[near]
impl Contract {
    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(owner_id: AccountId, total_supply: U128, metadata: FungibleTokenMetadata) -> Self {
        require!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            owner_id: owner_id.clone(),
            token: FungibleToken::new(StorageKey::FungibleToken),
            metadata: LazyOption::new(StorageKey::Metadata, Some(&metadata)),
        };
        this.token.internal_register_account(&owner_id);
        this.token.internal_deposit(&owner_id, total_supply.into());

        near_contract_standards::fungible_token::events::FtMint {
            owner_id: &owner_id,
            amount: total_supply,
            memo: Some("new tokens are minted"),
        }
        .emit();

        this
    }

    #[payable]
    pub fn update_metadata(&mut self, metadata: FungibleTokenMetadata) {
        assert_one_yocto();
        require!(self.owner_id == env::predecessor_account_id(), "Not allow");
        metadata.assert_valid();
        let current_metadata = self.metadata.get().unwrap();
        require!(
            current_metadata.decimals == metadata.decimals,
            "Can't change decimals"
        );
        self.metadata.set(&metadata);
    }

    #[payable]
    pub fn update_owner(&mut self, new_owner: AccountId) -> bool {
        assert_one_yocto();
        require!(
            env::predecessor_account_id() == self.owner_id,
            "Owner's method"
        );
        require!(!new_owner.as_str().is_empty(), "New owner cannot be empty");
        log!("Owner updated from {} to {}", self.owner_id, new_owner);
        self.owner_id = new_owner;
        true
    }
}

#[near]
impl FungibleTokenCore for Contract {
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for Contract {
    #[private]
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        let (used_amount, burned_amount) =
            self.token
                .internal_ft_resolve_transfer(&sender_id, receiver_id, amount);
        if burned_amount > 0 {
            log!("Account @{} burned {}", sender_id, burned_amount);
        }
        used_amount.into()
    }
}

#[near]
impl StorageManagement for Contract {
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        self.token.storage_withdraw(amount)
    }

    #[payable]
    fn storage_unregister(&mut self, force: Option<bool>) -> bool {
        #[allow(unused_variables)]
        if let Some((account_id, balance)) = self.token.internal_storage_unregister(force) {
            log!("Closed @{} with {}", account_id, balance);
            true
        } else {
            false
        }
    }

    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata
            .get()
            .unwrap_or_else(|| FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: String::default(),
                symbol: String::default(),
                icon: None,
                reference: None,
                reference_hash: None,
                decimals: 0,
            })
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_contract_standards::fungible_token::Balance;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::{testing_env, Gas};

    use super::*;

    const TOTAL_SUPPLY: Balance = 1_000_000_000_000_000;

    fn current() -> AccountId {
        accounts(0)
    }

    fn owner() -> AccountId {
        accounts(1)
    }

    fn user1() -> AccountId {
        accounts(2)
    }

    fn user2() -> AccountId {
        accounts(3)
    }

    fn setup() -> (Contract, VMContextBuilder) {
        let mut context = VMContextBuilder::new();

        let contract = Contract::new(
            owner(),
            TOTAL_SUPPLY.into(),
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "Example NEAR fungible token".to_string(),
                symbol: "EXAMPLE".to_string(),
                icon: None,
                reference: None,
                reference_hash: None,
                decimals: 24,
            },
        );

        context.storage_usage(env::storage_usage());
        context.current_account_id(current());

        testing_env!(context.build());

        (contract, context)
    }

    #[test]
    fn test_new() {
        let (contract, _) = setup();

        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(owner()).0, TOTAL_SUPPLY);
    }

    #[test]
    fn test_metadata() {
        let (contract, _) = setup();

        assert_eq!(contract.ft_metadata().decimals, 24);
        assert!(contract.ft_metadata().icon.is_none());
        assert!(!contract.ft_metadata().spec.is_empty());
        assert!(!contract.ft_metadata().name.is_empty());
        assert!(!contract.ft_metadata().symbol.is_empty());
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default_panics() {
        Contract::default();
    }

    #[test]
    fn test_deposit() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        assert!(contract.storage_balance_of(user1()).is_none());

        contract.storage_deposit(None, None);

        let storage_balance = contract.storage_balance_of(user1()).unwrap();
        assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
        assert!(storage_balance.available.is_zero());
    }

    #[test]
    fn test_deposit_on_behalf_of_another_user() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        assert!(contract.storage_balance_of(user2()).is_none());

        // predecessor is user1, but deposit is for user2
        contract.storage_deposit(Some(user2()), None);

        let storage_balance = contract.storage_balance_of(user2()).unwrap();
        assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
        assert!(storage_balance.available.is_zero());

        // ensure that user1's storage wasn't affected
        assert!(contract.storage_balance_of(user1()).is_none());
    }

    #[should_panic]
    #[test]
    fn test_deposit_panics_on_less_amount() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(100))
            .build());

        assert!(contract.storage_balance_of(user1()).is_none());

        // this panics
        contract.storage_deposit(None, None);
    }

    #[test]
    fn test_deposit_account_twice() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // this registers the predecessor
        contract.storage_deposit(None, None);

        let storage_balance = contract.storage_balance_of(user1()).unwrap();
        assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);

        // this doesn't panic, and just refunds the deposit as the account is registered already
        contract.storage_deposit(None, None);

        // this indicates that total balance hasn't changed
        let storage_balance = contract.storage_balance_of(user1()).unwrap();
        assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
    }

    #[test]
    fn test_unregister() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        contract.storage_deposit(None, None);

        assert!(contract.storage_balance_of(user1()).is_some());

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        assert_eq!(contract.storage_unregister(None), true);

        assert!(contract.storage_balance_of(user1()).is_none());
    }

    #[should_panic]
    #[test]
    fn test_unregister_panics_on_zero_deposit() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        contract.storage_deposit(None, None);

        assert!(contract.storage_balance_of(user1()).is_some());

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(0))
            .build());

        contract.storage_unregister(None);
    }

    #[test]
    fn test_unregister_of_non_registered_account() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        // "false" indicates that the account wasn't registered
        assert_eq!(contract.storage_unregister(None), false);
    }

    #[should_panic]
    #[test]
    fn test_unregister_panics_on_non_zero_balance() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        contract.storage_deposit(None, None);

        assert!(contract.storage_balance_of(user1()).is_some());

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 10;

        contract.ft_transfer(user1(), transfer_amount.into(), None);

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        contract.storage_unregister(None);
    }

    #[test]
    fn test_unregister_with_force() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        contract.storage_deposit(None, None);

        assert!(contract.storage_balance_of(user1()).is_some());

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 10;

        contract.ft_transfer(user1(), transfer_amount.into(), None);

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        // force to unregister no matter what
        // this reduces total supply because user's tokens are burnt
        assert_eq!(contract.storage_unregister(Some(true)), true);

        assert!(contract.storage_balance_of(user1()).is_none());
        assert_eq!(contract.ft_balance_of(user1()).0, 0);
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY - transfer_amount);
    }

    #[test]
    fn test_withdraw() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        // Basic Fungible Token implementation never transfers Near to caller
        // See: https://github.com/near/near-sdk-rs/blob/5a4c595125364ffe8d7866aa0418a3c92b1c3a6a/near-contract-standards/src/fungible_token/storage_impl.rs#L82
        let storage_balance = contract.storage_withdraw(None);
        assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
        assert!(storage_balance.available.is_zero());

        // Basic Fungible Token implementation never transfers Near to caller
        // See: https://github.com/near/near-sdk-rs/blob/5a4c595125364ffe8d7866aa0418a3c92b1c3a6a/near-contract-standards/src/fungible_token/storage_impl.rs#L82
        let storage_balance = contract.storage_withdraw(None);
        assert_eq!(storage_balance.total, contract.storage_balance_bounds().min);
        assert!(storage_balance.available.is_zero());
    }

    #[should_panic]
    #[test]
    fn test_withdraw_panics_on_non_registered_account() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        contract.storage_withdraw(None);
    }

    #[should_panic]
    #[test]
    fn test_withdraw_panics_on_zero_deposit() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(0))
            .build());

        contract.storage_withdraw(None);
    }

    #[should_panic]
    #[test]
    fn test_withdraw_panics_on_amount_greater_than_zero() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        // Basic Fungible Token implementation sets storage_balance_bounds.min == storage_balance_bounds.max
        // which means available balance will always be 0
        // See: https://github.com/near/near-sdk-rs/blob/5a4c595125364ffe8d7866aa0418a3c92b1c3a6a/near-contract-standards/src/fungible_token/storage_impl.rs#L82
        contract.storage_withdraw(Some(NearToken::from_yoctonear(1)));
    }

    #[test]
    fn test_transfer() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 10;

        contract.ft_transfer(user1(), transfer_amount.into(), None);

        assert_eq!(
            contract.ft_balance_of(owner()).0,
            (TOTAL_SUPPLY - transfer_amount)
        );
        assert_eq!(contract.ft_balance_of(user1()).0, transfer_amount);
    }

    #[should_panic]
    #[test]
    fn test_transfer_panics_on_self_receiver() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 10;

        contract.ft_transfer(owner(), transfer_amount.into(), None);
    }

    #[should_panic]
    #[test]
    fn test_transfer_panics_on_zero_amount() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        contract.ft_transfer(user1(), 0.into(), None);
    }

    #[should_panic]
    #[test]
    fn test_transfer_panics_on_zero_deposit() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(0))
            .build());

        let transfer_amount = TOTAL_SUPPLY / 10;
        contract.ft_transfer(user1(), transfer_amount.into(), None);
    }

    #[should_panic]
    #[test]
    fn test_transfer_panics_on_non_registered_sender() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        let transfer_amount = TOTAL_SUPPLY / 10;
        contract.ft_transfer(user1(), transfer_amount.into(), None);
    }

    #[should_panic]
    #[test]
    fn test_transfer_panics_on_non_registered_receiver() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        let transfer_amount = TOTAL_SUPPLY / 10;
        contract.ft_transfer(user1(), transfer_amount.into(), None);
    }

    #[should_panic]
    #[test]
    fn test_transfer_panics_on_amount_greater_than_balance() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        let transfer_amount = TOTAL_SUPPLY + 10;
        contract.ft_transfer(user1(), transfer_amount.into(), None);
    }

    #[test]
    fn test_transfer_call() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 10;

        contract.ft_transfer_call(user1(), transfer_amount.into(), None, "".to_string());

        assert_eq!(
            contract.ft_balance_of(owner()).0,
            (TOTAL_SUPPLY - transfer_amount)
        );
        assert_eq!(contract.ft_balance_of(user1()).0, transfer_amount);
    }

    #[should_panic]
    #[test]
    fn test_transfer_call_panics_on_self_receiver() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 10;

        contract.ft_transfer_call(owner(), transfer_amount.into(), None, "".to_string());
    }

    #[should_panic]
    #[test]
    fn test_transfer_call_panics_on_zero_amount() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        contract.ft_transfer_call(user1(), 0.into(), None, "".to_string());
    }

    #[should_panic]
    #[test]
    fn test_transfer_call_panics_on_zero_deposit() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(0))
            .build());

        let transfer_amount = TOTAL_SUPPLY / 10;
        contract.ft_transfer_call(user1(), transfer_amount.into(), None, "".to_string());
    }

    #[should_panic]
    #[test]
    fn test_transfer_call_panics_on_non_registered_sender() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        let transfer_amount = TOTAL_SUPPLY / 10;
        contract.ft_transfer_call(user1(), transfer_amount.into(), None, "".to_string());
    }

    #[should_panic]
    #[test]
    fn test_transfer_call_panics_on_non_registered_receiver() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        let transfer_amount = TOTAL_SUPPLY / 10;
        contract.ft_transfer_call(user1(), transfer_amount.into(), None, "".to_string());
    }

    #[should_panic]
    #[test]
    fn test_transfer_call_panics_on_amount_greater_than_balance() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        let transfer_amount = TOTAL_SUPPLY + 10;
        contract.ft_transfer_call(user1(), transfer_amount.into(), None, "".to_string());
    }
    #[should_panic]
    #[test]
    fn test_transfer_call_panics_on_unsufficient_gas() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(user1())
            .attached_deposit(contract.storage_balance_bounds().min)
            .build());

        // Paying for account registration of user1, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .prepaid_gas(Gas::from_tgas(10))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 10;

        contract.ft_transfer_call(user1(), transfer_amount.into(), None, "".to_string());
    }

    #[test]
    fn test_update_owner_success() {
        let (mut contract, mut context) = setup();

        testing_env!(context
            .predecessor_account_id(owner())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());
        let new_owner: AccountId = "bob.testnet".parse().unwrap();
        let result = contract.update_owner(new_owner.clone());
        assert!(result, "Owner should be updated successfully");
        assert_eq!(contract.owner_id, new_owner);
    }

    #[test]
    #[should_panic(expected = "Owner's method")]
    fn test_update_owner_only_owner_can_call() {
        let (mut contract, mut context) = setup();
        let old_owner: AccountId = "alice.testnet".parse().unwrap();
        let new_owner: AccountId = "bob.testnet".parse().unwrap();
        testing_env!(context
            .predecessor_account_id(old_owner)
            .attached_deposit(NearToken::from_yoctonear(1))
            .build());

        contract.update_owner(new_owner.clone());
    }
}
