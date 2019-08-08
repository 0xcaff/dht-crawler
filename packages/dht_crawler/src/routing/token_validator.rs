use crypto::{
    digest::Digest,
    sha1::Sha1,
};
use krpc_encoding as proto;
use rand;
use std::net::SocketAddrV4;

/// Generates and validates tokens. A token generated with
/// [`TokenValidator::generate_token`] is valid until
/// [`TokenValidator::rotate_tokens`] is called twice.
///
/// ```
/// # use std::net::SocketAddrV4;
/// # use dht_crawler::routing::TokenValidator;
/// let mut validator = TokenValidator::new();
/// let addr: SocketAddrV4 = "129.21.63.170:34238".parse().unwrap();
///
/// let token = validator.generate_token(&addr);
/// assert_eq!(true, validator.verify_token(&addr, &token));
///
/// validator.rotate_tokens();
/// assert_eq!(true, validator.verify_token(&addr, &token));
///
/// validator.rotate_tokens();
/// assert_eq!(false, validator.verify_token(&addr, &token));
/// ```
pub struct TokenValidator {
    /// Secret used when generating tokens for `get_peers` and `announce_peer`.
    token_secret: [u8; 4],

    /// Last secret. Tokens generated with this secret are also valid.
    last_token_secret: [u8; 4],
}

impl TokenValidator {
    pub fn new() -> TokenValidator {
        TokenValidator {
            token_secret: rand::random(),
            last_token_secret: rand::random(),
        }
    }

    /// Generates a token for `addr`. This token will be valid
    pub fn generate_token(&self, addr: &SocketAddrV4) -> [u8; 20] {
        generate_token(addr, &self.token_secret)
    }

    pub fn verify_token(&self, addr: &SocketAddrV4, token: &[u8]) -> bool {
        // This is vulnerable to a side-channel attack.
        generate_token(addr, &self.token_secret) == token
            || generate_token(addr, &self.last_token_secret) == token
    }

    pub fn rotate_tokens(&mut self) {
        let new_secret: [u8; 4] = rand::random();
        self.last_token_secret = self.token_secret;
        self.token_secret = new_secret;
    }
}

/// Generates a token given an address and secret.
fn generate_token(addr: &SocketAddrV4, secret: &[u8; 4]) -> [u8; 20] {
    let mut hasher = Sha1::new();

    let addr_bytes = proto::addr_to_bytes(addr);

    hasher.input(&addr_bytes);
    hasher.input(secret);

    let mut output = [0u8; 20];
    hasher.result(&mut output);

    output
}
