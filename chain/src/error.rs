use thiserror::Error;

/// Reasons a transaction or block is rejected.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ChainError {
    #[error("double spend")]
    DoubleSpend,
    #[error("bad signature")]
    BadSignature,
    #[error("bad range proof")]
    BadRangeProof,
    #[error("unbalanced transaction")]
    Unbalanced,
    #[error("unknown output in ring")]
    UnknownRingMember,
    #[error("malformed point or scalar")]
    Malformed,
    #[error("invalid ring")]
    BadRing,
    #[error("not enough decoy outputs in the ledger")]
    NotEnoughDecoys,
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("empty transaction")]
    Empty,
    #[error("bad block: {0}")]
    BadBlock(&'static str),
    #[error("encoding: {0}")]
    Encoding(String),
}
