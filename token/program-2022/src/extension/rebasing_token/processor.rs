use super::{instruction::RebasingTokenInstruction, RebasingTokenMint};
use crate::{
    check_program_account,
    extension::{
        rebasing_token::instruction::InitializeMintData, BaseStateWithExtensionsMut,
        PodStateWithExtensionsMut,
    },
    instruction::{decode_instruction_data, decode_instruction_type},
    pod::PodMint,
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
    }
}
