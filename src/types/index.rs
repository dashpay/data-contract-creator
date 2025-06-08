use serde::{Deserialize, Serialize};

/// Index properties for database optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexProperties(pub String, pub String);

impl Default for IndexProperties {
    fn default() -> Self {
        Self("".to_string(), "asc".to_string())
    }
}

impl IndexProperties {
    /// Creates a new index property with the given field name and sort order
    pub fn new(field: String, order: String) -> Self {
        Self(field, order)
    }

    /// Returns the field name
    pub fn field(&self) -> &str {
        &self.0
    }

    /// Returns the sort order
    pub fn order(&self) -> &str {
        &self.1
    }

    /// Sets the field name
    pub fn set_field(&mut self, field: String) {
        self.0 = field;
    }

    /// Sets the sort order (should always be "asc" for Dash Platform)
    pub fn set_order(&mut self, order: String) {
        self.1 = order;
    }
}

/// Index definition for document type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Index {
    pub name: String,
    pub properties: Vec<IndexProperties>,
    pub unique: bool,
}

impl Index {
    /// Creates a new index with the given name
    pub fn new(name: String) -> Self {
        Self {
            name,
            properties: Vec::new(),
            unique: false,
        }
    }

    /// Adds a property to this index
    pub fn add_property(&mut self, field: String) {
        self.properties
            .push(IndexProperties::new(field, "asc".to_string()));
    }

    /// Removes a property at the given index
    pub fn remove_property(&mut self, index: usize) -> Option<IndexProperties> {
        if index < self.properties.len() {
            Some(self.properties.remove(index))
        } else {
            None
        }
    }

    /// Validates the index configuration
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Index name cannot be empty".to_string());
        }

        if self.properties.is_empty() {
            return Err("Index must have at least one property".to_string());
        }

        for prop in &self.properties {
            if prop.field().is_empty() {
                return Err("Index property field cannot be empty".to_string());
            }
            if prop.order() != "asc" {
                return Err(
                    "Index properties must use 'asc' sort order for Dash Platform".to_string(),
                );
            }
        }

        Ok(())
    }
}
