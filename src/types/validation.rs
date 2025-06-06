use serde::{Deserialize, Serialize};

/// Validation error with path information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub path: String,
    pub message: String,
    pub error_type: ValidationErrorType,
}

/// Types of validation errors
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationErrorType {
    SchemaValidation,
    RequiredField,
    InvalidFormat,
    IndexError,
    PropertyError,
    Custom(String),
}

impl ValidationError {
    /// Creates a new validation error
    pub fn new(path: String, message: String, error_type: ValidationErrorType) -> Self {
        Self {
            path,
            message,
            error_type,
        }
    }

    /// Creates a schema validation error
    pub fn schema_error(path: String, message: String) -> Self {
        Self::new(path, message, ValidationErrorType::SchemaValidation)
    }

    /// Creates a required field error
    pub fn required_field_error(path: String, field: String) -> Self {
        Self::new(
            path,
            format!("Required field '{}' is missing", field),
            ValidationErrorType::RequiredField,
        )
    }

    /// Creates an index error
    pub fn index_error(path: String, message: String) -> Self {
        Self::new(path, message, ValidationErrorType::IndexError)
    }

    /// Creates a property error
    pub fn property_error(path: String, message: String) -> Self {
        Self::new(path, message, ValidationErrorType::PropertyError)
    }

    /// Creates a custom error
    pub fn custom_error(path: String, message: String, custom_type: String) -> Self {
        Self::new(path, message, ValidationErrorType::Custom(custom_type))
    }

    /// Returns a formatted error message for display
    pub fn display_message(&self) -> String {
        if self.path.is_empty() {
            self.message.clone()
        } else {
            format!("{}: {}", self.path, self.message)
        }
    }
}
