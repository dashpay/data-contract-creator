use crate::types::{DataType, DocumentType, Index, IndexProperties, Property};
use serde_json::Value;

/// Service for parsing JSON into internal data structures
pub struct JsonParser;

impl JsonParser {
    /// Parses a JSON string into document types
    pub fn parse_contract(json_str: &str) -> Result<Vec<DocumentType>, String> {
        let json_value: Value =
            serde_json::from_str(json_str).map_err(|e| format!("Invalid JSON: {}", e))?;

        let obj = json_value
            .as_object()
            .ok_or("Root level must be an object")?;

        let mut document_types = Vec::new();

        for (name, doc_def) in obj {
            let doc_type = Self::parse_document_type(name, doc_def)?;
            document_types.push(doc_type);
        }

        Ok(document_types)
    }

    /// Parses a single document type from JSON
    fn parse_document_type(name: &str, doc_def: &Value) -> Result<DocumentType, String> {
        let doc_obj = doc_def
            .as_object()
            .ok_or_else(|| format!("Document type '{}' must be an object", name))?;

        let mut doc_type = DocumentType::new(name.to_string());

        // Parse properties
        if let Some(properties) = doc_obj.get("properties") {
            doc_type.properties = Self::parse_properties(properties)?;
        }

        // Parse indices
        if let Some(indices) = doc_obj.get("indices") {
            doc_type.indices = Self::parse_indices(indices)?;
        }

        // Parse required array
        if let Some(required) = doc_obj.get("required") {
            doc_type.required = Self::parse_required_array(required)?;

            // Check for system properties
            doc_type.created_at_required = doc_type.required.contains(&"$createdAt".to_string());
            doc_type.updated_at_required = doc_type.required.contains(&"$updatedAt".to_string());
        }

        // Parse additionalProperties
        if let Some(additional_props) = doc_obj.get("additionalProperties") {
            doc_type.additionalProperties = additional_props.as_bool().unwrap_or(false);
        }

        // Parse description
        if let Some(description) = doc_obj.get("description").and_then(|v| v.as_str()) {
            doc_type.description = description.to_string();
        }

        // Parse keywords
        if let Some(keywords) = doc_obj.get("keywords").and_then(|v| v.as_array()) {
            let keyword_strings: Vec<String> = keywords
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
            doc_type.keywords = keyword_strings.join(", ");
        }

        // Parse comment
        if let Some(comment) = doc_obj.get("$comment").and_then(|v| v.as_str()) {
            doc_type.comment = comment.to_string();
        }

        // Update required flags for properties
        for property in &mut doc_type.properties {
            property.required = doc_type.required.contains(&property.name);
        }

        Ok(doc_type)
    }

    /// Parses properties object
    fn parse_properties(properties: &Value) -> Result<Vec<Property>, String> {
        let props_obj = properties
            .as_object()
            .ok_or("Properties must be an object")?;

        let mut props = Vec::new();

        for (name, prop_def) in props_obj {
            // Skip system properties
            if name.starts_with('$') {
                continue;
            }

            let property = Self::parse_property(name, prop_def)?;
            props.push(property);
        }

        // Sort by position
        props.sort_by_key(|p| p.position);

        Ok(props)
    }

    /// Parses a single property from JSON
    fn parse_property(name: &str, prop_def: &Value) -> Result<Property, String> {
        let prop_obj = prop_def
            .as_object()
            .ok_or_else(|| format!("Property '{}' must be an object", name))?;

        let mut property = Property::new(name.to_string(), DataType::String);

        // Parse position
        if let Some(position) = prop_obj.get("position") {
            property.position = position
                .as_u64()
                .ok_or_else(|| format!("Position for property '{}' must be a number", name))?;
        }

        // Parse type
        if let Some(prop_type) = prop_obj.get("type") {
            let type_str = prop_type
                .as_str()
                .ok_or_else(|| format!("Type for property '{}' must be a string", name))?;

            property.data_type = match type_str {
                "string" => DataType::String,
                "integer" => DataType::Integer,
                "number" => DataType::Number,
                "array" => DataType::Array,
                "object" => DataType::Object,
                "boolean" => DataType::Boolean,
                _ => {
                    return Err(format!(
                        "Unknown type '{}' for property '{}'",
                        type_str, name
                    ))
                }
            };
        }

        // Parse description
        if let Some(description) = prop_obj.get("description") {
            if let Some(desc_str) = description.as_str() {
                property.description = Some(desc_str.to_string());
            }
        }

        // Parse type-specific properties
        match property.data_type {
            DataType::String => {
                Self::parse_string_properties(&mut property, prop_obj);
            }
            DataType::Integer | DataType::Number => {
                Self::parse_number_properties(&mut property, prop_obj);
            }
            DataType::Array => {
                Self::parse_array_properties(&mut property, prop_obj);
            }
            DataType::Object => {
                Self::parse_object_properties(&mut property, prop_obj)?;
            }
            DataType::Boolean => {
                // Boolean has no additional properties
            }
        }

        Ok(property)
    }

    /// Parses string-specific properties
    fn parse_string_properties(property: &mut Property, prop_obj: &serde_json::Map<String, Value>) {
        if let Some(min_length) = prop_obj.get("minLength").and_then(|v| v.as_u64()) {
            property.min_length = Some(min_length as u32);
        }
        if let Some(max_length) = prop_obj.get("maxLength").and_then(|v| v.as_u64()) {
            property.max_length = Some(max_length as u32);
        }
        if let Some(pattern) = prop_obj.get("pattern").and_then(|v| v.as_str()) {
            property.pattern = Some(pattern.to_string());
        }
        if let Some(format) = prop_obj.get("format").and_then(|v| v.as_str()) {
            property.format = Some(format.to_string());
        }
    }

    /// Parses number/integer-specific properties
    fn parse_number_properties(property: &mut Property, prop_obj: &serde_json::Map<String, Value>) {
        if let Some(minimum) = prop_obj.get("minimum").and_then(|v| v.as_i64()) {
            property.minimum = Some(minimum as i32);
        }
        if let Some(maximum) = prop_obj.get("maximum").and_then(|v| v.as_i64()) {
            property.maximum = Some(maximum as i32);
        }
    }

    /// Parses array-specific properties
    fn parse_array_properties(property: &mut Property, prop_obj: &serde_json::Map<String, Value>) {
        if let Some(byte_array) = prop_obj.get("byteArray").and_then(|v| v.as_bool()) {
            property.byte_array = Some(byte_array);
        }
        if let Some(min_items) = prop_obj.get("minItems").and_then(|v| v.as_u64()) {
            property.min_items = Some(min_items as u32);
        }
        if let Some(max_items) = prop_obj.get("maxItems").and_then(|v| v.as_u64()) {
            property.max_items = Some(max_items as u32);
        }
        if let Some(content_media_type) = prop_obj.get("contentMediaType").and_then(|v| v.as_str())
        {
            property.content_media_type = Some(content_media_type.to_string());
        }
    }

    /// Parses object-specific properties
    fn parse_object_properties(
        property: &mut Property,
        prop_obj: &serde_json::Map<String, Value>,
    ) -> Result<(), String> {
        if let Some(nested_props) = prop_obj.get("properties") {
            let nested_properties = Self::parse_properties(nested_props)?;
            property.properties = Some(Box::new(nested_properties));
        }

        if let Some(min_properties) = prop_obj.get("minProperties").and_then(|v| v.as_u64()) {
            property.min_properties = Some(min_properties as u32);
        }
        if let Some(max_properties) = prop_obj.get("maxProperties").and_then(|v| v.as_u64()) {
            property.max_properties = Some(max_properties as u32);
        }

        if let Some(required) = prop_obj.get("required") {
            property.rec_required = Some(Self::parse_required_array(required)?);
        }

        if let Some(additional_properties) = prop_obj
            .get("additionalProperties")
            .and_then(|v| v.as_bool())
        {
            property.additional_properties = Some(additional_properties);
        }

        Ok(())
    }

    /// Parses indices array
    fn parse_indices(indices: &Value) -> Result<Vec<Index>, String> {
        let indices_array = indices.as_array().ok_or("Indices must be an array")?;

        let mut parsed_indices = Vec::new();

        for (i, index_def) in indices_array.iter().enumerate() {
            let index = Self::parse_index(index_def)
                .map_err(|e| format!("Error parsing index {}: {}", i, e))?;
            parsed_indices.push(index);
        }

        Ok(parsed_indices)
    }

    /// Parses a single index from JSON
    fn parse_index(index_def: &Value) -> Result<Index, String> {
        let index_obj = index_def.as_object().ok_or("Index must be an object")?;

        let mut index = Index::default();

        // Parse name
        if let Some(name) = index_obj.get("name").and_then(|v| v.as_str()) {
            index.name = name.to_string();
        } else {
            return Err("Index must have a name".to_string());
        }

        // Parse properties
        if let Some(properties) = index_obj.get("properties") {
            index.properties = Self::parse_index_properties(properties)?;
        }

        // Parse unique flag
        if let Some(unique) = index_obj.get("unique").and_then(|v| v.as_bool()) {
            index.unique = unique;
        }

        Ok(index)
    }

    /// Parses index properties array
    fn parse_index_properties(properties: &Value) -> Result<Vec<IndexProperties>, String> {
        let props_array = properties
            .as_array()
            .ok_or("Index properties must be an array")?;

        let mut index_props = Vec::new();

        for prop_def in props_array {
            let prop_obj = prop_def
                .as_object()
                .ok_or("Index property must be an object")?;

            // Each object should have exactly one key-value pair
            if prop_obj.len() != 1 {
                return Err(
                    "Index property object must have exactly one key-value pair".to_string()
                );
            }

            let (field_name, order_value) = prop_obj.iter().next().unwrap();
            let order = order_value
                .as_str()
                .ok_or("Index property order must be a string")?;

            index_props.push(IndexProperties::new(field_name.clone(), order.to_string()));
        }

        Ok(index_props)
    }

    /// Parses required array
    fn parse_required_array(required: &Value) -> Result<Vec<String>, String> {
        let required_array = required.as_array().ok_or("Required must be an array")?;

        let mut required_props = Vec::new();

        for req_def in required_array {
            let prop_name = req_def
                .as_str()
                .ok_or("Required property name must be a string")?;
            required_props.push(prop_name.to_string());
        }

        Ok(required_props)
    }
}
