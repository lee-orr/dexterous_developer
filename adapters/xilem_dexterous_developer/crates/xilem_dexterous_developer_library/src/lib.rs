pub mod macros;
pub mod types;
#[cfg(feature = "hot_internal")]
mod hot;
#[cfg(not(feature = "hot_internal"))]
mod cold;

#[cfg(feature = "hot_internal")]
pub use hot::*;
#[cfg(not(feature = "hot_internal"))]
pub use cold::*;