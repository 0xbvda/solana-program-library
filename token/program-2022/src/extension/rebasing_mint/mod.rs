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

        self._try_amount_to_ui_amount(amount, decimals, total_supply, rescale)
    }

    fn _try_amount_to_ui_amount(
        &self,
        amount: u64,
        decimals: u8,
        total_supply: u64,
        rebase_supply: u64,
    ) -> Result<String, ProgramError> {
        let scaled_amount = amount
            .checked_mul(rebase_supply)
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

        self._try_ui_amount_into_amount(rescaled_amount, decimals, total_supply, rescale)
    }

    fn _try_ui_amount_into_amount(
        &self,
        ui_amount: u64,
        decimals: u8,
        total_supply: u64,
        rebase_supply: u64,
    ) -> Result<u64, ProgramError> {
        let amount = ui_amount
            .checked_mul(total_supply)
            .ok_or(TokenError::Overflow)?
            .checked_div(rebase_supply)
            .ok_or(TokenError::Overflow)?;

        crate::try_ui_amount_into_amount(amount.to_string(), decimals)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn specific_amount_to_ui_amount() -> Result<(), ProgramError> {
        let config = RebasingMintConfig {
            rescale_authority: OptionalNonZeroPubkey::default(),
            rescale_type: 2,
        };

        let ui_amount = config._try_amount_to_ui_amount(10000, 0, 1000000, 500000)?;
        assert_eq!(ui_amount, "5000");
        let ui_amount = config._try_amount_to_ui_amount(20489552, 0, 6829850799, 1406623726)?;
        assert_eq!(ui_amount, "4219871");
        let ui_amount = config._try_amount_to_ui_amount(75029540, 0, 9872307908, 3494106051)?;
        assert_eq!(ui_amount, "26555205");
        let ui_amount = config._try_amount_to_ui_amount(42739323, 0, 9713482589, 9363004432)?;
        assert_eq!(ui_amount, "41197219");
        let ui_amount = config._try_amount_to_ui_amount(3786730, 0, 9466825946, 6164962985)?;
        assert_eq!(ui_amount, "2465984");
        let ui_amount = config._try_amount_to_ui_amount(13125741, 0, 9375529681, 597054347)?;
        assert_eq!(ui_amount, "835876");
        let ui_amount = config._try_amount_to_ui_amount(23561988, 0, 3318589989, 4289267781)?;
        assert_eq!(ui_amount, "30453800");
        let ui_amount = config._try_amount_to_ui_amount(59388358, 0, 7814257711, 366902980)?;
        assert_eq!(ui_amount, "2788462");
        let ui_amount = config._try_amount_to_ui_amount(66880085, 0, 7687366139, 7801353630)?;
        assert_eq!(ui_amount, "67871776");
        let ui_amount = config._try_amount_to_ui_amount(10973742, 0, 4389497196, 9830771087)?;
        assert_eq!(ui_amount, "24576925");
        let ui_amount = config._try_amount_to_ui_amount(35649, 0, 118830159, 295365455)?;
        assert_eq!(ui_amount, "88609");
        let ui_amount = config._try_amount_to_ui_amount(19443795, 0, 4226912171, 6953104185)?;
        assert_eq!(ui_amount, "31984277");
        let ui_amount = config._try_amount_to_ui_amount(851433, 0, 946037185, 2606963479)?;
        assert_eq!(ui_amount, "2346265");
        let ui_amount = config._try_amount_to_ui_amount(13416217, 0, 4192567826, 4266339254)?;
        assert_eq!(ui_amount, "13652285");
        let ui_amount = config._try_amount_to_ui_amount(30960754, 0, 6450157188, 5525341320)?;
        assert_eq!(ui_amount, "26521637");
        let ui_amount = config._try_amount_to_ui_amount(21974491, 0, 2712900214, 4089512207)?;
        assert_eq!(ui_amount, "33125047");
        let ui_amount = config._try_amount_to_ui_amount(503232, 0, 1258081029, 4898826356)?;
        assert_eq!(ui_amount, "1959528");

        Ok(())
    }

    #[test]
    fn specific_ui_amount_to_amount() -> Result<(), ProgramError> {
        let config = RebasingMintConfig {
            rescale_authority: OptionalNonZeroPubkey::default(),
            rescale_type: 2,
        };

        let ui_amount = config._try_ui_amount_into_amount(5000, 0, 1_000_000, 500_000)?;
        assert_eq!(ui_amount, 10000);
        let ui_amount = config._try_ui_amount_into_amount(4219871, 0, 6829850799, 1406623726)?;
        assert_eq!(ui_amount, 20489551);
        let ui_amount = config._try_ui_amount_into_amount(26555205, 0, 9872307908, 3494106051)?;
        assert_eq!(ui_amount, 75029537);
        let ui_amount = config._try_ui_amount_into_amount(41197219, 0, 9713482589, 9363004432)?;
        assert_eq!(ui_amount, 42739322);
        let ui_amount = config._try_ui_amount_into_amount(2465984, 0, 9466825946, 6164962985)?;
        assert_eq!(ui_amount, 3786728);
        let ui_amount = config._try_ui_amount_into_amount(835876, 0, 9375529681, 597054347)?;
        assert_eq!(ui_amount, 13125740);
        let ui_amount = config._try_ui_amount_into_amount(30453800, 0, 3318589989, 4289267781)?;
        assert_eq!(ui_amount, 23561987);
        let ui_amount = config._try_ui_amount_into_amount(2788462, 0, 7814257711, 366902980)?;
        assert_eq!(ui_amount, 59388344);
        let ui_amount = config._try_ui_amount_into_amount(67871776, 0, 7687366139, 7801353630)?;
        assert_eq!(ui_amount, 66880084);
        let ui_amount = config._try_ui_amount_into_amount(24576925, 0, 4389497196, 9830771087)?;
        assert_eq!(ui_amount, 10973741);
        let ui_amount = config._try_ui_amount_into_amount(88609, 0, 118830159, 295365455)?;
        assert_eq!(ui_amount, 35648);
        let ui_amount = config._try_ui_amount_into_amount(31984277, 0, 4226912171, 6953104185)?;
        assert_eq!(ui_amount, 19443794);
        let ui_amount = config._try_ui_amount_into_amount(2346265, 0, 946037185, 2606963479)?;
        assert_eq!(ui_amount, 851432);
        let ui_amount = config._try_ui_amount_into_amount(13652285, 0, 4192567826, 4266339254)?;
        assert_eq!(ui_amount, 13416216);
        let ui_amount = config._try_ui_amount_into_amount(26521637, 0, 6450157188, 5525341320)?;
        assert_eq!(ui_amount, 30960752);
        let ui_amount = config._try_ui_amount_into_amount(33125047, 0, 2712900214, 4089512207)?;
        assert_eq!(ui_amount, 21974490);
        let ui_amount = config._try_ui_amount_into_amount(1959528, 0, 1258081029, 4898826356)?;
        assert_eq!(ui_amount, 503231);

        Ok(())
    }
}
