use super::{instruction::RebasingTokenInstruction, RebasingTokenMint};
use crate::{
    check_program_account,
    error::TokenError,
    extension::{
        rebasing_token::{
            instruction::{InitializeMintData, TransferCheckedData},
            RebasingTokenAccount,
        },
        BaseStateWithExtensionsMut, PodStateWithExtensionsMut,
    },
    instruction::{decode_instruction_data, decode_instruction_type},
    pod::{PodAccount, PodMint},
    processor::Processor,
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_option::COption,
    pubkey::Pubkey,
};

/// Processes an [InitializeMint] instruction.
pub fn process_initialize_mint(
    accounts: &[AccountInfo],
    share_authority: COption<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let mint_info = next_account_info(account_info_iter)?;

    check_program_account(mint_info.owner)?;
    let mint_data = &mut mint_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack_uninitialized(mint_data)?;
    let rebasing_token_mint = mint.init_extension::<RebasingTokenMint>(true)?;

    rebasing_token_mint.share_authority = share_authority.try_into()?;

    Ok(())
}

/// Processes an [TransferChecked] instruction.
pub fn process_transfer_checked(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    share_amount: u64,
    decimals: Option<u8>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let source_account_info = next_account_info(account_info_iter)?;
    let mint_account_info = next_account_info(account_info_iter)?;

    let mut mint_data = mint_account_info.data.borrow_mut();
    let mut mint = PodStateWithExtensionsMut::<PodMint>::unpack(&mut mint_data)?;
    let token_supply = mint.base.supply;

    let mint_extension = mint.get_extension_mut::<RebasingTokenMint>()?;

    let mut token_data = source_account_info.data.borrow_mut();
    let mut token_acc = PodStateWithExtensionsMut::<PodAccount>::unpack(&mut token_data)?;
    let token_acc_extension = token_acc.get_extension_mut::<RebasingTokenAccount>()?;

    // Assert that are there sufficient shares
    if token_acc_extension.shares < share_amount {
        return Err(TokenError::InsufficientFunds.into());
    }

    // Subtract transfered shared amount from token_acc_extension.shares
    token_acc_extension.shares = token_acc_extension
        .shares
        .checked_sub(share_amount)
        .ok_or(TokenError::Overflow)?
        .into();

    let share_supply = mint_extension.supply;

    let transfer_amount = share_amount
        .checked_mul(token_supply.into())
        .ok_or(TokenError::Overflow)?
        .checked_div(share_supply)
        .ok_or(TokenError::Overflow)?;

    Processor::process_transfer(program_id, accounts, transfer_amount, decimals, None)?;

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
        RebasingTokenInstruction::InitializeMint => {
            msg!("RebasingTokenInstruction::InitializeMint");
            let data = decode_instruction_data::<InitializeMintData>(input)?;
            process_initialize_mint(accounts, data.share_authority.into())
        }
        RebasingTokenInstruction::TransferChecked => {
            msg!("RebasingTokenInstruction::TransferChecked");
            let data = decode_instruction_data::<TransferCheckedData>(input)?;
            process_transfer_checked(
                program_id,
                accounts,
                data.share_amount.into(),
                data.decimals.into(),
            )
        }
    }
}
