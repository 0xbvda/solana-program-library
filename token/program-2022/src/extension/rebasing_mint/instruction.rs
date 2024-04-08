use {
    super::RebaseType,
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
    std::convert::TryInto,
};
#[cfg(feature = "serde-traits")]
use {
    crate::serialization::aeciphertext_fromstr,
    serde::{Deserialize, Serialize},
};

/// Rebasing Token extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum RebasingMintInstruction {
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
    Initialize,
}

/// Data expected by `RebasingTokenInstruction::InitializeMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeInstructionData {
    /// Authority to modify the share amount of each account
    pub rebase_pubkey: OptionalNonZeroPubkey,
    /// u8 bit flag indicating the type of rescale
    /// 1 --> staking account
    /// 2 --> mint account
    pub rebase_type: u8,
}

/// Create a `Initialize` rebasing extension instruction
pub fn initialize(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    rebase_pubkey: Option<Pubkey>,
    rebase_type: u8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    RebaseType::validate(rebase_type)?;

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::RebasingTokenExtension,
        RebasingMintInstruction::Initialize,
        &InitializeInstructionData {
            rebase_pubkey: rebase_pubkey.try_into()?,
            rebase_type,
        },
    ))
}
