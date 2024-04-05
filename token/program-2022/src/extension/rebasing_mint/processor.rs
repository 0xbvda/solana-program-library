use super::{instruction::RebasingMintInstruction, RebaseType, RebasingMintConfig};
use crate::{
    check_program_account,
    error::TokenError,
    extension::{
        rebasing_mint::instruction::InitializeInstructionData, BaseStateWithExtensionsMut,
        PodStateWithExtensions, PodStateWithExtensionsMut,
    },
    instruction::{decode_instruction_data, decode_instruction_type},
    pod::PodMint,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh1::try_from_slice_unchecked,
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
    stake::state::StakeStateV2,
};
use spl_pod::optional_keys::OptionalNonZeroPubkey;

/// Processes an [Initialize] rebasing extension instruction.
pub fn process_initialize(
    accounts: &[AccountInfo],
    rebase_pubkey: &OptionalNonZeroPubkey,
    rebase_type: &u8,
) -> ProgramResult {
    let rebase_type_: RebaseType = (*rebase_type).try_into()?;

    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;
    let rebase_info = next_account_info(account_info_iter)?;

    let acc_match = Some(*rebase_info.key) == Option::from(*rebase_pubkey);

    if !acc_match {
        return Err(TokenError::RebaseAccountMismatch.into());
    }

    // TODO: Assert type
    match rebase_type_ {
        RebaseType::Staking => {
            let _stake_state =
                try_from_slice_unchecked::<StakeStateV2>(&rebase_info.data.borrow())?;
        }
        RebaseType::Mint => {
            let data = rebase_info.data.borrow();
            let _mint_state = PodStateWithExtensions::<PodMint>::unpack(&data)
                .map_err(|_| Into::<ProgramError>::into(TokenError::InvalidMint))?;
        }
    };

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let rebasing_token_mint = mint.init_extension::<RebasingMintConfig>(true)?;

    rebasing_token_mint.rescale_authority = *rebase_pubkey;

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    check_program_account(program_id)?;

    match decode_instruction_type(input)? {
        RebasingMintInstruction::Initialize => {
            msg!("RebasingMintInstruction::Initialize");
            let InitializeInstructionData {
                rebase_pubkey,
                rebase_type,
            } = decode_instruction_data(input)?;

            process_initialize(accounts, rebase_pubkey, rebase_type)
        }
    }
}
