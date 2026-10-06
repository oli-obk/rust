#[cfg(feature = "nightly")]
use rustc_macros::{Decodable_NoContext, Encodable_NoContext, StableHash_NoContext};

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
#[cfg_attr(
    feature = "nightly",
    derive(StableHash_NoContext, Decodable_NoContext, Encodable_NoContext)
)]
pub enum CoherenceDomain {
    /// Only look at upstream crate things
    Upstream,
    /// Normal compilation before the existence of expansion time macros
    Everything,
}
