#![allow(unused_variables, unused_mut, non_camel_case_types, non_snake_case)]

extern crate lcm;
#[macro_use]
extern crate lcm_derive;

include!(concat!(env!("OUT_DIR"), "/mod.rs"));

#[cfg(test)]
mod hashes;
