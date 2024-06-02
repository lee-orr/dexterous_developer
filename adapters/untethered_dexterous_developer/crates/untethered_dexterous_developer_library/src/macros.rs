pub use paste::paste;

#[cfg(feature = "hot_internal")]
mod hot {
    #[macro_export]
    macro_rules! reloadable_main {
        ($f:ident) => {
            
        }
    }
}