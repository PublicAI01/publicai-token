

---

# Fungible Token Contract

This NEAR smart contract implements a fungible token (FT) based on the NEAR Fungible Token Standard. The contract supports minting, transferring, metadata management, storage management, and resolving transfers. It uses NEAR's contract standards and provides comprehensive functionality for fungible token operations.

---

## Features

1. **Minting Tokens**:
    - Tokens are minted during contract initialization, with the total supply owned by the `owner_id`.

2. **Token Metadata**:
    - Supports metadata management, including name, symbol, decimals, and more.

3. **Token Transfers**:
    - Users can transfer tokens to other accounts or interact with other contracts using `ft_transfer_call`.

4. **Storage Management**:
    - Users can register accounts, deposit storage, withdraw storage, and unregister accounts.

5. **Transfer Resolution**:
    - Handles transfer resolution when the receiver contract fails or partially uses the transferred tokens.

6. **Compliance with NEAR Standards**:
    - Implements NEAR's fungible token standard, metadata standard, and storage management standard.

---

## Contract Details

### Constants

- **`FT_METADATA_SPEC`**: Specification version of the fungible token metadata.
- **`TOTAL_SUPPLY`**: The initial total supply of tokens, defined during contract initialization.

---

### Contract Initialization

```rust
pub fn new(owner_id: AccountId, total_supply: U128, metadata: FungibleTokenMetadata) -> Self
```

Initializes the contract with the following parameters:
- `owner_id`: The account ID of the contract owner, who receives the total supply.
- `total_supply`: The total supply of tokens minted during initialization.
- `metadata`: The metadata for the fungible token, including name, symbol, decimals, and other details.

#### Example Command:
```bash
near call <contract_account_id> new '{"owner_id": "<owner_account_id>", "total_supply": "1000000000000000000000000", "metadata": {"spec": "ft-1.0.0", "name": "Example Token", "symbol": "EXAMPLE", "decimals": 18}}' --accountId <owner_account_id>
```

---

### Token Transfers

#### Transfer Tokens

```rust
fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>)
```

Allows users to transfer tokens to another account.

#### Example Command:
```bash
near call <contract_account_id> ft_transfer '{"receiver_id": "<receiver_account_id>", "amount": "1000000000000000000", "memo": "Transfer memo"}' --accountId <sender_account_id> --depositYocto 1
```

---

#### Transfer Tokens with Callback

```rust
fn ft_transfer_call(
    &mut self,
    receiver_id: AccountId,
    amount: U128,
    memo: Option<String>,
    msg: String,
) -> PromiseOrValue<U128>
```

Allows users to transfer tokens to another account or contract and includes a callback for post-transfer interactions.

#### Example Command:
```bash
near call <contract_account_id> ft_transfer_call '{"receiver_id": "<receiver_contract_id>", "amount": "1000000000000000000", "memo": "Transfer memo", "msg": "Callback message"}' --accountId <sender_account_id> --depositYocto 1
```

---

### Metadata Management

```rust
fn ft_metadata(&self) -> FungibleTokenMetadata
```

Returns the metadata of the fungible token, including name, symbol, decimals, and other details.

#### Example Command:
```bash
near view <contract_account_id> ft_metadata
```

---

### Storage Management

#### Deposit Storage

```rust
fn storage_deposit(
    &mut self,
    account_id: Option<AccountId>,
    registration_only: Option<bool>,
) -> StorageBalance
```

Allows users to deposit storage for registering accounts.

#### Example Command:
```bash
near call <contract_account_id> storage_deposit '{"account_id": "<account_id>", "registration_only": false}' --accountId <sender_account_id> --depositYocto 1000000000000000000000
```

---

#### Withdraw Storage

```rust
fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance
```

Allows users to withdraw storage deposits.

---

#### Unregister Account

```rust
fn storage_unregister(&mut self, force: Option<bool>) -> bool
```

Unregisters an account and optionally force closes the account.

---

### Transfer Resolution

```rust
fn ft_resolve_transfer(
    &mut self,
    sender_id: AccountId,
    receiver_id: AccountId,
    amount: U128,
) -> U128
```

Handles transfer resolution when the receiver contract fails or does not use the full transferred amount.

---

## Testing

### Test Scenarios

The provided tests cover the following scenarios:
1. **Contract Initialization**:
    - Verifies the contract initializes correctly with the specified `owner_id`, `total_supply`, and metadata.

2. **Token Transfers**:
    - Ensures tokens can be transferred between accounts.
    - Checks for invalid transfers, such as transferring to self or transferring zero tokens.

3. **Metadata Management**:
    - Tests that the metadata is set correctly and can be updated.

4. **Storage Management**:
    - Tests storage deposit, withdrawal, and unregister functionality.
    - Verifies edge cases, such as unregistering accounts with non-zero balances.

5. **Transfer Resolution**:
    - Ensures transfers are resolved correctly when the receiver contract fails or partially uses the tokens.

---

### Test Commands

Run the tests using the following command:
```bash
cargo test -- --nocapture
```

---

## Example Test Cases

### Contract Initialization

```rust
#[test]
fn test_new() {
    let (contract, _) = setup();
    assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
    assert_eq!(contract.ft_balance_of(owner()).0, TOTAL_SUPPLY);
}
```

### Token Transfer

```rust
#[test]
fn test_transfer() {
    let (mut contract, mut context) = setup();
    testing_env!(context
        .predecessor_account_id(owner())
        .attached_deposit(NearToken::from_yoctonear(1))
        .build());
    let transfer_amount = TOTAL_SUPPLY / 10;
    contract.ft_transfer(user1(), transfer_amount.into(), None);
    assert_eq!(contract.ft_balance_of(owner()).0, TOTAL_SUPPLY - transfer_amount);
    assert_eq!(contract.ft_balance_of(user1()).0, transfer_amount);
}
```

---

## Notes

- Ensure the `owner_id` has sufficient storage and tokens before initializing the contract.
- This contract complies with NEAR's fungible token standards, including metadata and storage management.
- Gas considerations should be taken into account for `ft_transfer_call` operations.

---

## License

This contract is open-source and available under the MIT License.

---