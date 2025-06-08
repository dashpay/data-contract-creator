use super::{Index, Property};
use serde::{Deserialize, Serialize};

/// Represents a document type in a Dash Platform data contract
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct DocumentType {
    pub name: String,
    pub properties: Vec<Property>,
    pub indices: Vec<Index>,
    pub required: Vec<String>,
    pub created_at_required: bool,
    pub updated_at_required: bool,
    pub additionalProperties: bool,
    pub comment: String,
    pub description: String,
    pub keywords: String,
}

impl Default for DocumentType {
    fn default() -> Self {
        Self {
            name: String::new(),
            properties: Vec::new(),
            indices: Vec::new(),
            required: Vec::new(),
            created_at_required: false,
            updated_at_required: false,
            additionalProperties: false,
            comment: String::new(),
            description: String::new(),
            keywords: String::new(),
        }
    }
}

impl DocumentType {
    /// Creates a new document type with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    /// Adds a property to this document type
    pub fn add_property(&mut self, property: Property) {
        self.properties.push(property);
    }

    /// Removes a property at the given index
    pub fn remove_property(&mut self, index: usize) -> Option<Property> {
        if index < self.properties.len() {
            Some(self.properties.remove(index))
        } else {
            None
        }
    }

    /// Adds an index to this document type
    pub fn add_index(&mut self, index: Index) {
        self.indices.push(index);
    }

    /// Removes an index at the given position
    pub fn remove_index(&mut self, index: usize) -> Option<Index> {
        if index < self.indices.len() {
            Some(self.indices.remove(index))
        } else {
            None
        }
    }

    /// Updates the required properties list based on current properties
    pub fn update_required_properties(&mut self) {
        self.required = self
            .properties
            .iter()
            .filter(|prop| prop.required)
            .map(|prop| prop.name.clone())
            .collect();
    }
}
