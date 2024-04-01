#[cfg(feature = "serde-traits")]
use {
    crate::serialization::aeciphertext_fromstr,
    serde::{Deserialize, Serialize},
};
use {
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
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
}

/// Data expected by `ConfidentialTransferInstruction::Transfer`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeMintData {
    /// Authority to modify the share amount of each account
    pub share_authority: OptionalNonZeroPubkey,
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
