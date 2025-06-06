/// Data types and structures for the Data Contract Creator
pub mod document_type;
pub mod index;
pub mod property;
pub mod validation;

pub use document_type::DocumentType;
pub use index::{Index, IndexProperties};
pub use property::{DataType, Property};
pub use validation::ValidationError;
