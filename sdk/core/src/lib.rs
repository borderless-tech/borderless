pub mod agents;
pub mod collections;
pub mod contracts;
pub mod lazy;
pub mod log;

pub use anyhow::{anyhow as new_error, ensure, Context, Error, Result};

pub mod serialize {
    pub use serde_json::from_slice;
    pub use serde_json::from_value;
    pub use serde_json::to_value;
    pub use serde_json::Error;
    pub use serde_json::Value;
}

// Directly export macros, so that the user can write:
// #[borderless::contract], #[borderless::agent] and #[borderless::action]
pub use borderless_sdk_macros::{action, agent, contract, schedule, NamedSink, State};

/// This module is **not** part of the public API.
/// It exists, because the procedural macros and some internal implementations (like the contract runtime) rely on it.
///
/// You probably don't want to use this directly.
#[doc(hidden)]
#[path = "private.rs"]
pub mod __private;

// Re-export all id-types at top-level
pub use borderless_id_types::*;

// Re-export entire hash crate
pub use borderless_hash as hash;

// Re-export pkg crate
pub use borderless_pkg as pkg;

pub mod prelude {
    pub use crate::agents::*;
    pub use crate::common::*;
    pub use crate::contracts::*;
    pub use crate::events::*;
    pub use crate::{ensure, new_error, Context, Error, Result};
    pub use borderless_sdk_macros::*;
}

pub mod common;
pub mod events;
pub mod http;

/// Trait that must be implemented on the `Sink` enum inside a contract module.
///
/// Implementing this trait ensures, that you can split the sink into a static string
/// (which represents the 'alias'-string we use to match sinks) and a CallAction object,
/// which will be used to generate the output transaction.
pub trait NamedSink {
    /// Splits the sink into its alias and the encoded CallAction object.
    ///
    /// Errors while converting the action should be converted into a wasm trap.
    fn into_action(self) -> (&'static str, events::CallAction);
}

pub mod time {
    #[cfg(target_arch = "wasm32")]
    use borderless_abi as abi;

    use std::{
        ops::{Add, AddAssign, Sub, SubAssign},
        time::Duration,
    };

    // Very simple re-implementation of the SystemTime API
    #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct SystemTime(i64);

    pub fn timestamp() -> i64 {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            abi::timestamp()
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("ts < unix-epoch")
                .as_millis() as i64
        }
    }

    impl SystemTime {
        pub fn now() -> Self {
            Self(timestamp())
        }

        pub fn duration_since(&self, earlier: SystemTime) -> Result<Duration, Duration> {
            let diff = self
                .0
                .checked_sub(earlier.0)
                .ok_or_else(|| Duration::from_millis((earlier.0 - self.0) as u64))?;
            Ok(Duration::from_millis(diff as u64))
        }

        pub fn elapsed(&self) -> Duration {
            let diff = SystemTime::now().0 - self.0;
            Duration::from_millis(diff as u64)
        }
    }

    impl Add<Duration> for SystemTime {
        type Output = SystemTime;

        /// # Panics
        ///
        /// This function may panic if the resulting point in time cannot be represented by the underlying data structure
        fn add(self, dur: Duration) -> SystemTime {
            SystemTime(self.0 + dur.as_millis() as i64)
        }
    }

    impl AddAssign<Duration> for SystemTime {
        fn add_assign(&mut self, other: Duration) {
            *self = *self + other;
        }
    }

    impl Sub<Duration> for SystemTime {
        type Output = SystemTime;

        fn sub(self, dur: Duration) -> SystemTime {
            SystemTime(self.0 - dur.as_millis() as i64)
        }
    }

    impl SubAssign<Duration> for SystemTime {
        fn sub_assign(&mut self, other: Duration) {
            *self = *self - other;
        }
    }
}
