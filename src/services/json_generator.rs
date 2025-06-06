use crate::types::{DataType, DocumentType, Index, Property};
use serde_json::{Map, Value};

/// Service for generating JSON from internal data structures
pub struct JsonGenerator;

impl JsonGenerator {
    /// Generates a complete data contract JSON from document types
    pub fn generate_contract(document_types: &[DocumentType]) -> Value {
        let mut contract = Map::new();

        for doc_type in document_types {
            if !doc_type.name.is_empty() {
                let doc_json = Self::generate_document_type(doc_type);
                contract.insert(doc_type.name.clone(), doc_json);
            }
        }

        Value::Object(contract)
    }

    /// Generates JSON for a single document type
    fn generate_document_type(doc_type: &DocumentType) -> Value {
        let mut doc_obj = Map::new();

        // Set type to object
        doc_obj.insert("type".to_string(), Value::String("object".to_string()));

        // Generate properties
        if !doc_type.properties.is_empty() {
            let properties = Self::generate_properties(&doc_type.properties);
            doc_obj.insert("properties".to_string(), properties);
        }

        // System properties are not added to properties object
        // They exist at the platform level and are only referenced in required array

        // Generate indices
        if !doc_type.indices.is_empty() {
            let indices = Self::generate_indices(&doc_type.indices);
            doc_obj.insert("indices".to_string(), Value::Array(indices));
        }

        // Generate required array
        let mut required = doc_type
            .properties
            .iter()
            .filter(|prop| prop.required)
            .map(|prop| prop.name.clone())
            .collect::<Vec<_>>();

        if doc_type.created_at_required {
            required.push("$createdAt".to_string());
        }
        if doc_type.updated_at_required {
            required.push("$updatedAt".to_string());
        }

        if !required.is_empty() {
            let required_values: Vec<Value> = required.into_iter().map(Value::String).collect();
            doc_obj.insert("required".to_string(), Value::Array(required_values));
        }

        // Set additionalProperties
        doc_obj.insert(
            "additionalProperties".to_string(),
            Value::Bool(doc_type.additionalProperties),
        );

        // Add description if present
        if !doc_type.description.is_empty() {
            doc_obj.insert("description".to_string(), Value::String(doc_type.description.clone()));
        }

        // Add keywords if present
        if !doc_type.keywords.is_empty() {
            // Split keywords by comma and trim whitespace
            let keywords: Vec<String> = doc_type.keywords
                .split(',')
                .map(|k| k.trim().to_string())
                .filter(|k| !k.is_empty())
                .collect();
            
            if !keywords.is_empty() {
                let keyword_values: Vec<Value> = keywords.into_iter().map(Value::String).collect();
                doc_obj.insert("keywords".to_string(), Value::Array(keyword_values));
            }
        }

        // Add comment if present (internal documentation)
        if !doc_type.comment.is_empty() {
            doc_obj.insert("$comment".to_string(), Value::String(doc_type.comment.clone()));
        }

        Value::Object(doc_obj)
    }

    /// Generates properties object
    fn generate_properties(properties: &[Property]) -> Value {
        let mut props_obj = Map::new();

        for prop in properties {
            if !prop.name.is_empty() {
                let prop_json = Self::generate_property(prop);
                props_obj.insert(prop.name.clone(), prop_json);
            }
        }

        Value::Object(props_obj)
    }

    /// Generates JSON for a single property
    fn generate_property(prop: &Property) -> Value {
        let mut prop_obj = Map::new();

        // Set position
        prop_obj.insert("position".to_string(), Value::Number(prop.position.into()));

        // Set type
        prop_obj.insert(
            "type".to_string(),
            Value::String(prop.data_type.as_str().to_string()),
        );

        // Add description if present
        if let Some(ref description) = prop.description {
            if !description.is_empty() {
                prop_obj.insert(
                    "description".to_string(),
                    Value::String(description.clone()),
                );
            }
        }

        // Add type-specific properties
        match prop.data_type {
            DataType::String => {
                Self::add_string_properties(&mut prop_obj, prop);
            }
            DataType::Integer | DataType::Number => {
                Self::add_number_properties(&mut prop_obj, prop);
            }
            DataType::Array => {
                Self::add_array_properties(&mut prop_obj, prop);
            }
            DataType::Object => {
                Self::add_object_properties(&mut prop_obj, prop);
            }
            DataType::Boolean => {
                // Boolean type has no additional properties
            }
        }

        Value::Object(prop_obj)
    }

    /// Adds string-specific properties
    fn add_string_properties(prop_obj: &mut Map<String, Value>, prop: &Property) {
        if let Some(min_length) = prop.min_length {
            prop_obj.insert("minLength".to_string(), Value::Number(min_length.into()));
        }
        if let Some(max_length) = prop.max_length {
            prop_obj.insert("maxLength".to_string(), Value::Number(max_length.into()));
        }
        if let Some(ref pattern) = prop.pattern {
            if !pattern.is_empty() {
                prop_obj.insert("pattern".to_string(), Value::String(pattern.clone()));
            }
        }
        if let Some(ref format) = prop.format {
            if !format.is_empty() {
                prop_obj.insert("format".to_string(), Value::String(format.clone()));
            }
        }
    }

    /// Adds number/integer-specific properties
    fn add_number_properties(prop_obj: &mut Map<String, Value>, prop: &Property) {
        if let Some(minimum) = prop.minimum {
            prop_obj.insert("minimum".to_string(), Value::Number(minimum.into()));
        }
        if let Some(maximum) = prop.maximum {
            prop_obj.insert("maximum".to_string(), Value::Number(maximum.into()));
        }
    }

    /// Adds array-specific properties
    fn add_array_properties(prop_obj: &mut Map<String, Value>, prop: &Property) {
        // Arrays must have byteArray: true in Dash Platform
        prop_obj.insert("byteArray".to_string(), Value::Bool(true));

        if let Some(min_items) = prop.min_items {
            prop_obj.insert("minItems".to_string(), Value::Number(min_items.into()));
        }
        if let Some(max_items) = prop.max_items {
            prop_obj.insert("maxItems".to_string(), Value::Number(max_items.into()));
        }
        if let Some(ref content_media_type) = prop.content_media_type {
            if !content_media_type.is_empty() {
                prop_obj.insert(
                    "contentMediaType".to_string(),
                    Value::String(content_media_type.clone()),
                );
            }
        }
    }

    /// Adds object-specific properties
    fn add_object_properties(prop_obj: &mut Map<String, Value>, prop: &Property) {
        if let Some(ref nested_props) = prop.properties {
            if !nested_props.is_empty() {
                let nested_properties = Self::generate_properties(nested_props);
                prop_obj.insert("properties".to_string(), nested_properties);
            }
        }

        if let Some(min_properties) = prop.min_properties {
            prop_obj.insert(
                "minProperties".to_string(),
                Value::Number(min_properties.into()),
            );
        }
        if let Some(max_properties) = prop.max_properties {
            prop_obj.insert(
                "maxProperties".to_string(),
                Value::Number(max_properties.into()),
            );
        }

        if let Some(ref required) = prop.rec_required {
            if !required.is_empty() {
                let required_values: Vec<Value> =
                    required.iter().map(|s| Value::String(s.clone())).collect();
                prop_obj.insert("required".to_string(), Value::Array(required_values));
            }
        }

        if let Some(additional_properties) = prop.additional_properties {
            prop_obj.insert(
                "additionalProperties".to_string(),
                Value::Bool(additional_properties),
            );
        }
    }

    /// Generates indices array
    fn generate_indices(indices: &[Index]) -> Vec<Value> {
        indices
            .iter()
            .filter(|index| !index.name.is_empty() && !index.properties.is_empty())
            .map(Self::generate_index)
            .collect()
    }

    /// Generates JSON for a single index
    fn generate_index(index: &Index) -> Value {
        let mut index_obj = Map::new();

        index_obj.insert("name".to_string(), Value::String(index.name.clone()));

        let properties: Vec<Value> = index
            .properties
            .iter()
            .filter(|prop| !prop.0.is_empty())
            .map(|prop| {
                let mut prop_map = Map::new();
                prop_map.insert(prop.0.clone(), Value::String(prop.1.clone()));
                Value::Object(prop_map)
            })
            .collect();

        index_obj.insert("properties".to_string(), Value::Array(properties));

        if index.unique {
            index_obj.insert("unique".to_string(), Value::Bool(true));
        }

        Value::Object(index_obj)
    }
}
