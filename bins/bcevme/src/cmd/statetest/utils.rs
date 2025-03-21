use k256::ecdsa::SigningKey;
use bcevm::primitives::Address;

pub fn recover_address(private_key: &[u8]) -> Option<Address> {
    SigningKey::from_slice(private_key)
        .ok()
        .map(|key| Address::from_raw_public_key(&key.verifying_key().to_encoded_point(false).as_bytes()[1..]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bcevm::primitives::{address, hex};

    #[test]
    fn sanity_test() {
        assert_eq!(
            Some(address!("a94f5374fce5edbc8e2a8697c15331677e6ebf0b")),
            recover_address(&hex!("45a915e4d060149eb4365960e6a7a45f334393093061116b197e3240065ff2d8"))
        );
    }
}
