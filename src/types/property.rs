use serde::{Deserialize, Serialize};

/// Property data types supported by Dash Platform
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum DataType {
    #[default]
    String,
    Integer,
    Array,
    Object,
    Number,
    Boolean,
}

impl DataType {
    /// Returns the string representation of the data type
    pub fn as_str(&self) -> &'static str {
        match self {
            DataType::String => "string",
            DataType::Integer => "integer",
            DataType::Array => "array",
            DataType::Object => "object",
            DataType::Number => "number",
            DataType::Boolean => "boolean",
        }
    }

    /// Returns all available data types
    pub fn all() -> Vec<DataType> {
        vec![
            DataType::String,
            DataType::Integer,
            DataType::Array,
            DataType::Object,
            DataType::Number,
            DataType::Boolean,
        ]
    }
}

/// Property in a document type with validation parameters specific to each data type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Property {
    pub name: String,
    pub data_type: DataType,
    pub required: bool,
    pub position: u64,
    pub description: Option<String>,
    pub comment: Option<String>,

    // String-specific validation
    pub min_length: Option<u32>,
    pub max_length: Option<u32>,
    pub pattern: Option<String>,
    pub format: Option<String>,

    // Number/Integer-specific validation
    pub minimum: Option<i32>,
    pub maximum: Option<i32>,

    // Array-specific validation
    pub byte_array: Option<bool>,
    pub min_items: Option<u32>,
    pub max_items: Option<u32>,
    pub content_media_type: Option<String>,

    // Object-specific validation
    pub properties: Option<Box<Vec<Property>>>,
    pub min_properties: Option<u32>,
    pub max_properties: Option<u32>,
    pub rec_required: Option<Vec<String>>,
    pub additional_properties: Option<bool>,
}

impl Property {
    /// Creates a new property with the given name and data type
    pub fn new(name: String, data_type: DataType) -> Self {
        Self {
            name,
            data_type,
            ..Default::default()
        }
    }

    /// Clears validation parameters that don't apply to the current data type
    pub fn clear_invalid_parameters(&mut self) {
        match self.data_type {
            DataType::String => {
                self.minimum = None;
                self.maximum = None;
                self.byte_array = None;
                self.min_items = None;
                self.max_items = None;
                self.content_media_type = None;
                self.properties = None;
                self.min_properties = None;
                self.max_properties = None;
                self.rec_required = None;
                self.additional_properties = None;
            }
            DataType::Integer | DataType::Number => {
                self.min_length = None;
                self.max_length = None;
                self.pattern = None;
                self.format = None;
                self.byte_array = None;
                self.min_items = None;
                self.max_items = None;
                self.content_media_type = None;
                self.properties = None;
                self.min_properties = None;
                self.max_properties = None;
                self.rec_required = None;
                self.additional_properties = None;
            }
            DataType::Array => {
                self.min_length = None;
                self.max_length = None;
                self.pattern = None;
                self.format = None;
                self.minimum = None;
                self.maximum = None;
                self.properties = None;
                self.min_properties = None;
                self.max_properties = None;
                self.rec_required = None;
                self.additional_properties = None;
                self.byte_array = Some(true); // Arrays must be byte arrays in Dash Platform
            }
            DataType::Object => {
                self.min_length = None;
                self.max_length = None;
                self.pattern = None;
                self.format = None;
                self.minimum = None;
                self.maximum = None;
                self.byte_array = None;
                self.min_items = None;
                self.max_items = None;
                self.content_media_type = None;
            }
            DataType::Boolean => {
                self.min_length = None;
                self.max_length = None;
                self.pattern = None;
                self.format = None;
                self.minimum = None;
                self.maximum = None;
                self.byte_array = None;
                self.min_items = None;
                self.max_items = None;
                self.content_media_type = None;
                self.properties = None;
                self.min_properties = None;
                self.max_properties = None;
                self.rec_required = None;
                self.additional_properties = None;
            }
        }
    }

    /// Validates that the property configuration is valid for Dash Platform
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Property name cannot be empty".to_string());
        }

        match self.data_type {
            DataType::String => {
                if let (Some(min), Some(max)) = (self.min_length, self.max_length) {
                    if min > max {
                        return Err(
                            "Minimum length cannot be greater than maximum length".to_string()
                        );
                    }
                }
            }
            DataType::Integer | DataType::Number => {
                if let (Some(min), Some(max)) = (self.minimum, self.maximum) {
                    if min > max {
                        return Err("Minimum cannot be greater than maximum".to_string());
                    }
                }
            }
            DataType::Array => {
                if let (Some(min), Some(max)) = (self.min_items, self.max_items) {
                    if min > max {
                        return Err(
                            "Minimum items cannot be greater than maximum items".to_string()
                        );
                    }
                }
                if self.max_items.map_or(false, |max| max > 255) {
                    return Err("Maximum items for arrays cannot exceed 255".to_string());
                }
            }
            DataType::Object => {
                if let (Some(min), Some(max)) = (self.min_properties, self.max_properties) {
                    if min > max {
                        return Err(
                            "Minimum properties cannot be greater than maximum properties"
                                .to_string(),
                        );
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }
}
