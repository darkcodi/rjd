//! Diff module for comparing JSON values
//!
//! This module provides the core diff algorithm for comparing JSON structures.
//! It uses a recursive tree traversal approach to identify added, removed,
//! and modified values between two JSON documents.

mod engine;
mod visitor;

pub use engine::diff;
pub use visitor::traverse;
