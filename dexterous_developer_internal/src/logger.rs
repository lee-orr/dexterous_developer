#[cfg(feature = "hot")]
pub use log::{debug, error, info, trace, warn};

#[cfg(not(feature = "hot"))]
pub use crate::{debug, error, info, trace, warn};

mod print_logger {
    #[macro_export]
    macro_rules! info {
        ($($x:expr),+) => (println!($($x),+))
    }
    #[macro_export]
    macro_rules! trace {
        ($($x:expr),+) => (println!($($x),+))
    }
    #[macro_export]
    macro_rules! debug {
        ($($x:expr),+) => (println!($($x),+))
    }
    #[macro_export]
    macro_rules! warn {
        ($($x:expr),+) => (eprintln!($($x),+))
    }
    #[macro_export]
    macro_rules! error {
        ($($x:expr),+) => (eprintln!($($x),+))
    }
}
