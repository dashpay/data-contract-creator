use anyhow::{anyhow, Result};
use serde_json::json;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

/// Service for interacting with OpenAI API
pub struct OpenAiService;

impl OpenAiService {
    /// Context prepended to the first user-input prompt when creating a new contract
    const FIRST_PROMPT_PRE: &'static str = r#"
I'm going to ask you to generate a Dash Platform data contract after giving you some context and rules. 

*Background info*: 
Dash Platform is a blockchain for decentralized applications that are backed by data contracts. 
Data contracts are JSON schemas that are meant to define the structures of data an application can store. 
They must define at least one document type, where a document type defines a type of document that can be submitted to a data contract.

*Example*: 
Here is an example of a data contract with one document type, "nft":

{"nft":{"type":"object","properties":{"name":{"position":0,"type":"string","description":"Name of the NFT token","maxLength":63},"description":{"position":1,"type":"string","description":"Description of the NFT token","maxLength":256},"imageUrl":{"position":2,"type":"string","description":"URL of the image associated with the NFT token","maxLength":2048,"format":"uri"},"imageHash":{"position":3,"type":"array","description":"SHA256 hash of the bytes of the image specified by tokenImageUrl","byteArray":true,"minItems":32,"maxItems":32},"imageFingerprint":{"position":4,"type":"array","description":"dHash the image specified by tokenImageUrl","byteArray":true,"minItems":8,"maxItems":8},"price":{"position":5,"type":"number","description":"Price of the NFT token in Dash","minimum":0},"quantity":{"position":6,"type":"integer","description":"Number of tokens in circulation","minimum":0},"metadata":{"position":7,"type":"array","description":"Any additional metadata associated with the NFT token","byteArray":true,"minItems":0,"maxItems":2048}},"indices":[{"name":"price","properties":[{"price":"asc"}]},{"name":"quantity","properties":[{"quantity":"asc"}]},{"name":"priceAndQuantity","properties":[{"price":"asc"},{"quantity":"asc"}]}],"required":["name","price","quantity"],"additionalProperties":false}}

While this example data contract only has one document type, data contracts should usually have more than one. For example, the example "nft" data contract could also have document types for "listing" and "transaction". Maybe the developer also wants to have user profiles, so they could include a "userProfile" document type.

*Requirements*:
The following requirements must be met in Dash Platform data contracts:
 - Indexes may only have "asc" sort order.
 - All "string" properties that are used in indexes must specify "maxLength", which must be no more than 63.
 - All "array" properties that are used in indexes must specify "maxItems", and it must be less than or equal to 255.
 - All "array" properties must specify `"byteArray": true`.
 - All "object" properties must define at least 1 property within themselves.
 - All properties must define a "position" field, which is a number starting at 0, incrementing for each property.

*App description*: 
Now I will give you a user prompt that describes the application that you will generate a data contract for.

When creating the data contract, please:
 - Include descriptions for every document type and property. Be creative, extensive, and utilize multiple document types if possible.
 - Include both "description" and "comment" fields for every document type (at the same level as "type", "properties", etc.).
 - Include indexes for any properties that it makes sense for a useful app to index. More is better. 
 - Do not explain anything or return anything else other than a properly formatted data contract JSON schema. 
 - Double check that all requirements and requests above are met. Again, all "array" properties must specify `"byteArray": true`.

App description: 

"#;

    /// Context prepended to user-input prompts after the first prompt
    const SECOND_PROMPT_PRE: &'static str = r#"
I'm going to ask you to make some changes to a Dash Platform data contract after giving you some context and rules. 

*Requirements*:
The following requirements must be met in Dash Platform data contracts: 
 - Indexes may only have "asc" sort order. 
 - All "array" properties must specify "byteArray": true. 
 - All "string" properties that are used in indexes must specify "maxLength", which must be no more than 63. 
 - All "array" properties that are used in indexes must specify "maxItems", and it must be less than or equal to 255. 
 - All "object" properties must define at least 1 property within themselves. 

*Changes to be made*: 
Make the following change(s) to this Dash Platform data contract JSON schema, along with any other changes that are necessary to make it valid according to the rules above. 
Note that the highest-level keys in the data contract are called "document types".
Do not explain anything or return anything else other than a properly formatted JSON schema:

"#;

    /// Calls OpenAI API to generate or modify data contracts
    pub async fn generate_contract(prompt: &str, existing_schema: Option<&str>) -> Result<String> {
        let full_prompt = if let Some(schema) = existing_schema {
            format!(
                "{}\n\nExisting schema:\n{}\n\nUser request:\n{}",
                Self::SECOND_PROMPT_PRE,
                schema,
                prompt
            )
        } else {
            format!("{}{}", Self::FIRST_PROMPT_PRE, prompt)
        };

        Self::call_api(&full_prompt).await
    }

    /// Makes the actual API call to OpenAI
    async fn call_api(prompt: &str) -> Result<String> {
        let params = json!({
            "model": "gpt-4o",
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": 4096,
            "temperature": 0.2
        });

        let mut opts = RequestInit::new();
        let headers =
            web_sys::Headers::new().map_err(|e| anyhow!("Failed to create headers: {:?}", e))?;

        headers
            .append("Content-Type", "application/json")
            .map_err(|e| anyhow!("Failed to set content type: {:?}", e))?;

        // Note: For local testing, uncomment and add your API key:
        // headers.append("Authorization", &format!("Bearer {}", "YOUR_API_KEY_HERE"))
        //     .map_err(|e| anyhow!("Failed to set authorization: {:?}", e))?;

        opts.method("POST");
        opts.headers(&headers);
        opts.body(Some(&JsValue::from_str(&params.to_string())));
        opts.mode(RequestMode::Cors);

        // Use the Lambda endpoint for production, OpenAI directly for local testing
        let url = "https://22vazdmku2qz3prrn57elhdj2i0wyejr.lambda-url.us-west-2.on.aws/";
        // For local testing, use: "https://api.openai.com/v1/chat/completions"

        let request = Request::new_with_str_and_init(url, &opts)
            .map_err(|e| anyhow!("Failed to create request: {:?}", e))?;

        let window = web_sys::window().ok_or_else(|| anyhow!("Failed to obtain window object"))?;

        let response = JsFuture::from(window.fetch_with_request(&request))
            .await
            .map_err(|e| {
                anyhow!(
                    "Fetch request failed: {:?}",
                    e.as_string().unwrap_or_default()
                )
            })?;

        let response: Response = response.dyn_into().map_err(|e| {
            anyhow!(
                "Failed to convert response: {:?}",
                e.as_string().unwrap_or_default()
            )
        })?;

        let text = JsFuture::from(
            response
                .text()
                .map_err(|_| anyhow!("Failed to read response text"))?,
        )
        .await
        .map_err(|e| {
            anyhow!(
                "Failed to get response text: {:?}",
                e.as_string().unwrap_or_default()
            )
        })?;

        let text = text
            .as_string()
            .ok_or_else(|| anyhow!("Failed to convert response to string"))?;

        if !response.ok() {
            let status = response.status();
            let error_message = Self::extract_error_message(&text);
            return Err(anyhow!("HTTP {} error from API: {}", status, error_message));
        }

        Self::extract_json_schema(&text)
    }

    /// Extracts error message from API response
    fn extract_error_message(text: &str) -> String {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text) {
            json.get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or(text)
                .to_string()
        } else {
            text.to_string()
        }
    }

    /// Extracts JSON schema from OpenAI response
    fn extract_json_schema(response_text: &str) -> Result<String> {
        let json: serde_json::Value = serde_json::from_str(response_text)
            .map_err(|e| anyhow!("Failed to parse API response: {}", e))?;

        let content = json
            .get("choices")
            .and_then(|choices| choices.get(0))
            .and_then(|choice| choice.get("message"))
            .and_then(|message| message.get("content"))
            .and_then(|content| content.as_str())
            .ok_or_else(|| anyhow!("Invalid response format from API"))?;

        // Extract JSON from the response
        let start = content
            .find('{')
            .ok_or_else(|| anyhow!("No JSON found in API response"))?;
        let end = content
            .rfind('}')
            .ok_or_else(|| anyhow!("No valid JSON found in API response"))?;

        let schema_json = &content[start..=end];

        // Validate that it's proper JSON
        serde_json::from_str::<serde_json::Value>(schema_json)
            .map_err(|e| anyhow!("Extracted text is not valid JSON: {}", e))?;

        Ok(schema_json.to_string())
    }
}
