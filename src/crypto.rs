use rand::thread_rng;
use rand::Rng;
use crate::db::models::*;
use bcrypt::{hash, verify, BcryptResult};
use data_encoding::BASE32;
use oath::{totp_raw_now, HashType};

const TOTP_SECRET_LENGTH: usize = 64;
const TOTP_DIGITS: u32 = 8;
const TOTP_HASH: HashType = HashType::SHA1;
const TOTP_TIME_WINDOW: u64 = 30;

/// Generate new totp secret
pub fn totp_gen_secret() -> TOTP {
    let mut secret = [0u8; TOTP_SECRET_LENGTH];
    let mut rng = thread_rng();
    rng.fill(&mut secret);
    TOTP {
        secret: secret.to_vec(),
        mode: TOTP_HASH.into(),
        digits: TOTP_DIGITS,
    }
}

/// ASCII encode totp secret
pub fn totp_encode_secret(secret: &[u8]) -> String {
    BASE32.encode(secret)
}

/// Calculate TOTP answer based on secret
pub fn totp_calculate(totp: &TOTP) -> u64 {
    totp_raw_now(
        &totp.secret,
        totp.digits,
        0,
        TOTP_TIME_WINDOW,
        &totp.mode.as_HashType(),
    )
}

/// Hash password with bcrypt with given cost, blocking
///
/// Call with actix_threadpool inside actix async routines
pub fn bcrypt_password(password: &str, cost: u32) -> BcryptResult<String> {
    hash(password, cost)
}

/// verify password with bcrypt hash, blocking
///
/// Not to be used directly.
pub fn bcrypt_verify(password: &str, hash: &str) -> BcryptResult<bool> {
    let start = std::time::Instant::now();
    let res = verify(password, hash);
    debug!("Took {}ms to verify", start.elapsed().as_millis());
    res
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn totp_verify() {
        let totp = totp_gen_secret();
        let encoded = totp_encode_secret(&totp.secret);
        assert_eq!(totp.secret, BASE32.decode(encoded.as_bytes()).unwrap());
    }
}
