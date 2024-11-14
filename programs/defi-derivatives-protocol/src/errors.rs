use anchor_lang::prelude::*;

/// Custom errors for the protocol
#[error_code]
pub enum ProtocolError {
    #[msg("Option has already been exercised.")]
    OptionAlreadyExercised,
    #[msg("Option has expired.")]
    OptionExpired,
    #[msg("Invalid expiration time.")]
    InvalidExpiration,
    #[msg("Insufficient funds.")]
    InsufficientFunds,
    // Add more errors as needed
}
