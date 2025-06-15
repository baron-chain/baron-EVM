#![warn(rustdoc::all, unreachable_pub)]
#![allow(rustdoc::bare_urls)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![deny(unused_must_use, rust_2018_idioms)]
#![cfg_attr(not(feature = "std"), no_std)]

#[macro_use]
#[cfg(not(feature = "std"))]
extern crate alloc as std;

mod builder;
mod context;
mod db;
mod evm;
mod frame;
mod handler;
mod inspector;
mod journaled_state;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

#[cfg(feature = "optimism")]
pub mod optimism;

pub use builder::EvmBuilder;
pub use context::*;
pub use db::*;
pub use evm::{Evm, CALL_STACK_LIMIT};
pub use frame::*;
pub use handler::Handler;
pub use inspector::*;
pub use journaled_state::*;

#[cfg(feature = "optimism")]
pub use optimism::{L1BlockInfo, BASE_FEE_RECIPIENT, L1_BLOCK_CONTRACT, L1_FEE_RECIPIENT};

pub use bcevm_interpreter as interpreter;
pub use bcevm_interpreter::primitives;
pub use bcevm_precompile as precompile;
