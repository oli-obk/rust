use rustc_macros::StableHash;

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq, StableHash)]
pub enum CoherenceDomain {
    /// Only look at upstream crate things
    Upstream,
    /// Normal compilation before the existence of expansion time macros
    Everything,
}
