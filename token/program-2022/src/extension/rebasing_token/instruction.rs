#[cfg(feature = "serde-traits")]
use {
    crate::serialization::aeciphertext_fromstr,
    serde::{Deserialize, Serialize},
};
use {
    crate::{
        check_program_account,
        error::TokenError,
        instruction::{encode_instruction, TokenInstruction},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        program_option::COption,
        pubkey::Pubkey,
    },
    spl_pod::{optional_keys::OptionalNonZeroPubkey, primitives::PodU64},
};

/// Rebasing Token extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum RebasingTokenInstruction {
    /// Initializes rebasing token for a mint.
    ///
    /// The `RebasingTokenInstruction::InitializeMint` instruction
    /// requires no signers and MUST be included within the same Transaction
    /// as `TokenInstruction::InitializeMint`. Otherwise another party can
    /// initialize the configuration.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeMint`
    /// instruction has already executed for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///
    /// Data expected by this instruction:
    ///   `InitializeRebasingMintData`
    InitializeMint,
    /// Transfers rebasing tokens from one account to another either directly
    /// or via a delegate.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The source account. Must include the
    ///      `TransferFeeAmount` extension.
    ///   1. `[]` The token mint. Must include the `TransferFeeConfig`
    ///      extension.
    ///   2. `[writable]` The destination account. Must include the
    ///      `TransferFeeAmount` extension.
    ///   3. `[signer]` The source account's owner/delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The source account.
    ///   1. `[]` The token mint.
    ///   2. `[writable]` The destination account.
    ///   3. `[]` The source account's multisignature owner/delegate.
    ///   4. ..4+M `[signer]` M signer accounts.
    TransferChecked,
}

/// Data expected by `RebasingTokenInstruction::InitializeMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeMintData {
    /// Authority to modify the share amount of each account
    pub share_authority: OptionalNonZeroPubkey,
}

/// Data expected by `RebasingTokenInstruction::TransferChecked`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct TransferCheckedData {
    /// The amount of tokens to transfer.
    pub share_amount: PodU64,
    /// Expected number of base 10 digits to the right of the decimal place.
    pub decimals: u8,
}

/// Create a `InitializeMint` instruction
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    share_authority: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::RebasingTokenExtension,
        RebasingTokenInstruction::InitializeMint,
        &InitializeMintData {
            share_authority: share_authority.try_into()?,
        },
    ))
}

/// Create a `TransferChecked` instruction
#[allow(clippy::too_many_arguments)]
pub fn transfer_checked(
    token_program_id: &Pubkey,
    source: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    signers: &[&Pubkey],
    share_amount: u64,
    decimals: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    let mut accounts = Vec::with_capacity(4 + signers.len());
    accounts.push(AccountMeta::new(*source, false));
    accounts.push(AccountMeta::new_readonly(*mint, false));
    accounts.push(AccountMeta::new(*destination, false));
    accounts.push(AccountMeta::new_readonly(*authority, signers.is_empty()));
    for signer in signers.iter() {
        accounts.push(AccountMeta::new_readonly(**signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::RebasingTokenExtension,
        RebasingTokenInstruction::TransferChecked,
        &TransferCheckedData {
            share_amount: share_amount.into(),
            decimals,
        },
    ))
}
