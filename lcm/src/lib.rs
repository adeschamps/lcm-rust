//! From the [LCM Homepage](http://lcm-proj.github.io/):
//! >
//! LCM is a set of libraries and tools for message passing and data marshalling,
//! targeted at real-time systems where high-bandwidth and low latency are critical.
//! It provides a publish/subscribe message passing model
//! and automatic marshalling/unmarshalling code generation
//! with bindings for applications in a variety of programming languages.
//!
//! This crate provides a Rust implementation of the LCM protocol and code generator.
//! See also the `lcm-gen` crate for generating message types from a specification file.

// Re-export the `lcm-derive` crate for ease of use. I am not sure if being
// able to do this without `#![feature(use_extern_macros)]` is a bug or not.
#[allow(unused_imports)]
#[macro_use]
extern crate lcm_derive;
#[doc(hidden)]
pub use lcm_derive::*;

#[macro_use]
extern crate log;

extern crate byteorder;
#[macro_use]
extern crate failure;
extern crate net2;
extern crate regex;

mod utils;

pub mod error;

mod lcm;
pub use lcm::{Lcm, Subscription};

mod message;
pub use message::{Marshall, Message};
