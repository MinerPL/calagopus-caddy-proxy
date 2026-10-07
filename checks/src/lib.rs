//! Compiles the extension modules that do not depend on the Panel's `shared` crate.
#![allow(dead_code)]

#[path = "../../src/domain.rs"]
mod domain;

#[path = "../../src/labels.rs"]
mod labels;
