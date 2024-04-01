#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    spl_pod::optional_keys::OptionalNonZeroPubkey,
};

/// Transfer fee extension instructions
pub mod instruction;

/// Transfer fee extension processor
pub mod processor;

/// Rebasing Token Mint state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RebasingTokenMint {
    /// The authority to determine share allocations.
    pub share_authority: OptionalNonZeroPubkey,
}

/// Rebasing Token Account state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct RebasingTokenAccount {
    /// Shares to compute rebasing token balance
    pub shares: u64,
}

impl Extension for RebasingTokenMint {
    const TYPE: ExtensionType = ExtensionType::RebasingTokenMint;
}
