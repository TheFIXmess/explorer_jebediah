//! # Resource bag
//!
//! Stores resources collected by Jebediah during exploration.
//!
//! TODO(Vivi): implement this module.
//! - See explorer_viviana's bag.rs for the structure to mirror
//! - `summarize()` must return `Vec<(ResourceType, usize)>` to match the protocol generic
//! - Wire the bag into ai.rs wherever `TODO(Vivi): store in bag` appears

use crate::BagSummary;

pub struct JebBag {}

impl JebBag {
    pub fn new() -> Self {
        Self {}
    }

    pub fn summarize(&self) -> BagSummary {
        // TODO(Vivi): return actual contents
        vec![]
    }
}
