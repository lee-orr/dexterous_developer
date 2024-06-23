pub mod macros;

#[cfg(not(feature = "hot_internal"))]
mod cold;
#[cfg(feature = "hot_internal")]
mod hot;

#[cfg(not(feature = "hot_internal"))]
pub use cold::*;
#[cfg(feature = "hot_internal")]
pub use hot::*;

pub mod ffi {
    pub use rmp_serde::from_slice;
    pub use rmp_serde::to_vec;
    pub use safer_ffi::Vec;
    pub use abi_stable::DynTrait;
}

pub use anyhow::Result;
