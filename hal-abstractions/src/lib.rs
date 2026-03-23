//! Hardware abstraction traits for IoT firmware
//!
//! This crate defines traits that abstract over hardware differences
//! between boards. BSPs implement these traits.

#![cfg_attr(not(test), no_std)]
#![deny(unsafe_code)]
#![deny(warnings)]

pub mod sensor;

// Traits will be added in Batch 7
// pub mod rtc;
// pub mod rng;
// pub mod network;
