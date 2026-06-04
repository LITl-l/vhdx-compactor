//! Portable, OS-independent logic for the WSL2 VHDX compactor.
//!
//! Everything in this crate is unit-testable on any host (no Windows APIs, no
//! GUI), which is what lets the project be developed and tested on Linux while
//! the binary itself only runs on Windows.

pub mod model;
pub mod parse;
