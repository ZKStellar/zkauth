#![cfg(test)]

use super::*;
use soroban_sdk::{testutils::{Address as _, BytesN as _}, Address, Bytes, BytesN, Env};

fn create_token_contract<'a>(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract_v2(admin.clone()).address()
}

#[test]
fn test_deposit_zero_or_negative_amount_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let contract_id = env.register(ZkPay, (Bytes::new(&env), token.clone()));
    let client = ZkPayClient::new(&env, &contract_id);

    let depositor = Address::generate(&env);
    let commitment = BytesN::random(&env);

    // Negative amount check
    let res_neg = client.try_deposit(&depositor, &commitment, &-100);
    assert_eq!(res_neg, Err(Ok(Error::InvalidAmount)));

    // Zero amount check
    let res_zero = client.try_deposit(&depositor, &commitment, &0);
    assert_eq!(res_zero, Err(Ok(Error::InvalidAmount)));
}

#[test]
fn test_commitment_collision_rejected() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let contract_id = env.register(ZkPay, (Bytes::new(&env), token.clone()));
    let client = ZkPayClient::new(&env, &contract_id);

    let token_admin_client = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    let depositor = Address::generate(&env);
    token_admin_client.mint(&depositor, &10_000);

    let commitment = BytesN::random(&env);

    // First deposit succeeds
    let res1 = client.try_deposit(&depositor, &commitment, &1_000);
    assert_eq!(res1, Ok(Ok(())));
    assert!(client.commitment_exists(&commitment));

    // Duplicate deposit with same commitment fails
    let res2 = client.try_deposit(&depositor, &commitment, &2_000);
    assert_eq!(res2, Err(Ok(Error::CommitmentAlreadyExists)));
}

#[test]
fn test_nullifier_used_getter() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let contract_id = env.register(ZkPay, (Bytes::new(&env), token.clone()));
    let client = ZkPayClient::new(&env, &contract_id);

    let nullifier = BytesN::random(&env);
    assert!(!client.nullifier_used(&nullifier));
}
