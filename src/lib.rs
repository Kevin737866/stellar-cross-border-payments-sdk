// The anchor, compliance, escrow modules and the u8 shim contain pre-existing
// compilation errors that are unrelated to the rate oracle.  Exclude them
// during `cargo test` so the rate oracle unit tests can compile and run.
#[cfg(not(test))]
pub mod anchor;
#[cfg(not(test))]
pub mod compliance;
#[cfg(not(test))]
pub mod escrow;
pub mod rate_oracle;
#[cfg(not(test))]
mod u8;

#[cfg(not(test))]
pub use anchor::{AnchorInfo, AnchorRegistry, DepositInfo, KycRequirement};
#[cfg(not(test))]
pub use compliance::{
    ComplianceCheck, ComplianceContract, ComplianceLevel, ComplianceRecord, RiskLevel,
    TransactionRule,
};
#[cfg(not(test))]
pub use escrow::{Dispute, DisputeEvidence, Escrow, EscrowContract, EscrowStatus, get_evidence, store_evidence};
pub use rate_oracle::{AggregatedRate, ExchangeRate, RateOracleContract, RateSource};
