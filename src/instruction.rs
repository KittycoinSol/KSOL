use solana_program::{program_error::ProgramError};

pub enum Instruction {
    CreateStake {amount: u64},
    EndStake,
    AddToRewardsPool {amount: u64, airdrop: bool},
    Vote {selection: u8},
    Airdrop {amount: u64},
    ChangeAirdropFee {fee: u64},
}

impl Instruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (&tag, rest) = input.split_first().ok_or(ProgramError::InvalidInstructionData)?;
        Ok(match tag {
            0 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::CreateStake {amount}
            }
            1 => {
                Self::EndStake
            }
            2 => {
                let (amount, rest) = Self::unpack_u64(rest)?;
                let (airdrop, _rest) = Self::unpack_bool(rest)?;
                Self::AddToRewardsPool {amount, airdrop}
            }
            3 => {
                let (selection, _rest) = Self::unpack_u8(rest)?;
                Self::Vote {selection}
            }
            4 => {
                let (amount, _rest) = Self::unpack_u64(rest)?;
                Self::Airdrop {amount}
            }
            5 => {
                let (fee, _rest) = Self::unpack_u64(rest)?;
                Self::ChangeAirdropFee {fee}
            }
            _ => {
                return Err(ProgramError::InvalidInstructionData);
            }
        })
    }

    fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
        if input.len() >= 1 {
            let (uint, rest) = input.split_at(1);
            Ok((uint[0], rest))
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    // fn unpack_u8_array(input: &[u8]) -> Result<[u8; 5], ProgramError> {
    //     if input.len() >= 5 {
    //         let mut result: [u8; 5] = [0; 5];
    //         for i in 0..5 {
    //             let (arr, rest) = input.split_at(1);
    //             result[i] = u8::from_le_bytes([arr[0]]);
    //         }
    //         Ok(result)
    //     } else {
    //         msg!("{}", input.len());
    //         Err(ProgramError::InvalidInstructionData)
    //     }
    // }

    // fn unpack_u16(input: &[u8]) -> Result<(u16, &[u8]), ProgramError> {
    //     if input.len() >= 2 {
    //         let (uint, rest) = input.split_at(2);
    //         Ok((u16::from_be_bytes([uint[0], uint[1]]), rest))
    //     } else {
    //         Err(ProgramError::InvalidInstructionData)
    //     }
    // }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() >= 8 {
            let (uint, rest) = input.split_at(8);
            Ok((
                u64::from_le_bytes([
                    uint[0], uint[1], uint[2], uint[3], uint[4], uint[5], uint[6], uint[7],
                ]),
                rest,
            ))
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    // fn unpack_u128(input: &[u8]) -> Result<(u128, &[u8]), ProgramError> {
    //     if input.len() >= 16 {
    //         let (uint, rest) = input.split_at(16);
    //         Ok((
    //             u128::from_be_bytes([
    //                 uint[0], uint[1], uint[2], uint[3], uint[4], uint[5], uint[6], uint[7],
    //                 uint[8], uint[9], uint[10], uint[11], uint[12], uint[13], uint[14], uint[15],
    //             ]),
    //             rest,
    //         ))
    //     } else {
    //         Err(ProgramError::InvalidInstructionData)
    //     }
    // }

    fn unpack_bool(input: &[u8]) -> Result<(bool, &[u8]), ProgramError> {
        if input.len() >= 1 {
            let (uint, rest) = input.split_at(1);
            Ok((uint[0] == 1, rest))
        } else {
            Err(ProgramError::InvalidInstructionData)
        }
    }

    // fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
    //     if input.len() >= 32 {
    //         let (key, rest) = input.split_at(32);
    //         let pk = Pubkey::new(key);
    //         Ok((pk, rest))
    //     } else {
    //         Err(ProgramError::InvalidInstructionData)
    //     }
    // }
}
