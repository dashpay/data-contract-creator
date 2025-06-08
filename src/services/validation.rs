use crate::types::ValidationError;
use anyhow::Result;
use dpp::{
    consensus::ConsensusError,
    data_contract::{DataContractFactory, JsonValue},
    platform_value::Value as PlatformValue,
    prelude::Identifier,
    util::json_value::JsonValueExt,
    validation::json_schema_validator::JsonSchemaValidator,
    version::PlatformVersion,
};
use serde_json::json;
use std::collections::HashSet;

/// Service for validating data contracts using Dash Platform Protocol
pub struct ValidationService;

impl ValidationService {
    /// Validates a JSON schema against Dash Platform Protocol rules using DPP
    pub fn validate_schema(json_str: &str) -> Result<Vec<ValidationError>, String> {
        // Basic JSON validation
        let json_obj: JsonValue =
            serde_json::from_str(json_str).map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Handle empty contracts
        if let Ok(obj) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(map) = obj.as_object() {
                if map.is_empty() {
                    return Ok(vec![ValidationError::schema_error(
                        "".to_string(),
                        "Data contract must have at least one document type".to_string(),
                    )]);
                }
            }
        }

        // Convert `serde_json::Value` to `dpp::platform_value::Value`
        let platform_value: PlatformValue = PlatformValue::from(json_obj);

        let factory = DataContractFactory::new(PlatformVersion::latest().protocol_version)
            .map_err(|e| format!("Failed to create data contract factory: {}", e))?;
        let owner_id = Identifier::random();

        // Create data contract
        let contract_result = factory.create(owner_id, u64::default(), platform_value, None, None);

        match contract_result {
            Ok(contract) => {
                // Convert DataContract to JsonValue
                let mut contract_json: JsonValue =
                    serde_json::to_value(contract.data_contract().as_v1())
                        .map_err(|e| format!("Failed to serialize contract: {}", e))?;

                // Insert a blank description for the validator
                contract_json
                    .insert("description".to_string(), json!(""))
                    .map_err(|e| format!("Failed to insert description: {}", e))?;

                // Create the validator
                let validator =
                    JsonSchemaValidator::new_compiled(&contract_json, &PlatformVersion::latest())
                        .map_err(|e| format!("Failed to create JsonSchemaValidator: {}", e))?;

                // Validate the data contract
                let results = validator
                    .validate(&contract_json, &PlatformVersion::latest())
                    .map_err(|e| format!("Validation failed: {}", e))?;

                let errors = results.errors;
                Ok(Self::extract_basic_error_messages(&errors))
            }
            Err(e) => Ok(vec![ValidationError::schema_error(
                "".to_string(),
                format!("{}", e),
            )]),
        }
    }

    /// Extracts the BasicError messages from DPP consensus errors
    fn extract_basic_error_messages(errors: &[ConsensusError]) -> Vec<ValidationError> {
        let messages: Vec<ValidationError> = errors
            .iter()
            .filter_map(|error| {
                if let ConsensusError::BasicError(inner) = error {
                    if let dpp::errors::consensus::basic::basic_error::BasicError::JsonSchemaError(json_error) = inner {
                        if json_error.error_summary().contains("\"items\" is a required property") {
                            Some(ValidationError::schema_error(
                                json_error.instance_path().to_string(),
                                "Array properties must specify \"byteArray\": true. In the dynamic form, just change the property from an array to a string and back to an array again, and resubmit.".to_string(),
                            ))
                        } else {
                            Some(ValidationError::schema_error(
                                json_error.instance_path().to_string(),
                                format!("JsonSchemaError: {}", json_error.error_summary()),
                            ))
                        }
                    } else {
                        Some(ValidationError::schema_error(
                            "".to_string(),
                            format!("{}", inner),
                        ))
                    }
                } else {
                    None
                }
            })
            .collect();

        // Remove duplicates
        let unique_messages: HashSet<String> =
            messages.iter().map(|e| e.display_message()).collect();

        unique_messages
            .into_iter()
            .map(|msg| ValidationError::schema_error("".to_string(), msg))
            .collect()
    }

    /// Legacy method for backward compatibility - now uses DPP validation
    pub fn validate_byte_arrays(_json_str: &str) -> Vec<ValidationError> {
        // This is now handled by DPP validation in validate_schema
        Vec::new()
    }

    /// Legacy method for backward compatibility - now uses DPP validation
    pub fn validate_indexed_string_lengths(_json_str: &str) -> Vec<ValidationError> {
        // This is now handled by DPP validation in validate_schema
        Vec::new()
    }
}
