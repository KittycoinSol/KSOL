use solana_program::pubkey::Pubkey;
use borsh::{BorshDeserialize, BorshSerialize};


#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct GreetingAccount {
    pub counter: u32,
    pub stats: [u8; 5]
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Stake {
    pub is_initialized: bool,
    pub owner: Pubkey,
    pub amount: u64,
    pub time_started: i64
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct RewardsPool {
    pub is_initialized: bool,
    pub amount: u64,
    pub airdrop_supply: u64,
    pub airdrop_fee: u64,
    pub total_coins_staked: u64,
    pub total_stakes_count: u32
}

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct Ballot {
    pub is_initialized: bool,
    pub choices: [u32;5]
}
