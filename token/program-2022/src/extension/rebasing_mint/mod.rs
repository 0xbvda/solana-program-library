use crate::{error::TokenError, pod::PodMint};
#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};

use super::PodStateWithExtensions;

use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    solana_program::{
        account_info::AccountInfo, borsh1::try_from_slice_unchecked, msg,
        program_error::ProgramError, stake::state::StakeStateV2,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

/// Transfer fee extension instructions
pub mod instruction;

/// Transfer fee extension processor
pub mod processor;

/// Rebasing Token Mint state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RebasingMintConfig {
    /// The authority to determine supply rescaling.
    pub rescale_authority: OptionalNonZeroPubkey,
    /// u8 bit flag indicating the type of rescale
    /// 1 --> staking account
    /// 2 --> mint account
    pub rescale_type: u8,
}

impl Extension for RebasingMintConfig {
    const TYPE: ExtensionType = ExtensionType::RebasingMintConfig;
}

/// Enum to flag the type of account of the rescale authority
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(C)]
pub enum RebaseType {
    /// Flags the rescale account as being a staking account
    Staking,
    /// Flags the rescale account as being a mint account
    Mint,
}

impl TryInto<RebaseType> for u8 {
    type Error = ProgramError;

    fn try_into(self) -> Result<RebaseType, Self::Error> {
        match self {
            1 => Ok(RebaseType::Staking),
            2 => Ok(RebaseType::Mint),
            _ => {
                msg!("Error: Invalid value, `rescale_type` must be either 1 or 2");
                return Err(ProgramError::InvalidArgument);
            }
        }
    }
}

impl RebaseType {
    /// Ensures that the u8 bit flag is either 1 or 2
    /// corresponding to the RescaleType variants
    pub fn validate(val: u8) -> Result<(), ProgramError> {
        match val {
            1 | 2 => Ok(()),
            _ => {
                msg!("Error: Invalid value, `rescale_type` must be either 1 or 2");
                return Err(ProgramError::InvalidArgument);
            }
        }
    }
}

/// Gets the amount staked from a stake account's state
pub fn get_stake(stake_state: &StakeStateV2) -> Result<u64, ProgramError> {
    match stake_state {
        StakeStateV2::Uninitialized => {
            msg!("Error: Stake account cannot be in Uninitialized mode, must be in Stake mode");
            return Err(ProgramError::InvalidAccountData);
        }
        StakeStateV2::Initialized(_meta) => {
            msg!("Error: Stake account cannot be in Initialized mode, must be in Stake mode");
            return Err(ProgramError::InvalidAccountData);
        }
        StakeStateV2::Stake(_meta, stake, _stake_flags) => Ok(stake.delegation.stake),
        StakeStateV2::RewardsPool => {
            msg!("Error: Stake account cannot be in Rewards mode, must be in Stake mode");
            return Err(ProgramError::InvalidAccountData);
        }
    }
}

impl RebasingMintConfig {
    /// Checks that the rebase account corresponds to the one mentionde in the
    /// rebasing config extension
    pub fn check_rebase_account(&self, rebase_info: &AccountInfo) -> Result<(), ProgramError> {
        let acc_match = Some(*rebase_info.key) == Option::from(self.rescale_authority);

        if !acc_match {
            return Err(TokenError::RebaseAccountMismatch.into());
        }

        Ok(())
    }
    /// Convert a raw amount to its UI representation using the given decimals
    /// field Excess zeroes or unneeded decimal point are trimmed.
    pub fn try_amount_to_ui_amount(
        &self,
        amount: u64,
        decimals: u8,
        total_supply: u64,
        rebase_info: &AccountInfo,
    ) -> Result<String, ProgramError> {
        let rescale_type = self.rescale_type.try_into()?;
        let rescale = match rescale_type {
            RebaseType::Staking => {
                let stake_state =
                    try_from_slice_unchecked::<StakeStateV2>(&rebase_info.data.borrow())?;

                get_stake(&stake_state)?
            }
            RebaseType::Mint => {
                let mint_data = rebase_info.data.borrow();
                let rescale_mint = PodStateWithExtensions::<PodMint>::unpack(&mint_data)
                    .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;

                rescale_mint.base.supply.into()
            }
        };

        let scaled_amount = amount
            .checked_mul(rescale)
            .ok_or(TokenError::Overflow)?
            .checked_div(total_supply)
            .ok_or(TokenError::Overflow)?;

        Ok(crate::amount_to_ui_amount_string_trimmed(
            scaled_amount,
            decimals,
        ))
    }

    /// Try to convert a UI representation of a token amount to its raw amount
    /// using the given decimals field
    pub fn try_ui_amount_into_amount(
        &self,
        ui_amount: &str,
        decimals: u8,
        total_supply: u64,
        rebase_info: &AccountInfo,
    ) -> Result<u64, ProgramError> {
        let rescaled_amount = ui_amount
            .parse::<u64>()
            .map_err(|_| ProgramError::InvalidArgument)?;

        let rescale_type = self.rescale_type.try_into()?;
        let rescale = match rescale_type {
            RebaseType::Staking => {
                let stake_state =
                    try_from_slice_unchecked::<StakeStateV2>(&rebase_info.data.borrow())?;

                get_stake(&stake_state)?
            }
            RebaseType::Mint => {
                let mint_data = rebase_info.data.borrow();

                let rescale_mint = PodStateWithExtensions::<PodMint>::unpack(&mint_data)
                    .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;

                rescale_mint.base.supply.into()
            }
        };

        let amount = rescaled_amount
            .checked_mul(total_supply)
            .ok_or(TokenError::Overflow)?
            .checked_div(rescale)
            .ok_or(TokenError::Overflow)?;

        crate::try_ui_amount_into_amount(amount.to_string(), decimals)
    }
}
