use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

use crate::instruction::Instruction;
use crate::state::{Ballot, RewardsPool, Stake};

entrypoint!(process_instruction);

const KITTYCOIN_MINT: [u8; 32] = [
    83, 253, 12, 237, 188, 72, 195, 47, 36, 136, 47, 129, 204, 109, 25, 144, 91, 81, 3, 78, 116,
    125, 93, 233, 32, 239, 68, 27, 127, 118, 163, 167,
];
const ADMIN_ADDRESS: [u8; 32] = [
    252, 183, 216, 215, 153, 134, 231, 182, 0, 96, 138, 106, 16, 14, 99, 194, 5, 112, 181, 170,
    137, 219, 8, 176, 131, 117, 4, 201, 41, 234, 154, 123,
];
const BASE_COIN: u64 = 1000000;
const COINS_FOR_1_INTEREST: u64 = 2160000; //Number of base coins to stake to earn 1 interest per minute.
const VOTING_FEE: u64 = 1000 * BASE_COIN;
const DEFUALT_AIRDROP_COST: u64 = solana_program::native_token::LAMPORTS_PER_SOL / 10000;
const AIRDROP_MAX: u64 = 200_000 * BASE_COIN;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("PROGRAM CALL");

    msg!("{:?}", instruction_data);

    let instruction = Instruction::unpack(instruction_data)?;

    match instruction {
        Instruction::CreateStake { amount } => {
            msg!("Instruction: CreateStake");
            process_create_stake(program_id, accounts, amount)
        }
        Instruction::EndStake => {
            msg!("Instruction: EndStake");
            process_end_stake(program_id, accounts)
        }
        Instruction::AddToRewardsPool { amount, airdrop } => {
            msg!("Instruction: AddToRewardsPool");
            process_add_to_rewards_pool(program_id, accounts, amount, airdrop)
        }
        Instruction::Vote { selection } => {
            msg!("Instruction: Vote");
            process_vote(program_id, accounts, selection)
        }
        Instruction::Airdrop { amount } => {
            msg!("Instruction: Airdrop");
            process_airdrop(program_id, accounts, amount)
        }
        Instruction::ChangeAirdropFee { fee } => {
            msg!("Instruction: ChangeAidropFee");
            process_change_airdrop_fee(program_id, accounts, fee)
        }
    }
}

///[0] owner
///[1] owner token
///[2] stake
///[3] pda token
///[4] token program
///[5] rewards account
fn process_create_stake(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let owner_account = next_account_info(accounts_iter)?;
    let owner_token_account = next_account_info(accounts_iter)?;
    let stake_account = next_account_info(accounts_iter)?;
    let pda_token_account = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let rewards_account = next_account_info(accounts_iter)?;

    let (pda, _bump_seed) = Pubkey::find_program_address(&[], program_id);

    if amount < BASE_COIN {
        msg!("Minimum 1 Kittycoin stake.");
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut stake_info = Stake::try_from_slice(&stake_account.data.borrow())?;
    if stake_info.is_initialized {
        return Err(ProgramError::AccountAlreadyInitialized);
    }
    stake_info.is_initialized = true;
    stake_info.owner = *owner_account.key;
    stake_info.amount = amount;
    stake_info.time_started = Clock::get()?.unix_timestamp;

    let mint = Pubkey::new_from_array(KITTYCOIN_MINT);
    let associated_pda_account =
        spl_associated_token_account::get_associated_token_address(&pda, &mint);

    if associated_pda_account != *pda_token_account.key {
        msg!(
            "{:?} not equal to {:?}",
            associated_pda_account,
            *pda_token_account.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    msg!("Staking {} Kittycoin.", amount);

    let ix = spl_token::instruction::transfer(
        token_program.key,
        owner_token_account.key,
        pda_token_account.key,
        owner_account.key,
        &[&owner_account.key],
        amount,
    )?;

    invoke(
        &ix,
        &[
            owner_token_account.clone(),
            pda_token_account.clone(),
            owner_account.clone(),
            token_program.clone(),
        ],
    )?;


    let mut rewards_info = RewardsPool::try_from_slice(&rewards_account.data.borrow())?;
    if !rewards_info.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    rewards_info.total_stakes_count += 1;
    rewards_info.total_coins_staked += amount;

    rewards_info.serialize(&mut &mut rewards_account.data.borrow_mut()[..])?;
    stake_info.serialize(&mut &mut stake_account.data.borrow_mut()[..])?;

    Ok(())
}

///[0] owner
///[1] owner token
///[2] stake
///[3] pda
///[4] pda token
///[5] token program
///[6] rewards
fn process_end_stake(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let owner_account = next_account_info(accounts_iter)?;
    let owner_token_account = next_account_info(accounts_iter)?;
    let stake_account = next_account_info(accounts_iter)?;
    let pda_account = next_account_info(accounts_iter)?;
    let pda_token_account = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let rewards_account = next_account_info(accounts_iter)?;

    let (pda, bump_seed) = Pubkey::find_program_address(&[], program_id);

    let mut rewards_info = RewardsPool::try_from_slice(&rewards_account.data.borrow())?;
    if !rewards_info.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    if rewards_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let mut stake_info = Stake::try_from_slice(&stake_account.data.borrow())?;
    if !stake_info.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    if !owner_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if stake_info.owner != *owner_account.key {
        return Err(ProgramError::IllegalOwner);
    }
    let mint = Pubkey::new_from_array(KITTYCOIN_MINT);
    let associated_pda_account =
        spl_associated_token_account::get_associated_token_address(&pda, &mint);

    if associated_pda_account != *pda_token_account.key {
        msg!(
            "{:?} not equal to {:?}",
            associated_pda_account,
            *pda_token_account.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let current_time = Clock::get()?.unix_timestamp;
    if current_time < stake_info.time_started {
        return Err(ProgramError::InvalidInstructionData);
    }
    let time_elapsed = current_time - stake_info.time_started;
    let minutes_elapsed: u64 = (time_elapsed / 60) as u64;
    msg!("MINUTES ELAPSED: {}", minutes_elapsed);

    let reward = stake_info.amount / COINS_FOR_1_INTEREST * minutes_elapsed;

    let mut payout = stake_info.amount;
    if rewards_info.amount >= reward {
        payout += reward;
        rewards_info.amount -= reward;
    }
    rewards_info.total_stakes_count -= 1;
    rewards_info.total_coins_staked -= stake_info.amount;

    let ix = spl_token::instruction::transfer(
        token_program.key,
        pda_token_account.key,
        owner_token_account.key,
        pda_account.key,
        &[&pda_account.key],
        payout,
    )?;

    invoke_signed(
        &ix,
        &[
            pda_token_account.clone(),
            owner_token_account.clone(),
            pda_account.clone(),
            token_program.clone(),
        ],
        &[&[&[], &[bump_seed]]],
    )?;

    msg!("Closing the account...{}", stake_info.time_started);
    **owner_account.lamports.borrow_mut() = owner_account
        .lamports()
        .checked_add(stake_account.lamports())
        .ok_or(ProgramError::InsufficientFunds)?;
    **stake_account.lamports.borrow_mut() = 0;

    stake_info.is_initialized = false;
    rewards_info.serialize(&mut &mut rewards_account.data.borrow_mut()[..])?;
    stake_info.serialize(&mut &mut stake_account.data.borrow_mut()[..])?;

    msg!("Stake ended. Coins received: {}", payout);
    msg!("Coins remaining in reward pool {}", rewards_info.amount);
    Ok(())
}

///[0] donator
///[1] donator token
///[2] pda token
///[3] token program
///[4] rewards
fn process_add_to_rewards_pool(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
    airdrop: bool,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let admin_account = next_account_info(accounts_iter)?;
    let admin_token_account = next_account_info(accounts_iter)?;
    let pda_token_account = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let rewards_account = next_account_info(accounts_iter)?;

    let (pda, _bump_seed) = Pubkey::find_program_address(&[], program_id);

    let mut rewards_info = RewardsPool::try_from_slice(&rewards_account.data.borrow())?;
    if !admin_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    if !rewards_info.is_initialized {
        if *admin_account.key != Pubkey::new_from_array(ADMIN_ADDRESS) {
            return Err(ProgramError::InvalidInstructionData);
        } else {
            rewards_info.is_initialized = true;
            rewards_info.airdrop_fee = DEFUALT_AIRDROP_COST;
        }
    }

    let mint = Pubkey::new_from_array(KITTYCOIN_MINT);
    let associated_pda_account =
        spl_associated_token_account::get_associated_token_address(&pda, &mint);

    if associated_pda_account != *pda_token_account.key {
        msg!(
            "{:?} not equal to {:?}",
            associated_pda_account,
            *pda_token_account.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let ix = spl_token::instruction::transfer(
        token_program.key,
        admin_token_account.key,
        pda_token_account.key,
        admin_account.key,
        &[&admin_account.key],
        amount,
    )?;

    invoke(
        &ix,
        &[
            admin_token_account.clone(),
            pda_token_account.clone(),
            admin_account.clone(),
            token_program.clone(),
        ],
    )?;
    if airdrop {
        rewards_info.airdrop_supply += amount;
    } else {
        rewards_info.amount += amount;
    }
    rewards_info.serialize(&mut &mut rewards_account.data.borrow_mut()[..])?;
    Ok(())
}

///[0] owner
///[1] owner token
///[2] ballot
///[3] pda token
///[4] token program
///[5] rewards
///[6] rewards
fn process_vote(program_id: &Pubkey, accounts: &[AccountInfo], selection: u8) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let owner_account = next_account_info(accounts_iter)?;
    let owner_token_account = next_account_info(accounts_iter)?;
    let ballot_account = next_account_info(accounts_iter)?;
    let pda_token_account = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let rewards_account = next_account_info(accounts_iter)?;

    let (pda, _bump_seed) = Pubkey::find_program_address(&[], program_id);

    let mut ballot_info = Ballot::try_from_slice(&ballot_account.data.borrow())?;
    if !ballot_info.is_initialized {
        if *owner_account.key == Pubkey::new_from_array(ADMIN_ADDRESS) && owner_account.is_signer {
            //The admin can start a new ballot.
            ballot_info.is_initialized = true;
        } else {
            return Err(ProgramError::UninitializedAccount);
        }
    }

    let mut rewards_info = RewardsPool::try_from_slice(&rewards_account.data.borrow())?;
    if !rewards_info.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }

    let mint = Pubkey::new_from_array(KITTYCOIN_MINT);
    let associated_pda_account =
        spl_associated_token_account::get_associated_token_address(&pda, &mint);

    if associated_pda_account != *pda_token_account.key {
        msg!(
            "{:?} not equal to {:?}",
            associated_pda_account,
            *pda_token_account.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let ix = spl_token::instruction::transfer(
        token_program.key,
        owner_token_account.key,
        pda_token_account.key,
        owner_account.key,
        &[&owner_account.key],
        VOTING_FEE,
    )?;

    invoke(
        &ix,
        &[
            owner_token_account.clone(),
            pda_token_account.clone(),
            owner_account.clone(),
            token_program.clone(),
        ],
    )?;

    ballot_info.choices[selection as usize] += 1;
    rewards_info.amount += VOTING_FEE;

    ballot_info.serialize(&mut &mut ballot_account.data.borrow_mut()[..])?;
    rewards_info.serialize(&mut &mut rewards_account.data.borrow_mut()[..])?;
    Ok(())
}

///[0] owner
///[1] owner token
///[2] pda
///[3] pda token
///[4] token program
///[5] rewards
///[6] treasury
fn process_airdrop(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let owner_account = next_account_info(accounts_iter)?;
    let owner_token_account = next_account_info(accounts_iter)?;
    let pda_account = next_account_info(accounts_iter)?;
    let pda_token_account = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let rewards_account = next_account_info(accounts_iter)?;
    let treasury_account = next_account_info(accounts_iter)?;

    let (pda, bump_seed) = Pubkey::find_program_address(&[], program_id);

    if *treasury_account.key != Pubkey::new_from_array(ADMIN_ADDRESS) {
        msg!("Incorrect treasury address!");
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut rewards_info = RewardsPool::try_from_slice(&rewards_account.data.borrow())?;
    if !rewards_info.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    if rewards_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    let mint = Pubkey::new_from_array(KITTYCOIN_MINT);
    let associated_pda_account =
        spl_associated_token_account::get_associated_token_address(&pda, &mint);

    if associated_pda_account != *pda_token_account.key {
        msg!(
            "{:?} not equal to {:?}",
            associated_pda_account,
            *pda_token_account.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let ix = spl_token::instruction::transfer(
        token_program.key,
        pda_token_account.key,
        owner_token_account.key,
        pda_account.key,
        &[&pda_account.key],
        amount,
    )?;

    invoke_signed(
        &ix,
        &[
            pda_token_account.clone(),
            owner_token_account.clone(),
            pda_account.clone(),
            token_program.clone(),
        ],
        &[&[&[], &[bump_seed]]],
    )?;

    **owner_account.lamports.borrow_mut() = owner_account
        .lamports()
        .checked_sub(rewards_info.airdrop_fee)
        .ok_or(ProgramError::InsufficientFunds)?;
    **treasury_account.lamports.borrow_mut() = treasury_account
        .lamports()
        .checked_add(rewards_info.airdrop_fee)
        .ok_or(ProgramError::InsufficientFunds)?;

    if amount > AIRDROP_MAX {
        return Err(ProgramError::InvalidInstructionData);
    }
    if rewards_info.airdrop_supply < amount {
        return Err(ProgramError::InsufficientFunds);
    }
    rewards_info.airdrop_supply -= amount;
    rewards_info.serialize(&mut &mut rewards_account.data.borrow_mut()[..])?;

    msg!("Coins airdropped {}", amount);
    msg!(
        "Coins remaining in airdrop pool {}",
        rewards_info.airdrop_supply
    );
    Ok(())
}

///[0] admin
///[1] rewards
fn process_change_airdrop_fee(program_id: &Pubkey, accounts: &[AccountInfo], fee: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let admin_account = next_account_info(accounts_iter)?;
    let rewards_account = next_account_info(accounts_iter)?;

    if *admin_account.key != Pubkey::new_from_array(ADMIN_ADDRESS) || !admin_account.is_signer {
        msg!("Incorrect admin address!");
        return Err(ProgramError::InvalidInstructionData);
    }

    let mut rewards_info = RewardsPool::try_from_slice(&rewards_account.data.borrow())?;
    if !rewards_info.is_initialized {
        return Err(ProgramError::UninitializedAccount);
    }
    if rewards_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    rewards_info.airdrop_fee = fee;
    rewards_info.serialize(&mut &mut rewards_account.data.borrow_mut()[..])?;

    msg!("Airdrop fee changed to {} lamports", fee);

    Ok(())
}
