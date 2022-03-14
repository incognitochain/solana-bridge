use solana_program::{
    program_error::ProgramError,
    msg,
    pubkey::{Pubkey, PUBKEY_BYTES},
    secp256k1_recover::{Secp256k1Pubkey},
    instruction::{AccountMeta, Instruction},
};
use crate::error::BridgeError::{
    InvalidInstruction,
    InstructionUnpackError
};
use crate::state::{
    UnshieldRequest,
    IncognitoProxy,
    DappRequest,
};
use std::{convert::TryInto, mem::size_of};
use crate::error::BridgeError;

pub enum BridgeInstruction {

    ///// shield
    Shield {
        /// shield info
        amount: u64,
        inc_address: [u8; 148],
    },

    ///// unshield
    UnShield {
        /// unshield info
        unshield_info: UnshieldRequest,
    },

    ///// init beacon info
    InitBeacon {
        /// beacon info
        init_beacon_info: IncognitoProxy,
    },

    ///// dapp interaction
    DappInteraction {
        /// beacon info
        dapp_request: DappRequest,
    },

    ///// withdraw request
    WithdrawRequest {
        /// withdraw request
        amount: u64,
        inc_address: [u8; 148],
    },
}

impl BridgeInstruction {
    /// Unpacks a byte buffer into a [BridgeInstruction](enum.BridgeInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 | 4 => {
                let (amount, rest) = Self::unpack_u64(rest)?;
                let (inc_address, _) = Self::unpack_bytes148(rest)?;
                if *tag == 0 {
                    Self::Shield {
                        amount,
                        inc_address: inc_address.clone()
                    }
                } else {
                    Self::WithdrawRequest {
                        amount,
                        inc_address: inc_address.clone()
                    }
                }
            },
            1 => {
                let (inst, rest) =  Self::unpack_bytes162(rest)?;
                let (height, rest) = Self::unpack_u64(rest)?;
                let (inst_paths_len,mut rest) = Self::unpack_u8(rest)?;
                let mut inst_paths = Vec::with_capacity(inst_paths_len as usize + 1);
                for _ in 0..inst_paths_len {
                    let (inst_node, rest_) = Self::unpack_bytes32(rest)?;
                    rest = rest_;
                    inst_paths.push(*inst_node);
                }
                let (inst_paths_is_left_len,mut rest) = Self::unpack_u8(rest)?;
                let mut inst_path_is_lefts = Vec::with_capacity(inst_paths_is_left_len as usize + 1);
                for _ in 0..inst_paths_is_left_len {
                    let (inst_paths_is_left, rest_) = Self::unpack_bool(rest)?;
                    rest = rest_;
                    inst_path_is_lefts.push(inst_paths_is_left);
                }
                let (inst_root, rest) = Self::unpack_bytes32(rest)?;
                let (blk_data, rest) = Self::unpack_bytes32(rest)?;
                let (indexes_len,mut rest) = Self::unpack_u8(rest)?;
                let mut indexes = Vec::with_capacity(indexes_len as usize + 1);
                for _ in 0..indexes_len {
                    let (index, rest_) = Self::unpack_u8(rest)?;
                    rest = rest_;
                    indexes.push(index);
                }
                let (signature_len,mut rest) = Self::unpack_u8(rest)?;
                let mut signatures = Vec::with_capacity(signature_len as usize + 1);
                for _ in 0..signature_len {
                    let (signature, rest_) = Self::unpack_bytes65(rest)?;
                    rest = rest_;
                    signatures.push(*signature);
                }

                let incognito_proof = UnshieldRequest {
                    inst: *inst,
                    height,
                    inst_paths,
                    inst_path_is_lefts,
                    inst_root: *inst_root,
                    blk_data: *blk_data,
                    indexes,
                    signatures,
                };

                Self::UnShield {
                    unshield_info: incognito_proof,
                }
            },
            2 => {
                let (vault_key, rest) = Self::unpack_pubkey(rest)?;
                let (bump_seed, rest) = Self::unpack_u8(rest)?;
                let (beacon_list_len, mut rest) =  Self::unpack_u8(rest)?;
                let mut beacons = Vec::with_capacity(beacon_list_len as usize + 1);
                for _ in 0..beacon_list_len {
                    let (beacon, rest_) = Self::unpack_bytes64(rest)?;
                    rest = rest_;
                    let new_beacon = Secp256k1Pubkey::new(beacon);
                    beacons.push(new_beacon);
                }
                Self::InitBeacon {
                    init_beacon_info: IncognitoProxy{
                        is_initialized: true,
                        bump_seed,
                        vault: vault_key,
                        beacons
                    }   
                }
            },
            3 => {
                let (inst_len, rest) = Self::unpack_u8(rest)?; // todo upgrade inst length
                let (inst_data, rest) = Self::unpack_nbytes(rest, inst_len)?;
                let (acc_len, rest) = Self::unpack_u8(rest)?;
                let (sign_index, _) = Self::unpack_u8(rest)?;
                Self::DappInteraction {
                    dapp_request: DappRequest {
                        inst: inst_data.to_vec(),
                        num_acc: acc_len,
                        sign_index
                    }
                }
            }
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_u64(input: &[u8]) -> Result<(u64, &[u8]), ProgramError> {
        if input.len() < 8 {
            msg!("u64 cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(8);
        let value = bytes
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InstructionUnpackError)?;
        Ok((value, rest))
    }

    fn unpack_bytes162(input: &[u8]) -> Result<(&[u8; 162], &[u8]), ProgramError> {
        if input.len() < 162 {
            msg!("162 bytes cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(162);
        Ok((
            bytes
                .try_into()
                .map_err(|_| InstructionUnpackError)?,
            rest,
        ))
    }

    fn unpack_bytes148(input: &[u8]) -> Result<(&[u8; 148], &[u8]), ProgramError> {
        if input.len() < 148 {
            msg!("148 bytes cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(148);
        Ok((
            bytes
                .try_into()
                .map_err(|_| InstructionUnpackError)?,
            rest,
        ))
    }

    fn unpack_u8(input: &[u8]) -> Result<(u8, &[u8]), ProgramError> {
        if input.is_empty() {
            msg!("u8 cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(1);
        let value = bytes
            .get(..1)
            .and_then(|slice| slice.try_into().ok())
            .map(u8::from_le_bytes)
            .ok_or(InstructionUnpackError)?;
        Ok((value, rest))
    }

    fn unpack_bytes32(input: &[u8]) -> Result<(&[u8; 32], &[u8]), ProgramError> {
        if input.len() < 32 {
            msg!("32 bytes cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(32);
        Ok((
            bytes
                .try_into()
                .map_err(|_| InstructionUnpackError)?,
            rest,
        ))
    }

    fn unpack_bytes65(input: &[u8]) -> Result<(&[u8; 65], &[u8]), ProgramError> {
        if input.len() < 65 {
            msg!("65 bytes cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(65);
        Ok((
            bytes
                .try_into()
                .map_err(|_| InstructionUnpackError)?,
            rest,
        ))
    }

    fn unpack_bytes64(input: &[u8]) -> Result<(&[u8; 64], &[u8]), ProgramError> {
        if input.len() < 64 {
            msg!("64 bytes cannot be unpacked {:?}", input);
            return Err(InstructionUnpackError.into());
        }
        let (bytes, rest) = input.split_at(64);
        Ok((
            bytes
                .try_into()
                .map_err(|_| InstructionUnpackError)?,
            rest,
        ))
    }

    fn unpack_bool(input: &[u8]) -> Result<(bool, &[u8]), ProgramError> {
        let (value, rest) = Self::unpack_u8(input)?;

        match value {
            0 => Ok((false, rest)),
            1 => Ok((false, rest)),
            _ => {
                msg!("Boolean cannot be unpacked");
                Err(BridgeError::InvalidBoolValue.into())
            }
        }
    }

    fn unpack_pubkey(input: &[u8]) -> Result<(Pubkey, &[u8]), ProgramError> {
        if input.len() < PUBKEY_BYTES {
            msg!("Pubkey cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (key, rest) = input.split_at(PUBKEY_BYTES);
        let pk = Pubkey::new(key);
        Ok((pk, rest))
    }

    fn unpack_nbytes(input: &[u8], n: u8) -> Result<(&[u8], &[u8]), ProgramError> {
        if input.len() < n as usize {
            msg!("Pubkey cannot be unpacked");
            return Err(InstructionUnpackError.into());
        }
        let (key, rest) = input.split_at(n as usize);
        Ok((key, rest))
    }

    pub fn pack(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(size_of::<Self>());
        match *self {
            Self::Shield {
                amount,
                inc_address,
            } => {
                buf.push(0);
                buf.extend_from_slice(&amount.to_le_bytes());
                buf.extend_from_slice(inc_address.as_ref());
            }
            // todo: implement unshield and init bridge
            _ => {

            }
        }
        buf
    }
}

/// Creates a 'Shield' instruction.
#[allow(clippy::too_many_arguments)]
pub fn shield(
    program_id: Pubkey,
    amount: u64,
    shield_maker_token_account: Pubkey,
    vault_token_account: Pubkey,
    incoginto_proxy: Pubkey,
    shield_maker_authority: Pubkey,
    inc_address: &[u8; 148],
) -> Instruction {
    Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(shield_maker_token_account, false),
            AccountMeta::new(vault_token_account, false),
            AccountMeta::new_readonly(incoginto_proxy, false),
            AccountMeta::new_readonly(shield_maker_authority, true),
            AccountMeta::new_readonly(spl_token::id(), false),
        ],
        data: BridgeInstruction::Shield { amount, inc_address: inc_address.clone() }.pack(),
    }
}