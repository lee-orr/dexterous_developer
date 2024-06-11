pub mod macros;

#[cfg(feature = "hot_internal")]
mod hot;
#[cfg(not(feature = "hot_internal"))]
mod cold;

#[cfg(feature = "hot_internal")]
pub use hot::*;
#[cfg(not(feature = "hot_internal"))]
pub use cold::*;

pub mod ffi {
    pub use safer_ffi::Vec;
    pub use rmp_serde::to_vec;
    pub use rmp_serde::from_slice;
}

pub use anyhow::Result;
