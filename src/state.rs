use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    secp256k1_recover::{Secp256k1Pubkey},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use borsh::{BorshSerialize, BorshDeserialize};
use solana_program::keccak::hash;
use crate::error::BridgeError;

/// ====== INCOGNITO PDA BURNID =======
#[derive(Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct PDABurnId {
    pub is_initialized: bool,
}

impl PDABurnId {
    pub const LEN: usize = 1;
}

impl IsInitialized for PDABurnId {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

/// ====== INCOGNITO PROXY =======
/// 
/// Max number of beacon addresses
pub const MAX_BEACON_ADDRESSES: usize = 20;
pub const ADDRESS_LENGTH: usize = 20;

// Incognito proxy stores beacon list
#[derive(Clone, Default, PartialEq)]
pub struct IncognitoProxy {
    /// init beacon
    pub is_initialized: bool,
    /// bump seed
    pub bump_seed: u8,
    /// beacon list
    pub beacons: Vec<[u8; 20]>,
    /// beacon height
    pub height: u64,
    /// prev beacon height
    pub previous_height: u64,
    /// next beacon height
    pub next_height: u64,
}

impl IsInitialized for IncognitoProxy {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IncognitoProxy {
    /// Create a new lending market
    pub fn new(params: IncognitoProxy) -> Self {
        let mut incognito_proxy = Self::default();
        Self::init(&mut incognito_proxy, params);
        incognito_proxy
    }

    /// Initialize a lending market
    pub fn init(&mut self, params: IncognitoProxy) {
        self.is_initialized = params.is_initialized;
        self.bump_seed = params.bump_seed;
        self.beacons = params.beacons;
        self.height = params.height;
        self.previous_height = params.previous_height;
        self.next_height = params.next_height;
    }
}

impl Sealed for IncognitoProxy {}

impl Pack for IncognitoProxy {
    /// 1 + 1 + 1 + 64 * 20 + 8 + 8 + 8
    const LEN: usize = 1307;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, IncognitoProxy::LEN];
        let (
            is_initialized,
            bump_seed,
            beacon_len,
            data_flat,
            height,
            previous_height,
            next_height
        ) = array_refs![
            src, 
            1,
            1,
            1, 
            ADDRESS_LENGTH * MAX_BEACON_ADDRESSES,
            8,
            8,
            8
        ];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(BridgeError::InvalidBoolValue.into()),
        };
        let beacon_len = u8::from_le_bytes(*beacon_len);
        let mut beacons = Vec::with_capacity(beacon_len as usize + 1);
        let mut offset = 0;
        for _ in 0..beacon_len {
            let beacon_flat = array_ref![data_flat, offset, ADDRESS_LENGTH];
            beacons.push(*beacon_flat);
            offset += ADDRESS_LENGTH;
        }
        let height: u64 = u64::from_le_bytes(*height)?;
        let previous_height: u64 = u64::from_le_bytes(*previous_height)?;
        let next_height: u64 = u64::from_le_bytes(*next_height)?;
        
        Ok(IncognitoProxy {
            is_initialized,
            bump_seed: u8::from_le_bytes(*bump_seed),
            beacons,
            height,
            previous_height,
            next_height,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, IncognitoProxy::LEN];
        let (
            is_initialized,
            bump_seed,
            beacon_len,
            data_flat
        ) = mut_array_refs![
            dst, 
            1, 
            1,
            1, 
            ADDRESS_LENGTH * MAX_BEACON_ADDRESSES
        ];
        *beacon_len = u8::try_from(self.beacons.len()).unwrap().to_le_bytes();
        *bump_seed = self.bump_seed.to_le_bytes();
        pack_bool(self.is_initialized, is_initialized);

        let mut offset = 0;
        // beacons
        for beacon in &self.beacons {
            let beacon_flat = array_mut_ref![data_flat, offset, ADDRESS_LENGTH];
            #[allow(clippy::ptr_offset_with_cast)]
            beacon_flat.copy_from_slice(&beacon.to_bytes());
            offset += ADDRESS_LENGTH;
        }

    }

}

// Dapp interaction
#[derive(Clone, Default)]
pub struct DappRequest {
    // instruction
    pub inst: Vec<u8>,
    // number of accounts
    pub num_acc: u8,
    // sign acc index
    pub sign_index: u8,
}

/// Reserve liquidity
#[derive(Clone, Debug, PartialEq)]
pub struct BeaconRequests {
    // instruction in bytes
    pub inst: [u8; 162],
    // beacon height
    pub height: u64,
    // inst paths to build merkle tree
    pub inst_paths: Vec<[u8; 32]>,
    // inst path indicator
    pub inst_path_is_lefts: Vec<bool>,
    // instruction root
    pub inst_root: [u8; 32],
    // blkData
    pub blk_data: [u8; 32],
    // signature index
    pub indexes: Vec<u8>,
    // signature 
    pub signatures: Vec<[u8; 65]>
}

fn pack_bool(boolean: bool, dst: &mut [u8; 1]) {
    *dst = (boolean as u8).to_le_bytes()
}