#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, token::Client as TokenClient,
    Address, Bytes, BytesN, Env, Symbol,
};
use ultrahonk_soroban_verifier::{UltraHonkVerifier, VkLoadError, PROOF_BYTES};

const VK: Symbol = symbol_short!("VK");
const NATIVE: Symbol = symbol_short!("NATIVE");

#[contracterror]
#[repr(u32)]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum Error {
    VkInvalidLength = 1,
    VkInvalidParameters = 2,
    ProofParseError = 3,
    VerificationFailed = 4,
    VkNotSet = 5,
    NullifierAlreadyUsed = 6,
    CommitmentNotFound = 7,
    InvalidPublicInputsLength = 8,
    CommitmentAlreadyExists = 9,
    InvalidAmount = 10,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Deposit {
    pub amount: i128,
}

#[contract]
pub struct ZkPay;

#[contractimpl]
impl ZkPay {
    pub fn __constructor(env: Env, vk_bytes: Bytes, native_token: Address) {
        env.storage().instance().set(&VK, &vk_bytes);
        env.storage().instance().set(&NATIVE, &native_token);
    }

    pub fn deposit(
        env: Env,
        depositor: Address,
        commitment: BytesN<32>,
        amount: i128,
    ) -> Result<(), Error> {
        depositor.require_auth();
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }
        if env.storage().persistent().has(&commitment) {
            return Err(Error::CommitmentAlreadyExists);
        }

        let token: Address = env.storage().instance().get(&NATIVE).ok_or(Error::VkNotSet)?;
        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&depositor, &env.current_contract_address(), &amount);

        let deposit = Deposit { amount };
        env.storage().persistent().set(&commitment, &deposit);
        env.storage().persistent().extend_ttl(&commitment, 100_000, 1_000_000);
        Ok(())
    }

    pub fn withdraw(
        env: Env,
        proof: Bytes,
        public_inputs: Bytes,
        recipient: Address,
    ) -> Result<(), Error> {
        if public_inputs.len() != 64u32 {
            return Err(Error::InvalidPublicInputsLength);
        }
        if proof.len() as u32 != PROOF_BYTES as u32 {
            return Err(Error::ProofParseError);
        }

        let vk_bytes: Bytes = env.storage().instance().get(&VK).ok_or(Error::VkNotSet)?;
        let verifier = UltraHonkVerifier::new(&env, &vk_bytes).map_err(|e| match e {
            VkLoadError::WrongLength => Error::VkInvalidLength,
            VkLoadError::InvalidParameters => Error::VkInvalidParameters,
        })?;
        verifier
            .verify(&env, &proof, &public_inputs)
            .map_err(|_| Error::VerificationFailed)?;

        let mut commitment_arr = [0u8; 32];
        for i in 0u32..32u32 {
            commitment_arr[i as usize] = public_inputs.get_unchecked(i);
        }
        let commitment = BytesN::from_array(&env, &commitment_arr);

        let mut nullifier_arr = [0u8; 32];
        for i in 32u32..64u32 {
            nullifier_arr[(i - 32u32) as usize] = public_inputs.get_unchecked(i);
        }
        let nullifier = BytesN::from_array(&env, &nullifier_arr);

        if env.storage().persistent().has(&nullifier) {
            return Err(Error::NullifierAlreadyUsed);
        }
        env.storage().persistent().set(&nullifier, &());
        env.storage().persistent().extend_ttl(&nullifier, 100_000, 1_000_000);

        let deposit: Deposit = env
            .storage()
            .persistent()
            .get(&commitment)
            .ok_or(Error::CommitmentNotFound)?;

        let token: Address = env.storage().instance().get(&NATIVE).unwrap();
        let token_client = TokenClient::new(&env, &token);
        token_client.transfer(&env.current_contract_address(), &recipient, &deposit.amount);

        env.storage().persistent().remove(&commitment);

        Ok(())
    }

    pub fn commitment_exists(env: Env, commitment: BytesN<32>) -> bool {
        env.storage().persistent().has(&commitment)
    }

    pub fn nullifier_used(env: Env, nullifier: BytesN<32>) -> bool {
        env.storage().persistent().has(&nullifier)
    }
}

#[cfg(test)]
mod test;

