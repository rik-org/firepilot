#![doc = include_str!("../../README.md")]
#![forbid(unsafe_code)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(unreachable_pub)]

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;
extern crate url;

#[cfg(doctest)]
mod doctests {
    doc_comment::doctest!("../../README.md");
}

pub mod builder;
pub mod executor;
pub mod machine;
