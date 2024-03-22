#[cfg(any(feature = "hot", feature = "hot_internal"))]
pub mod internal_shared;

#[cfg(feature = "hot_internal")]
pub mod hot_internal;
