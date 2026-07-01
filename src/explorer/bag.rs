//! # Resource bag
//!
//! Stores resources collected by Jebediah during exploration.

use crate::BagSummary;

use common_game::components::resource::{
    BasicResource,
    BasicResourceType,
    ComplexResource,
    ComplexResourceType,
    ResourceType,
};

use std::collections::HashMap;

pub struct JebBag {
    basic: HashMap<BasicResourceType, Vec<BasicResource>>,
    complex: HashMap<ComplexResourceType, Vec<ComplexResource>>,
}

impl JebBag {
    pub fn new() -> Self {
        Self {
            basic: HashMap::new(),
            complex: HashMap::new(),
        }
    }

    pub fn add_basic(&mut self, resource: BasicResource) {
        let kind = resource.get_type();
        self.basic.entry(kind).or_default().push(resource);
    }

    pub fn add_complex(&mut self, resource: ComplexResource) {
        let kind = resource.get_type();
        self.complex.entry(kind).or_default().push(resource);
    }

    pub fn count_basic(&self, kind: BasicResourceType) -> usize {
        self.basic.get(&kind).map(|v| v.len()).unwrap_or(0)
    }

    pub fn count_complex(&self, kind: ComplexResourceType) -> usize {
        self.complex.get(&kind).map(|v| v.len()).unwrap_or(0)
    }

    pub fn summarize(&self) -> BagSummary {
        let mut out = Vec::new();

        for (kind, resources) in &self.basic {
            if !resources.is_empty() {
                out.push((ResourceType::Basic(*kind), resources.len()));
            }
        }

        for (kind, resources) in &self.complex {
            if !resources.is_empty() {
                out.push((ResourceType::Complex(*kind), resources.len()));
            }
        }

        out
    }
}

impl Default for JebBag {
    fn default() -> Self {
        Self::new()
    }
}