pub mod agents;
pub mod collections;
pub mod contracts;
pub mod lazy;
pub mod log;

pub use anyhow::{anyhow as new_error, ensure, Context, Error, Result};

pub mod serialize {
    pub use serde_json::from_slice;
    pub use serde_json::from_value;
    pub use serde_json::json;
    pub use serde_json::to_value;
    pub use serde_json::Error;
    pub use serde_json::Number;
    pub use serde_json::Value;
}

// Directly export macros, so that the user can write:
// #[borderless::contract], #[borderless::agent] and #[borderless::action]
pub use borderless_sdk_macros::{action, agent, contract, schedule, State};

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
    pub use crate::common::*;
    pub use crate::contracts::{ledger, TxCtx};
    pub use crate::events::*;
    pub use crate::serialize::json;
    /// Re-Export of `serde_json::json` macro as `value!`
    pub use crate::serialize::json as value;
    pub use crate::CallMethod;
    pub use crate::{ensure, new_error, Context, Error, Result};
    pub use borderless_sdk_macros::*;
}

pub mod common;
pub mod events;
pub mod http;

use crate::events::CBInit;

pub trait CallMethod: Sized + private_trait::Sealed {
    fn call_method(&self, method_name: &str) -> events::CallBuilder<CBInit>;
}

impl private_trait::Sealed for ContractId {}
impl CallMethod for ContractId {
    fn call_method(&self, method_name: &str) -> events::CallBuilder<CBInit> {
        events::CallBuilder::new(*self, method_name)
    }
}

impl private_trait::Sealed for events::Sink {}
impl CallMethod for events::Sink {
    fn call_method(&self, method_name: &str) -> events::CallBuilder<CBInit> {
        events::CallBuilder::new_with_writer(self.contract_id, method_name, &self.writer)
    }
}

impl private_trait::Sealed for AgentId {}
impl CallMethod for AgentId {
    fn call_method(&self, _method_name: &str) -> events::CallBuilder<CBInit> {
        todo!("Remove this")
    }
}

pub trait Participant: Sized + private_trait::Sealed {
    fn get_participant(elem: Self) -> Result<BorderlessId>;
}

impl private_trait::Sealed for BorderlessId {}
impl Participant for BorderlessId {
    fn get_participant(elem: Self) -> Result<BorderlessId> {
        contracts::env::participants()
            .into_iter()
            .find(|p| p.id == elem)
            .map(|p| p.id)
            .with_context(|| format!("Found no participant with id={elem}"))
    }
}

impl private_trait::Sealed for String {}
impl Participant for String {
    fn get_participant(elem: Self) -> Result<BorderlessId> {
        contracts::env::participant(elem)
    }
}

impl private_trait::Sealed for &str {}
impl Participant for &str {
    fn get_participant(elem: Self) -> Result<BorderlessId> {
        contracts::env::participant(elem)
    }
}

impl private_trait::Sealed for &common::Participant {}
impl Participant for &common::Participant {
    fn get_participant(elem: Self) -> Result<BorderlessId> {
        contracts::env::participant(&elem.alias)
    }
}

impl private_trait::Sealed for common::Participant {}
impl Participant for common::Participant {
    fn get_participant(elem: Self) -> Result<BorderlessId> {
        Participant::get_participant(&elem)
    }
}

mod private_trait {
    pub trait Sealed {}
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
