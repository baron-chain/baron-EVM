use crate::{utilities::right_pad, Error, Precompile, PrecompileResult, PrecompileWithAddress};
use bcevm_primitives::{alloy_primitives::B512, Bytes, B256};

pub const ECRECOVER: PrecompileWithAddress = PrecompileWithAddress(
    crate::u64_to_address(1),
    Precompile::Standard(ec_recover_run),
);

pub use self::secp256k1::ecrecover;

#[cfg(not(feature = "secp256k1"))]
mod secp256k1 {
    use k256::ecdsa::{Error, RecoveryId, Signature, VerifyingKey};
    use bcevm_primitives::{alloy_primitives::B512, keccak256, B256};

    pub fn ecrecover(sig: &B512, mut recid: u8, msg: &B256) -> Result<B256, Error> {
        let mut sig = Signature::from_slice(sig.as_slice())?;
        if let Some(sig_normalized) = sig.normalize_s() {
            sig = sig_normalized;
            recid ^= 1;
        }
        let recid = RecoveryId::from_byte(recid).expect("recovery ID is valid");
        let recovered_key = VerifyingKey::recover_from_prehash(&msg[..], &sig, recid)?;
        let mut hash = keccak256(&recovered_key.to_encoded_point(false).as_bytes()[1..]);
        hash[..12].fill(0);
        Ok(hash)
    }
}

#[cfg(feature = "secp256k1")]
mod secp256k1 {
    use bcevm_primitives::{alloy_primitives::B512, keccak256, B256};
    use secp256k1::{ecdsa::{RecoverableSignature, RecoveryId}, Message, Secp256k1};
    use k256 as _;

    pub fn ecrecover(sig: &B512, recid: u8, msg: &B256) -> Result<B256, secp256k1::Error> {
        let recid = RecoveryId::from_i32(recid as i32).expect("recovery ID is valid");
        let sig = RecoverableSignature::from_compact(sig.as_slice(), recid)?;
        let secp = Secp256k1::new();
        let msg = Message::from_digest(msg.0);
        let public = secp.recover_ecdsa(&msg, &sig)?;
        let mut hash = keccak256(&public.serialize_uncompressed()[1..]);
        hash[..12].fill(0);
        Ok(hash)
    }
}

pub fn ec_recover_run(input: &Bytes, gas_limit: u64) -> PrecompileResult {
    const ECRECOVER_BASE: u64 = 3_000;
    if ECRECOVER_BASE > gas_limit {
        return Err(Error::OutOfGas);
    }
    let input = right_pad::<128>(input);
    if !(input[32..63].iter().all(|&b| b == 0) && matches!(input[63], 27 | 28)) {
        return Ok((ECRECOVER_BASE, Bytes::new()));
    }
    let msg = <&B256>::try_from(&input[0..32]).unwrap();
    let recid = input[63] - 27;
    let sig = <&B512>::try_from(&input[64..128]).unwrap();
    let out = secp256k1::ecrecover(sig, recid, msg)
        .map(|o| o.to_vec().into())
        .unwrap_or_default();
    Ok((ECRECOVER_BASE, out))
}
