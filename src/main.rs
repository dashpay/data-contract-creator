//! Dash Platform Data Contract Creator
//! 
//! This web app allows users to generate data contract schemas using ChatGPT and a dynamic form. 
//! They also have the ability to import existing contracts and edit them. 
//! The schemas are validated against Dash Platform Protocol and error messages are provided if applicable.

use std::{collections::{HashMap, HashSet}, sync::Arc};
use serde::{Serialize, Deserialize};
use yew::{prelude::*, html, Component, Html, Event, InputEvent, FocusEvent, TargetCast};
use serde_json::{json, Map, Value};
use dpp::{self, consensus::ConsensusError, prelude::Identifier, Convertible};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Request, RequestInit, RequestMode, Response, HtmlSelectElement, console};

// Context prepended to the first user-input prompt, when creating a new contract
const FIRST_PROMPT_PRE: &str = r#"
Here is the an example Dash Platform data contract JSON schema. It's for the Dashpay app, which is meant to be similar to Venmo, but for the Dash Platform blockchain. It has three document types: "profile", "contactInfo", and "contactRequest".

{"profile":{"type":"object","indices":[{"name":"ownerId","properties":[{"$ownerId":"asc"}],"unique":true},{"name":"ownerIdAndUpdatedAt","properties":[{"$ownerId":"asc"},{"$updatedAt":"asc"}]}],"properties":{"avatarUrl":{"type":"string","format":"uri","maxLength":2048},"avatarHash":{"type":"array","byteArray":true,"minItems":32,"maxItems":32,"description":"SHA256 hash of the bytes of the image specified by avatarUrl"},"avatarFingerprint":{"type":"array","byteArray":true,"minItems":8,"maxItems":8,"description":"dHash the image specified by avatarUrl"},"publicMessage":{"type":"string","maxLength":140},"displayName":{"type":"string","maxLength":25}},"required":["$createdAt","$updatedAt"],"additionalProperties":false},"contactInfo":{"type":"object","indices":[{"name":"ownerIdAndKeys","properties":[{"$ownerId":"asc"},{"rootEncryptionKeyIndex":"asc"},{"derivationEncryptionKeyIndex":"asc"}],"unique":true},{"name":"ownerIdAndUpdatedAt","properties":[{"$ownerId":"asc"},{"$updatedAt":"asc"}]}],"properties":{"encToUserId":{"type":"array","byteArray":true,"minItems":32,"maxItems":32},"rootEncryptionKeyIndex":{"type":"integer","minimum":0},"derivationEncryptionKeyIndex":{"type":"integer","minimum":0},"privateData":{"type":"array","byteArray":true,"minItems":48,"maxItems":2048,"description":"This is the encrypted values of aliasName + note + displayHidden encoded as an array in cbor"}},"required":["$createdAt","$updatedAt","encToUserId","privateData","rootEncryptionKeyIndex","derivationEncryptionKeyIndex"],"additionalProperties":false},"contactRequest":{"type":"object","indices":[{"name":"ownerIdUserIdAndAccountRef","properties":[{"$ownerId":"asc"},{"toUserId":"asc"},{"accountReference":"asc"}],"unique":true},{"name":"ownerIdUserId","properties":[{"$ownerId":"asc"},{"toUserId":"asc"}]},{"name":"userIdCreatedAt","properties":[{"toUserId":"asc"},{"$createdAt":"asc"}]},{"name":"ownerIdCreatedAt","properties":[{"$ownerId":"asc"},{"$createdAt":"asc"}]}],"properties":{"toUserId":{"type":"array","byteArray":true,"minItems":32,"maxItems":32,"contentMediaType":"application/x.dash.dpp.identifier"},"encryptedPublicKey":{"type":"array","byteArray":true,"minItems":96,"maxItems":96},"senderKeyIndex":{"type":"integer","minimum":0},"recipientKeyIndex":{"type":"integer","minimum":0},"accountReference":{"type":"integer","minimum":0},"encryptedAccountLabel":{"type":"array","byteArray":true,"minItems":48,"maxItems":80},"autoAcceptProof":{"type":"array","byteArray":true,"minItems":38,"maxItems":102},"coreHeightCreatedAt":{"type":"integer","minimum":1}},"required":["$createdAt","toUserId","encryptedPublicKey","senderKeyIndex","recipientKeyIndex","accountReference"],"additionalProperties":false}}

The following requirements must be followed in Dash Platform data contracts: 
 - Indexes may only have "asc" sort order. 
 - All "array" properties must specify ""byteArray": true". 
 - All "string" properties that are used in indexes must specify "maxLength", and it cannot be more than 63. 
 - All "array" properties that are used in indexes must specify "maxItems", and it must be less than or equal to 255. 
 - All "object" properties must define at least 1 property within themselves. 

Note that not all properties need to be included in the "required" array. 
Also note that $ownerId, $createdAt, and $updatedAt are the only properties that can be prefixed with a "$", and are the only built-in "system properties" that can be used in a data contract and don't need to be defined in its properties.
 
Given this background, generate a highly comprehensive data contract JSON schema using the context below. 
Include descriptions for every document type and property. Be creative and extensive and utilize multiple document types. 
Include indexes for any properties that it makes sense for a useful app to index. 
Do not explain anything or return anything else other than a properly formatted data contract JSON schema:

"#;

// Context prepended to user-input prompts after the first prompt. The current schema comes after this, then the user-input context.
const SECOND_PROMPT_PRE: &str = r#"
The following requirements must be followed in Dash Platform data contracts: 
 - Indexes may only have "asc" sort order. 
 - All "array" properties must specify ""byteArray": true". 
 - All "string" properties that are used in indexes must specify "maxLength", and it cannot be more than 63. 
 - All "array" properties that are used in indexes must specify "maxItems", and it must be less than or equal to 255. 
 - All "object" properties must define at least 1 property within themselves. 

Note that not all properties need to be included in the "required" array.
Also note that $ownerId, $createdAt, and $updatedAt are the only properties that can be prefixed with a "$", and are the only built-in "system properties" that can be used in a data contract and don't need to be defined in its properties.

Make the following change(s) to this Dash Platform data contract JSON schema, along with any other changes that are necessary to make it valid according to the rules above. 
Note that the highest-level keys in the data contract are called "document types".
Do not explain anything or return anything else other than a properly formatted JSON schema:

"#;

/// Calls OpenAI
/// For text-davinci-003, max_tokens 3000 is the most we can reasonably allow. 
/// Temperature is like a randomness/creativity factor. Temps above 0.7 start to produce bad results. 0.2 seems to be optimal.
pub async fn call_openai(prompt: &str, user_key: &String) -> Result<String, anyhow::Error> {
    let params = serde_json::json!({
        "model": "text-davinci-003",
        "prompt": prompt,
        "max_tokens": 3000,
        "temperature": 0.2
    });
    let params = params.to_string();

    let mut opts = RequestInit::new();
    let headers = web_sys::Headers::new().unwrap();

    headers.append("Authorization", &format!("Bearer {}", user_key)).unwrap();
    headers.append("Content-Type", "application/json").unwrap();

    opts.method("POST");
    opts.headers(&headers);
    opts.body(Some(&JsValue::from_str(&params)));
    opts.mode(RequestMode::Cors);

    let request = Request::new_with_str_and_init("https://api.openai.com/v1/completions", &opts).unwrap();

    let window = web_sys::window().unwrap();
    let response = JsFuture::from(window.fetch_with_request(&request)).await;
    
    let response = match response {
        Ok(resp) => resp,
        Err(err) => return Err(anyhow::anyhow!(err.as_string().unwrap_or("Fetch request failed".to_string()))),
    };

    let response: Response = match response.dyn_into() {
        Ok(resp) => resp,
        Err(err) => return Err(anyhow::anyhow!(err.as_string().unwrap_or("Failed to convert JsValue to Response".to_string()))),
    };

    let text_future = match response.text() {
        Ok(txt_future) => txt_future,
        Err(_) => return Err(anyhow::anyhow!("Failed to read text from the response")),
    };

    let text_js = match JsFuture::from(text_future).await {
        Ok(txt_js) => txt_js,
        Err(err) => return Err(anyhow::anyhow!(err.as_string().unwrap_or("Failed to convert future to JsValue".to_string()))),
    };

    let text = match text_js.as_string() {
        Some(txt) => txt,
        None => return Err(anyhow::anyhow!("Failed to convert JsValue to String")),
    };

    // Log to console
    console::log_1(&text_js);

    if !response.ok() {
        let status = response.status();
        let parsed_json: Result<serde_json::Value, _> = serde_json::from_str(&text);
        let message = if let Ok(json) = parsed_json {
            json.get("error").and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or(&text).to_string()
        } else {
            text
        };
        return Err(anyhow::anyhow!("HTTP {} error from OpenAI: {}", status, message));
    }
    
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let schema_text = json["choices"][0]["text"].as_str().unwrap_or("");

    // Extract the JSON schema from the response
    let start = schema_text.find('{');
    let end = schema_text.rfind('}');

    match (start, end) {
        (Some(start), Some(end)) => {
            let schema_json = &schema_text[start..=end];
            console::log_1(&JsValue::from_str(schema_json));
            match serde_json::from_str::<serde_json::Value>(schema_json) {
                Ok(_) => Ok(schema_json.to_string()),
                Err(_) => Err(anyhow::anyhow!("Extracted text is not valid JSON.")),
            }
        }
        _ => Err(anyhow::anyhow!("No valid JSON found in the returned text.")),
    }
}

/// Anything that can be changed with the interface must go in Model
struct Model {

    // Dynamic form fields

    /// A vector of document types
    document_types: Vec<DocumentType>,

    /// Each full document type is a single string in json_object
    json_object: Vec<String>,

    /// A string containing a full imported data contract
    imported_json: String,

    /// DPP validation error messages
    error_messages: Vec<String>,



    // OpenAI fields

    /// The API key
    user_key: String,

    /// The prompt sent to API
    prompt: String,

    /// The schema extracted from the API response
    schema: String,

    /// Necessary to show a loader while awaiting response
    temp_prompt: Option<String>,

    /// History of prompts
    history: Vec<String>,

    /// True while awaiting response
    loading: bool,

    /// Error messages from the API
    error_messages_ai: Vec<String>,
}

/// Document type struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct DocumentType {
    name: String,
    properties: Vec<Property>,
    indices: Vec<Index>,
    required: Vec<String>,
    created_at_required: bool,
    updated_at_required: bool,
    additionalProperties: bool,
    comment: String
}

impl Default for DocumentType {
    fn default() -> Self {
        Self {
            name: String::new(),
            properties: vec![],
            indices: vec![],
            required: vec![],
            created_at_required: false,
            updated_at_required: false,
            additionalProperties: false,
            comment: String::new()
        }
    }
}

/// Property struct with optional fields for validation parameters specific to each data type
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Property {
    name: String,
    data_type: DataType,
    required: bool,
    description: Option<String>,
    comment: Option<String>,
    min_length: Option<u32>,  // For String data type
    max_length: Option<u32>,  // For String data type
    pattern: Option<String>,  // For String data type
    format: Option<String>,   // For String data type
    minimum: Option<i32>,     // For Integer data type
    maximum: Option<i32>,     // For Integer data type
    byte_array: Option<bool>,  // For Array data type
    min_items: Option<u32>,    // For Array data type
    max_items: Option<u32>,    // For Array data type
    content_media_type: Option<String>,  // For Array data type
    properties: Option<Box<Vec<Property>>>, // For Object data type
    min_properties: Option<u32>, // For Object data type
    max_properties: Option<u32>, // For Object data type
    rec_required: Option<Vec<String>>, // For Object data type
    additional_properties: Option<bool>, // For Object data type
}

/// Index struct
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct Index {
    name: String,
    properties: Vec<IndexProperties>,
    unique: bool,
}

/// Index properties struct
#[derive(Debug, Clone, Serialize, Deserialize)]
struct IndexProperties(String, String);

impl Default for IndexProperties {
    fn default() -> Self {
        Self {
            0: String::from(""),
            1: String::from("asc")
        }
    }
}

/// Property data types enum
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
enum DataType {
    #[default]
    String,
    Integer,
    Array,
    Object,
    Number,
    Boolean
}

/// Messages from input fields which call the functions to update Model
enum Msg {
    // General
    Submit,
    AddDocumentType,
    RemoveDocumentType(usize),
    AddProperty(usize),
    RemoveProperty(usize, usize),
    AddIndex(usize),
    RemoveIndex(usize, usize),
    AddIndexProperty(usize, usize),
    UpdateName(usize, String),
    UpdateComment(usize, String),
    UpdatePropertyName(usize, usize, String),
    UpdateIndexName(usize, usize, String),
    UpdatePropertyType(usize, usize, Property),
    UpdateIndexUnique(usize, usize, bool),
    UpdateIndexSorting(usize, usize, usize, String),
    UpdatePropertyRequired(usize, usize, bool),
    UpdateSystemPropertiesRequired(usize, usize, bool),
    UpdatePropertyDescription(usize, usize, String),
    UpdatePropertyComment(usize, usize, String),
    UpdateIndexProperty(usize, usize, usize, String),

    // Optional property parameters
    UpdateStringPropertyMinLength(usize, usize, Option<u32>),
    UpdateStringPropertyMaxLength(usize, usize, Option<u32>),
    UpdateStringPropertyPattern(usize, usize, String),
    UpdateStringPropertyFormat(usize, usize, String),
    UpdateIntegerPropertyMinimum(usize, usize, Option<i32>),
    UpdateIntegerPropertyMaximum(usize, usize, Option<i32>),
    //UpdateArrayPropertyByteArray(usize, usize, bool),
    UpdateArrayPropertyMinItems(usize, usize, Option<u32>),
    UpdateArrayPropertyMaxItems(usize, usize, Option<u32>),
    UpdateArrayPropertyCMT(usize, usize, String),
    UpdateObjectPropertyMinProperties(usize, usize, Option<u32>),
    UpdateObjectPropertyMaxProperties(usize, usize, Option<u32>),

    // Recursive properties
    AddRecProperty(usize, usize),
    RemoveRecProperty(usize, usize, usize),
    UpdateRecPropertyType(usize, usize, usize, String),
    UpdateRecPropertyName(usize, usize, usize, String),
    UpdateRecPropertyRequired(usize, usize, usize, bool),
    UpdateRecPropertyDescription(usize, usize, usize, String),
    UpdateRecPropertyComment(usize, usize, usize, String),
    UpdateStringRecPropertyMinLength(usize, usize, usize, Option<u32>),
    UpdateStringRecPropertyMaxLength(usize, usize, usize, Option<u32>),
    UpdateStringRecPropertyPattern(usize, usize, usize, String),
    UpdateStringRecPropertyFormat(usize, usize, usize, String),
    UpdateIntegerRecPropertyMaximum(usize, usize, usize, Option<i32>),
    UpdateIntegerRecPropertyMinimum(usize, usize, usize, Option<i32>),
    //UpdateArrayRecPropertyByteArray(usize, usize, usize, bool),
    UpdateArrayRecPropertyMinItems(usize, usize, usize, Option<u32>),
    UpdateArrayRecPropertyMaxItems(usize, usize, usize, Option<u32>),
    UpdateArrayRecPropertyCMT(usize, usize, usize, String),
    UpdateObjectRecPropertyMaxProperties(usize, usize, usize, Option<u32>),
    UpdateObjectRecPropertyMinProperties(usize, usize, usize, Option<u32>),

    // Import
    Import,
    UpdateImportedJson(String),
    Clear,

    // OpenAI
    UpdateUserKey(String),
    UpdatePrompt(String),
    GenerateSchema,
    ReceiveSchema(Result<String, anyhow::Error>),
    ClearInput,
}

/// Sets the validation parameters to default. Used to reset the fields when a 
/// user inputs data into the validation parameter fields and then changes data type.
fn default_additional_properties(data_type: &str) -> Property {
    match data_type {
        "String" => Property {
            data_type: DataType::String,
            min_length: None,
            max_length: None,
            pattern: None,
            format: None,
            ..Default::default()
        },
        "Integer" => Property {
            data_type: DataType::Integer,
            minimum: None,
            maximum: None,
            ..Default::default()
        },
        "Array" => Property {
            data_type: DataType::Array,
            byte_array: Some(true),
            min_items: None,
            max_items: None,
            content_media_type: None,
            ..Default::default()
        },
        "Object" => Property {
            data_type: DataType::Object,
            min_properties: None,
            max_properties: None,
            ..Default::default()
        },
        "Number" => Property {
            data_type: DataType::Number,
            ..Default::default()
        },
        "Boolean" => Property {
            data_type: DataType::Boolean,
            ..Default::default()
        },
        _ => panic!("Invalid data type selected"),
    }
}

// Contains functions that generate the webpage and json object
impl Model {

    fn view_document_types(&self, ctx: &yew::Context<Self>) -> Html {
        html! {
            <div>
                {for (0..self.document_types.len()).map(|i| self.view_document_type(i, ctx))}
            </div>
        }
    }

    fn view_document_type(&self, index: usize, ctx: &yew::Context<Self>) -> Html {
        html! {
            <>
            <div class="input-container">
                <div>
                    <h2>{format!("Document type {}", index+1)}</h2>
                    <h3>{"Name"}</h3>
                    <input type="text" placeholder="Name" value={self.document_types[index].name.clone()} onblur={ctx.link().callback(move |e: FocusEvent| Msg::UpdateName(index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                </div>
                <div>
                <div class="form-line">
                    <h3>{"Properties"}</h3>
                            {for (0..self.document_types[index].properties.len()).map(|i| self.view_property(index, i, ctx))}
                            <button class="button property" onclick={ctx.link().callback(move |_| Msg::AddProperty(index))}><span class="plus">{"+"}</span>{"Add property"}</button>
                   </div>
                            
                        
                            <div class="forms-line-checkboxes">
                                <label>{"Require $createdAt:   "}</label>
                                <input type="checkbox" id="toggle1" class="toggle-input" checked={self.document_types[index].created_at_required} onchange={ctx.link().callback(move |e: Event| Msg::UpdateSystemPropertiesRequired(index, 0, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
                                <label for="toggle1" class="toggle-label"></label>
                            </div>
                            <div class="forms-line-checkboxes">
                                <label>{"Require $updatedAt:   "}</label>
                                <input type="checkbox" id="toggle2" class="toggle-input" checked={self.document_types[index].updated_at_required} onchange={ctx.link().callback(move |e: Event| Msg::UpdateSystemPropertiesRequired(index, 1, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
                                <label for="toggle2" class="toggle-label"></label>
                            </div>                        
                    
                </div>
                <div>
                    <h3>{"Indices"}</h3>
                    
                        
                            {for (0..self.document_types[index].indices.len()).map(|i| self.view_index(index, i, ctx))}
                            <div class="forms-line">
                                <button class="button property" onclick={ctx.link().callback(move |_| Msg::AddIndex(index))}><span class="plus">{"+"}</span>{"Add index"}</button>
                </div>                        
                    
                </div>
                <div>
                    <h3>{"Comment"}</h3>
                    <input type="text2" placeholder="Comment" value={self.document_types[index].comment.clone()} onblur={ctx.link().callback(move |e: FocusEvent| Msg::UpdateComment(index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                </div>
                <br/>
                <div>
                <button class="button remove-text" onclick={ctx.link().callback(move |_| Msg::RemoveDocumentType(index))}>{format!("Remove document type {}", index+1)}</button>
                </div>
            </div>
            <br/>
            </>
        }
    }

    fn view_property(&self, doc_index: usize, prop_index: usize, ctx: &yew::Context<Self>) -> Html {
        let data_type_options = vec!["String", "Integer", "Array", "Object", "Number", "Boolean"];
        let selected_data_type = match self.document_types[doc_index].properties[prop_index].data_type {
            DataType::String => String::from("String"),
            DataType::Integer => String::from("Integer"),
            DataType::Array => String::from("Array"),
            DataType::Object => String::from("Object"),
            DataType::Number => String::from("Number"),
            DataType::Boolean => String::from("Boolean"),
        };
        let additional_properties = self.render_additional_properties(&selected_data_type, doc_index, prop_index, ctx);
        html! {
            <>
                <div class="forms-line-names">
                 <div class="form-headers">
                  <label>{format!("Property {} name", prop_index+1)}</label>
                  <input type="text3" placeholder={format!("Property {} name", prop_index+1)} value={self.document_types[doc_index].properties[prop_index].name.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePropertyName(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                 </div>
                 <div class="form-headers">
                  <label>{"Type"}</label>
                  <select value={selected_data_type.clone()} onchange={ctx.link().callback(move |e: Event| {
                    let selected_data_type = e.target_dyn_into::<HtmlSelectElement>().unwrap().value();
                    let new_property = default_additional_properties(selected_data_type.as_str());
                    Msg::UpdatePropertyType(doc_index, prop_index, new_property)
                })}>
                    {for data_type_options.iter().map(|option| html! {
                        <option value={String::from(*option)} selected={&String::from(*option)==&selected_data_type}>{String::from(*option)}</option>
                    })}
                  </select>
                 </div>
                 <div class="checkbox-block form-headers">
                  <label>{"Required"}</label>
                  <input type="checkbox" id="toggle" class="toggle-input"  checked={self.document_types[doc_index].properties[prop_index].required} onchange={ctx.link().callback(move |e: Event| Msg::UpdatePropertyRequired(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
                  <label for="toggle" class="toggle-label"></label>
                 </div>
                 <button class="button remove" onclick={ctx.link().callback(move |_| Msg::RemoveProperty(doc_index, prop_index))}>{"X"}</button>
                </div>
                <p><b>{if selected_data_type != String::from("Object") { "Optional property parameters:" } else {""}}</b></p>
                <div class="forms-line">
                        {additional_properties}
                        <div class="forms-line">
                            <label>{"Description: "}</label>
                            <input type="text3" value={self.document_types[doc_index].properties[prop_index].description.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePropertyDescription(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
            </div>                        <div class="forms-line">
                            <label>{"Comment: "}</label>
                            <input type="text3" value={self.document_types[doc_index].properties[prop_index].comment.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePropertyComment(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
            </div>                        <p></p>
                    
    </div>            
    </>
        }
    }

    fn render_additional_properties(&self, data_type: &String, doc_index: usize, prop_index: usize, ctx: &yew::Context<Self>) -> Html {
        let property = &self.document_types[doc_index].properties[prop_index];
        match data_type.as_str() {
            "String" => html! {
                <>
                <div class="forms-line">
                    <label>{"Min length: "}</label>
                    <input type="number" value={property.min_length.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<u32>().unwrap_or_default())
                        };
                        Msg::UpdateStringPropertyMinLength(doc_index, prop_index, num_value)
                    })} />
                    
                </div>
                <div class="forms-line">
                    <label>{"Max length: "}</label>
                    <input type="number" value={property.max_length.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<u32>().unwrap_or_default())
                        };
                        Msg::UpdateStringPropertyMaxLength(doc_index, prop_index, num_value)
                    })} />
                    
                </div>
                <div class="forms-line">
                    <label>{"RE2 pattern: "}</label>
                    <input type="text3" value={property.pattern.clone().unwrap_or_default()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringPropertyPattern(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                </div>
                <div class="forms-line">
                    <label>{"Format: "}</label>
                    <input type="text3" value={property.format.clone().unwrap_or_default()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringPropertyFormat(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                </div>
                </>
            },
            "Integer" => html! {
                <>
                <div class="forms-line">
                    <label>{"Minimum: "}</label>
                    <input type="number" value={property.minimum.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<i32>().unwrap_or_default())
                        };
                        Msg::UpdateIntegerPropertyMinimum(doc_index, prop_index, num_value)
                    })} />
                    
                </div>
                <div class="forms-line">
                    <label>{"Maximum: "}</label>
                    <input type="number" value={property.maximum.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<i32>().unwrap_or_default())
                        };
                        Msg::UpdateIntegerPropertyMaximum(doc_index, prop_index, num_value)
                    })} />
                                  </div>
                </>
            },
            "Array" => html! {
                <>
                /* <div class="forms-line">
                    <label>{"Byte array: "}</label>
                    <input type="checkbox" checked={property.byte_array.unwrap_or(false)} onchange={ctx.link().callback(move |e: Event| Msg::UpdateArrayPropertyByteArray(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
    </div>*/
                <div class="forms-line">
                    <label>{"Min items: "}</label>
                    <input type="number" value={property.min_items.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<u32>().unwrap_or_default())
                        };
                        Msg::UpdateArrayPropertyMinItems(doc_index, prop_index, num_value)
                    })} />
                    
                </div>
                <div class="forms-line">
                    <label>{"Max items: "}</label>
                    <input type="number" value={property.max_items.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<u32>().unwrap_or_default())
                        };
                        Msg::UpdateArrayPropertyMaxItems(doc_index, prop_index, num_value)
                    })} />
                   
                </div>
                <div class="forms-line">
                    <label>{"Content media type: "}</label>
                    <input type="text3" value={property.content_media_type.clone().unwrap_or_default()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateArrayPropertyCMT(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                </div>
                </>
            },
            "Object" => html! {
                <>
                <div class="forms-line">
                    
                    {for self.document_types[doc_index].properties[prop_index].properties.as_ref().unwrap_or(&Box::new(Vec::new())).iter().enumerate().map(|(i, _)| self.view_recursive_property(doc_index, prop_index, i, ctx))}
                    
                </div>
                <div class="forms-line">
                    <button class="button" onclick={ctx.link().callback(move |_| Msg::AddRecProperty(doc_index, prop_index))}>{"Add inner property"}</button>
                </div>
                <p><b>{"Optional property parameters:"}</b></p>
                <div class="forms-line">
                    <label>{"Min properties: "}</label>
                    <input type="number" value={property.min_properties.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<u32>().unwrap_or_default())
                        };
                        Msg::UpdateObjectPropertyMinProperties(doc_index, prop_index, num_value)
                    })} />
                </div>
                <div class="forms-line">
                    <label>{"Max properties: "}</label>
                    <input type="number" value={property.max_properties.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                        let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                        let num_value = if value.is_empty() {
                            None
                        } else {
                            Some(value.parse::<u32>().unwrap_or_default())
                        };
                        Msg::UpdateObjectPropertyMaxProperties(doc_index, prop_index, num_value)
                    })} />
                </div>
                </>
            },
            "Number" => html! {
                <>
                </>
            },
            "Boolean" => html! {
                <>
                </>
            },
            _ => html! {},
        }
    }

    fn view_recursive_property(&self, doc_index: usize, prop_index: usize, recursive_prop_index: usize, ctx: &yew::Context<Self>) -> Html {
        let data_type_options = vec!["String", "Integer", "Array", "Object", "Number", "Boolean"];
        let selected_data_type = match &self.document_types[doc_index].properties[prop_index].properties.clone() {
            Some(properties) => match properties.get(recursive_prop_index) {
                Some(property) => match property.data_type {
                    DataType::String => String::from("String"),
                    DataType::Integer => String::from("Integer"),
                    DataType::Array => String::from("Array"),
                    DataType::Object => String::from("Object"),
                    DataType::Number => String::from("Number"),
                    DataType::Boolean => String::from("Boolean"),
                },
                None => return html! {<>{"oops1"}</>},
            },
            None => return html! {<>{"oops2"}</>},
        };
    
        html! {
            <>
                //<><b>{format!("Inner property {}:", recursive_prop_index+1)}</b></><br/><br/>
                <div class="forms-line">
                    {format!("Inner property {} name", recursive_prop_index+1)}
                    {"Type"}
                    {"Required"}
                </div>
                <div class="forms-line">
                    
                        <input type="text3" placeholder={format!("Inner property {} name", recursive_prop_index+1)} value={match &self.document_types[doc_index].properties[prop_index].properties {
                            Some(properties) => properties.get(recursive_prop_index).map(|property| property.name.clone()).unwrap_or_default(),
                            None => String::new(),
                        }} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateRecPropertyName(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                  
                        <select value={selected_data_type.clone()} onchange={ctx.link().callback(move |e: Event| Msg::UpdateRecPropertyType(doc_index, prop_index, recursive_prop_index, match e.target_dyn_into::<HtmlSelectElement>().unwrap().value().as_str() {
                            "String" => String::from("String"),
                            "Integer" => String::from("Integer"),
                            "Array" => String::from("Array"),
                            "Object" => String::from("Object"),
                            "Number" => String::from("Number"),
                            "Boolean" => String::from("Boolean"),
                            _ => panic!("Invalid data type selected"),
                        }))}>
                            {for data_type_options.iter().map(|option| html! {
                                <option value={String::from(*option)} selected={&String::from(*option)==&selected_data_type}>{String::from(*option)}</option>
                            })}
                        </select>
                        <input type="checkbox" id="toggle3" class="toggle-input" checked={match &self.document_types[doc_index].properties[prop_index].properties {
                            Some(properties) => properties.get(recursive_prop_index).map(|property| property.required).unwrap_or(false),
                            None => false,
                        }} onchange={ctx.link().callback(move |e: Event| Msg::UpdateRecPropertyRequired(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
                        <label for="toggle3" class="toggle-label"></label>
                        <button class="button" onclick={ctx.link().callback(move |_| Msg::RemoveRecProperty(doc_index, prop_index, recursive_prop_index))}>{"Remove"}</button>
                </div>
                <p><b>{"Optional property parameters:"}</b></p>
                <div class="forms-line">
            
                            {self.rec_render_additional_properties(&selected_data_type, doc_index, prop_index, recursive_prop_index, ctx)}
                            
                                <label>{"Description: "}</label>
                                <input type="text3" value={if let Some(properties) = &self.document_types.get(doc_index).and_then(|doc| doc.properties.get(prop_index).and_then(|prop| prop.properties.clone())) {
                                    properties.get(recursive_prop_index).and_then(|prop| prop.description.clone()).unwrap_or_default()
                                } else {
                                    "".to_string()
                                }} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateRecPropertyDescription(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                      <label>{"Comment: "}</label><input type="text3" value={if let Some(properties) = &self.document_types.get(doc_index).and_then(|doc| doc.properties.get(prop_index).and_then(|prop| prop.properties.clone())) {
                                    properties.get(recursive_prop_index).and_then(|prop| prop.comment.clone()).unwrap_or_default()
                                } else {
                                    "".to_string()
                                }} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateRecPropertyComment(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                            </div>
                            <p></p>
    
            </>
        }
    }

    fn rec_render_additional_properties(&self, data_type: &String, doc_index: usize, prop_index: usize, recursive_prop_index: usize, ctx: &yew::Context<Self>) -> Html {
        let properties = self.document_types[doc_index].properties.get(prop_index).and_then(|p| p.properties.as_ref());
        match data_type.as_str() {
            "String" => {
                let min_length = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.min_length);
                let max_length = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.max_length);
                let pattern = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.pattern.clone());
                let format = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.format.clone());
                html! {
                    <>
                    <div class="forms-line">
                        <label>{"Min length: "}</label>
                        <input type="number" value={min_length.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<u32>().unwrap_or_default())
                            };
                            Msg::UpdateStringRecPropertyMinLength(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                        
                    </div>
                    <div class="forms-line">
                        <label>{"Max length: "}</label>
                        <input type="number" value={max_length.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<u32>().unwrap_or_default())
                            };
                            Msg::UpdateStringRecPropertyMaxLength(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                    </div>
                    <div class="forms-line">
                        <label>{"RE2 pattern: "}</label>
                        <input type="text3" value={pattern} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringRecPropertyPattern(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                    </div>
                    <div class="forms-line">
                        <label>{"Format: "}</label>
                        <input type="text3" value={format} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringRecPropertyFormat(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                    </div>
                    </>
                }
            },
            "Integer" => {
                let minimum = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.minimum);
                let maximum = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.maximum);
                html! {
                    <>
                    <div class="forms-line">
                        <label>{"Minimum: "}</label>
                        <input type="number" value={minimum.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<i32>().unwrap_or_default())
                            };
                            Msg::UpdateIntegerRecPropertyMinimum(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                    </div>
                    <div class="forms-line">
                        <label>{"Maximum: "}</label>
                        <input type="number" value={maximum.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<i32>().unwrap_or_default())
                            };
                            Msg::UpdateIntegerRecPropertyMaximum(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                    </div>
                    </>
                }
            },
            "Array" => {
                //let byte_array = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.byte_array);
                let max_items = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.max_items);
                let min_items = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.min_items);
                let content_media_type = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.content_media_type.clone());
                html! {
                    <>
                    /* <div class="forms-line">
                        <label>{"Byte array: "}</label>
                        <input type="checkbox" checked={byte_array.unwrap_or(false)} onchange={ctx.link().callback(move |e: Event| Msg::UpdateArrayRecPropertyByteArray(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
        </div>*/
                    <div class="forms-line">
                        <label>{"Min items: "}</label>
                        <input type="number" value={min_items.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<u32>().unwrap_or_default())
                            };
                            Msg::UpdateArrayRecPropertyMinItems(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                    </div>
                    <div class="forms-line">
                        <label>{"Max items: "}</label>
                        <input type="number" value={max_items.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<u32>().unwrap_or_default())
                            };
                            Msg::UpdateArrayRecPropertyMaxItems(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                    </div>
                    <div class="forms-line">
                        <label>{"Content media type: "}</label>
                        <input type="text3" value={content_media_type} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateArrayRecPropertyCMT(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                    </div>
                    </>
                }
            },            
            "Object" => {
                let min_props = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.min_properties);
                let max_props = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.max_properties);
                html! {
                    <>
                    <div class="forms-line">
                        <label>{"Min properties: "}</label>
                        <input type="number" value={min_props.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<u32>().unwrap_or_default())
                            };
                            Msg::UpdateObjectRecPropertyMinProperties(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                        
                    </div>
                    <div class="forms-line">
                        <label>{"Max properties: "}</label>
                        <input type="number" value={max_props.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value();
                            let num_value = if value.is_empty() {
                                None
                            } else {
                                Some(value.parse::<u32>().unwrap_or_default())
                            };
                            Msg::UpdateObjectRecPropertyMaxProperties(doc_index, prop_index, recursive_prop_index, num_value)
                        })} />
                        
                    </div>
                    </>
                }
            },
            "Number" => html! {
                <>
                </>
            },
            "Boolean" => html! {
                <>
                </>
            },
            _ => html! {},
        }
    }

    fn view_index(&self, doc_index: usize, index_index: usize, ctx: &yew::Context<Self>) -> Html {
        html! {
            <>
            <div class="forms-line">
                <th>{format!("Index {} name", index_index+1)}</th>
                <th>{"Unique"}</th>
                <th>{""}</th>
</div>            <div class="forms-line">
                <input type="text3" placeholder={format!("Index {} name", index_index+1)} value={self.document_types[doc_index].indices[index_index].name.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIndexName(doc_index, index_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                <input type="checkbox" id="toggle4" class="toggle-input" checked={self.document_types[doc_index].indices[index_index].unique} onchange={ctx.link().callback(move |e: Event| Msg::UpdateIndexUnique(doc_index, index_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
                <label for="toggle4" class="toggle-label"></label>
                <button class="button" onclick={ctx.link().callback(move |_| Msg::RemoveIndex(doc_index, index_index))}>{"Remove"}</button>
</div>            <div class="forms-line">
                
                    
                        
                            <p><b>{"Index properties:"}</b></p>
                            <div>{for (0..self.document_types[doc_index].indices[index_index].properties.len()).map(|i| self.view_index_properties(doc_index, index_index, i, ctx))}</div>
                        
                    
                
</div>            <p></p>
            <div class="forms-line">
                <button class="button" onclick={ctx.link().callback(move |_| Msg::AddIndexProperty(doc_index, index_index))}>{"Add index property"}</button>
</div>            <p></p>
            </>
        }
    }    

    fn view_index_properties(&self, doc_index: usize, index_index: usize, prop_index: usize, ctx: &yew::Context<Self>) -> Html {
        let sorting_options = vec!["Ascending", "Descending"];
        let mut current_sort = sorting_options[0];
        if self.document_types[doc_index].indices[index_index].properties[prop_index].1.clone() == String::from("desc") {
            current_sort = sorting_options[1];
        }
        html!(
            <div class="forms-line">
                {format!("Property {}: ", prop_index+1)}
                <input type="text3" value={self.document_types[doc_index].indices[index_index].properties[prop_index].0.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIndexProperty(doc_index, index_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                <select value={current_sort} onchange={ctx.link().callback(move |e: Event| Msg::UpdateIndexSorting(doc_index, index_index, prop_index, match e.target_dyn_into::<HtmlSelectElement>().unwrap().value().as_str() {
                    "Ascending" => String::from("asc"),
                    "Descending" => String::from("desc"),
                    _ => panic!("Invalid data type selected"),
                }))}>
                    {for sorting_options.iter().map(|option| html! {
                        <option value={String::from(*option)} selected={&String::from(*option)==current_sort}>{String::from(*option)}</option>
                    })}
                </select>
</div>        
            )
    }

    fn generate_json_object(&mut self) -> Vec<String> {
        let mut json_arr = Vec::new();
        for doc_type in &mut self.document_types {
            let mut props_map = Map::new();
            for prop in &mut doc_type.properties {
                let mut prop_obj = Map::new();
                prop_obj.insert("type".to_owned(), json!(match prop.data_type {
                    DataType::String => "string",
                    DataType::Integer => "integer",
                    DataType::Array => "array",
                    DataType::Object => "object",
                    DataType::Number => "number",
                    DataType::Boolean => "boolean",
                }));
                if prop.description.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    prop_obj.insert("description".to_owned(), json!(prop.description));
                }
                if prop.min_length.is_some() {
                    prop_obj.insert("minLength".to_owned(), json!(prop.min_length));
                }
                if prop.max_length.is_some() {
                    prop_obj.insert("maxLength".to_owned(), json!(prop.max_length));
                }
                if prop.pattern.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    prop_obj.insert("pattern".to_owned(), json!(prop.pattern));
                }
                if prop.format.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    prop_obj.insert("format".to_owned(), json!(prop.format));
                }
                if prop.minimum.is_some() {
                    prop_obj.insert("minimum".to_owned(), json!(prop.minimum));
                }
                if prop.maximum.is_some() {
                    prop_obj.insert("maximum".to_owned(), json!(prop.maximum));
                }
                if let Some(byte_array) = prop.byte_array {
                    prop_obj.insert("byteArray".to_owned(), json!(byte_array));
                }
                if prop.min_items.is_some() {
                    prop_obj.insert("minItems".to_owned(), json!(prop.min_items));
                }
                if prop.max_items.is_some() {
                    prop_obj.insert("maxItems".to_owned(), json!(prop.max_items));
                }
                if prop.content_media_type.is_some() {
                    prop_obj.insert("contentMediaType".to_owned(), json!(prop.content_media_type));
                }
                if prop.data_type == DataType::Object {
                    let rec_props_map = Self::generate_nested_properties(prop);
                    prop_obj.insert("properties".to_owned(), json!(rec_props_map));
                    }
                if prop.min_properties.is_some() {
                    prop_obj.insert("minProperties".to_owned(), json!(prop.min_properties));
                }
                if prop.max_properties.is_some() {
                    prop_obj.insert("maxProperties".to_owned(), json!(prop.max_properties));
                }
                if prop.rec_required.as_ref().map(|c| c.len()).unwrap_or_default() > 0 {
                    prop_obj.insert("required".to_owned(), json!(prop.rec_required));
                }
                if prop.data_type == DataType::Object {
                    prop_obj.insert("additionalProperties".to_owned(), json!(false));
                }
                if prop.comment.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    prop_obj.insert("$comment".to_owned(), json!(prop.comment));
                }
                props_map.insert(prop.name.clone(), json!(prop_obj));
                if prop.required {
                    if !doc_type.required.contains(&prop.name) {
                        doc_type.required.push(prop.name.clone());
                    }
                } else {
                    if doc_type.required.contains(&prop.name) {
                        doc_type.required.retain(|x| x != &prop.name);
                    }
                }
            }
            let mut indices_arr = Vec::new();
            for index in &doc_type.indices {
                if index.unique {
                    let index_obj = json!({
                        "name": index.name,
                        "properties": index.properties.iter().map(|inner_tuple| {
                            let mut inner_obj = Map::new();
                            inner_obj.insert(inner_tuple.0.clone(), json!(inner_tuple.1));
                            json!(inner_obj)
                        }).collect::<Vec<_>>(),
                        "unique": index.unique,
                    });
                    indices_arr.push(index_obj);
                } else {
                    let index_obj = json!({
                        "name": index.name,
                        "properties": index.properties.iter().map(|inner_tuple| {
                            let mut inner_obj = Map::new();
                            inner_obj.insert(inner_tuple.0.clone(), json!(inner_tuple.1));
                            json!(inner_obj)
                        }).collect::<Vec<_>>(),
                    });
                    indices_arr.push(index_obj);
                }
            }
            let mut doc_obj = Map::new();
            doc_obj.insert("type".to_owned(), json!("object"));
            doc_obj.insert("properties".to_owned(), json!(props_map));
            if !doc_type.indices.is_empty() {
                doc_obj.insert("indices".to_owned(), json!(indices_arr));
            }
            if doc_type.created_at_required && !doc_type.required.contains(&String::from("$createdAt")) {
                doc_type.required.push("$createdAt".to_string());
            }
            if doc_type.required.contains(&String::from("$createdAt")) && !doc_type.created_at_required {
                doc_type.required.retain(|x| x != &String::from("$createdAt"))
            }
            if doc_type.updated_at_required && !doc_type.required.contains(&String::from("$updatedAt")) {
                doc_type.required.push("$updatedAt".to_string());
            }
            if doc_type.required.contains(&String::from("$updatedAt")) && !doc_type.updated_at_required {
                doc_type.required.retain(|x| x != &String::from("$updatedAt"))
            }
            if !doc_type.required.is_empty() {
                doc_obj.insert("required".to_owned(), json!(doc_type.required));
            }
            doc_obj.insert("additionalProperties".to_owned(), json!(false));
            if doc_type.comment.len() > 0 {
                doc_obj.insert("$comment".to_owned(), json!(doc_type.comment));
            }
            let final_doc_obj = json!({
                doc_type.name.clone(): doc_obj
            });
            let formatted_doc_obj = &final_doc_obj.to_string()[1..final_doc_obj.to_string().len()-1];
            json_arr.push(formatted_doc_obj.to_string());
        }
        let s = json_arr.join(",");
        self.schema = format!("{{{}}}", s);
        json_arr
    }    

    fn generate_nested_properties(prop: &mut Property) -> Map<String, Value> {
        let mut rec_props_map = Map::new();
        if let Some(nested_props) = &mut prop.properties {
            for rec_prop in nested_props.iter_mut() {
                let mut rec_prop_obj = Map::new();
                rec_prop_obj.insert("type".to_owned(), json!(match rec_prop.data_type {
                    DataType::String => "string",
                    DataType::Integer => "integer",
                    DataType::Array => "array",
                    DataType::Object => "object",
                    DataType::Number => "number",
                    DataType::Boolean => "boolean",
                }));
                if rec_prop.description.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("description".to_owned(), json!(rec_prop.description));
                }
                if rec_prop.min_length.is_some() {
                    rec_prop_obj.insert("minLength".to_owned(), json!(rec_prop.min_length));
                }
                if rec_prop.max_length.is_some() {
                    rec_prop_obj.insert("maxLength".to_owned(), json!(rec_prop.max_length));
                }
                if rec_prop.pattern.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("pattern".to_owned(), json!(rec_prop.pattern));
                }
                if rec_prop.format.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("format".to_owned(), json!(rec_prop.format));
                }
                if rec_prop.minimum.is_some() {
                    rec_prop_obj.insert("minimum".to_owned(), json!(rec_prop.minimum));
                }
                if rec_prop.maximum.is_some() {
                    rec_prop_obj.insert("maximum".to_owned(), json!(rec_prop.maximum));
                }
                if let Some(byte_array) = rec_prop.byte_array {
                    rec_prop_obj.insert("byteArray".to_owned(), json!(byte_array));
                }
                if rec_prop.min_items.is_some() {
                    rec_prop_obj.insert("minItems".to_owned(), json!(rec_prop.min_items));
                }
                if rec_prop.max_items.is_some() {
                    rec_prop_obj.insert("maxItems".to_owned(), json!(rec_prop.max_items));
                }
                if rec_prop.content_media_type.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("contentMediaType".to_owned(), json!(rec_prop.content_media_type));
                }
                if rec_prop.data_type == DataType::Object {
                    rec_prop_obj.insert("properties".to_owned(), json!({"some_property":{"type": "string"}}));
                }
                if rec_prop.min_properties.is_some() {
                    rec_prop_obj.insert("minProperties".to_owned(), json!(rec_prop.min_properties));
                }
                if rec_prop.max_properties.is_some() {
                    rec_prop_obj.insert("maxProperties".to_owned(), json!(rec_prop.max_properties));
                }
                if rec_prop.data_type == DataType::Object {
                    rec_prop_obj.insert("additionalProperties".to_owned(), json!(false));
                }
                if rec_prop.comment.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("$comment".to_owned(), json!(rec_prop.comment));
                }
                rec_props_map.insert(rec_prop.name.clone(), json!(rec_prop_obj));
                if rec_prop.required {
                    if !prop.rec_required.as_ref().cloned().unwrap_or_default().contains(&rec_prop.name) {
                        prop.rec_required.get_or_insert_with(Vec::new).push(rec_prop.name.clone());
                    }
                } else {
                    if prop.rec_required.as_ref().map_or(false, |req| req.contains(&rec_prop.name)) {
                        prop.rec_required.as_mut().map(|v| v.retain(|x| x != &rec_prop.name));
                    }
                }
                rec_props_map.insert(rec_prop.name.clone(), json!(rec_prop_obj));
            }
        }
        rec_props_map
    }

    fn parse_imported_json(&mut self) {

        // Parse the string into a HashMap
        let parsed_json: HashMap<String, Value> = serde_json::from_str(&self.imported_json).unwrap_or_default();

        // Convert the HashMap into a Vec of Strings for json_object
        self.json_object = parsed_json.iter().map(|(k, v)| {
            format!("\"{}\":{}", k, v.to_string())
        }).collect();

        // Empty self.document_types
        self.document_types = Vec::new();

        // Iterate over each key-value pair in the parsed JSON and push to document_types
        for (doc_type_name, doc_type_value) in parsed_json {
            // Create a new default DocumentType and set its name
            let mut document_type = DocumentType::default();
            document_type.name = doc_type_name;

            // Check if value is an object
            if let Some(doc_type_obj) = doc_type_value.as_object() {
                // Check if $createdAt or $updatedAt are required
                if let Some(required) = doc_type_obj.get("required") {
                    if let Some(required_array) = required.as_array() {
                        document_type.created_at_required = required_array.contains(&Value::String("$createdAt".to_string()));
                        document_type.updated_at_required = required_array.contains(&Value::String("$updatedAt".to_string()));
                    }
                }
                // Iterate over properties
                if let Some(properties) = doc_type_obj.get("properties") {
                    if let Some(properties_obj) = properties.as_object() {
                        for (prop_name, prop_value) in properties_obj {
                            // Create a new default Property and set its name
                            let mut property = Property::default();
                            property.name = prop_name.to_string();

                            if let Some(required) = doc_type_obj.get("required") {
                                if let Some(required_array) = required.as_array() {
                                    if required_array.iter().any(|v| *v == Value::String(prop_name.clone())) {
                                        property.required = true;
                                    }
                                }
                            }

                            // Check if property value is an object
                            if let Some(prop_obj) = prop_value.as_object() {
                                // Set the Property.data_type to the value of "type"
                                if let Some(data_type) = prop_obj.get("type") {
                                    property.data_type = match data_type.as_str().unwrap() {
                                        "string" => DataType::String,
                                        "integer" => DataType::Integer,
                                        "array" => DataType::Array,
                                        "object" => DataType::Object,
                                        "number" => DataType::Number,
                                        "boolean" => DataType::Boolean,
                                        _ => panic!("Unexpected type value"),
                                    };
                                }
                                if let Some(byte_array) = prop_obj.get("byteArray") {
                                    property.byte_array = byte_array.as_bool();
                                }
                                if let Some(description) = prop_obj.get("description") {
                                    property.description = description.as_str().map(|s| s.to_string());
                                }
                                if let Some(comment) = prop_obj.get("$comment") {
                                    property.comment = comment.as_str().map(|s| s.to_string());
                                }
                                if let Some(min_length) = prop_obj.get("minLength") {
                                    property.min_length = min_length.as_u64().map(|num| num as u32);
                                }
                                if let Some(max_length) = prop_obj.get("maxLength") {
                                    property.max_length = max_length.as_u64().map(|num| num as u32);
                                }
                                if let Some(pattern) = prop_obj.get("pattern") {
                                    property.pattern = pattern.as_str().map(|s| s.to_string());
                                }
                                if let Some(format) = prop_obj.get("format") {
                                    property.format = format.as_str().map(|s| s.to_string());
                                }
                                if let Some(minimum) = prop_obj.get("minimum") {
                                    property.minimum = minimum.as_i64().map(|num| num as i32);
                                }
                                if let Some(maximum) = prop_obj.get("maximum") {
                                    property.maximum = maximum.as_i64().map(|num| num as i32);
                                }
                                if let Some(min_items) = prop_obj.get("minItems") {
                                    property.min_items = min_items.as_u64().map(|num| num as u32);
                                }
                                if let Some(max_items) = prop_obj.get("maxItems") {
                                    property.max_items = max_items.as_u64().map(|num| num as u32);
                                }
                                if let Some(content_media_type) = prop_obj.get("contentMediaType") {
                                    property.content_media_type = content_media_type.as_str().map(|s| s.to_string());
                                }
                                if let Some(min_properties) = prop_obj.get("minProperties") {
                                    property.min_properties = min_properties.as_u64().map(|num| num as u32);
                                }
                                if let Some(max_properties) = prop_obj.get("maxProperties") {
                                    property.max_properties = max_properties.as_u64().map(|num| num as u32);
                                }
                                if let Some(nested_props) = prop_obj.get("properties") {
                                    if let Some(nested_props_map) = nested_props.as_object() {
                                        let mut nested_props_vec = Vec::new();
                                        for (nested_prop_name, nested_prop_value) in nested_props_map {
                                            let mut nested_property = Property::default();
                                            nested_property.name = nested_prop_name.clone();
                                            if let Some(rec_required) = prop_obj.get("required") {
                                                if let Some(rec_required_array) = rec_required.as_array() {
                                                    if rec_required_array.iter().any(|v| *v == Value::String(nested_prop_name.clone())) {
                                                        nested_property.required = true;
                                                    }
                                                }
                                            }
                                            if let Some(nested_prop_obj) = nested_prop_value.as_object() {
                                                if let Some(data_type) = nested_prop_obj.get("type") {
                                                    nested_property.data_type = match data_type.as_str().unwrap() {
                                                        "string" => DataType::String,
                                                        "integer" => DataType::Integer,
                                                        "array" => DataType::Array,
                                                        "object" => DataType::Object,
                                                        "number" => DataType::Number,
                                                        "boolean" => DataType::Boolean,
                                                        _ => panic!("Unexpected type value"),
                                                    };
                                                }
                                                if let Some(byte_array) = nested_prop_obj.get("byteArray") {
                                                    nested_property.byte_array = byte_array.as_bool();
                                                }
                                                if let Some(description) = nested_prop_obj.get("description") {
                                                    nested_property.description = description.as_str().map(|s| s.to_string());
                                                }
                                                if let Some(comment) = nested_prop_obj.get("$comment") {
                                                    nested_property.comment = comment.as_str().map(|s| s.to_string());
                                                }
                                                if nested_prop_obj.contains_key("minLength") {
                                                    nested_property.min_length = nested_prop_obj.get("minLength").and_then(|v| v.as_u64()).map(|num| num as u32);
                                                } else {
                                                    nested_property.min_length = None;
                                                }
                                                if let Some(max_length) = nested_prop_obj.get("maxLength") {
                                                    nested_property.max_length = max_length.as_u64().map(|num| num as u32);
                                                }
                                                if let Some(pattern) = nested_prop_obj.get("pattern") {
                                                    nested_property.pattern = pattern.as_str().map(|s| s.to_string());
                                                }
                                                if let Some(format) = nested_prop_obj.get("format") {
                                                    nested_property.format = format.as_str().map(|s| s.to_string());
                                                }
                                                if let Some(minimum) = nested_prop_obj.get("minimum") {
                                                    nested_property.minimum = minimum.as_i64().map(|num| num as i32);
                                                }
                                                if let Some(maximum) = nested_prop_obj.get("maximum") {
                                                    nested_property.maximum = maximum.as_i64().map(|num| num as i32);
                                                }
                                                if let Some(min_items) = nested_prop_obj.get("minItems") {
                                                    nested_property.min_items = min_items.as_u64().map(|num| num as u32);
                                                }
                                                if let Some(max_items) = nested_prop_obj.get("maxItems") {
                                                    nested_property.max_items = max_items.as_u64().map(|num| num as u32);
                                                }
                                                if let Some(content_media_type) = nested_prop_obj.get("contentMediaType") {
                                                    nested_property.content_media_type = content_media_type.as_str().map(|s| s.to_string());
                                                }
                                                if let Some(min_properties) = nested_prop_obj.get("minProperties") {
                                                    nested_property.min_properties = min_properties.as_u64().map(|num| num as u32);
                                                }
                                                if let Some(max_properties) = nested_prop_obj.get("maxProperties") {
                                                    nested_property.max_properties = max_properties.as_u64().map(|num| num as u32);
                                                }
                                                nested_props_vec.push(nested_property);
                                            }
                                        }
                                        property.properties = Some(Box::new(nested_props_vec));
                                    }
                                }
                            }
                            // Add the property to the DocumentType
                            document_type.properties.push(property);
                        }
                    }
                }

                // Iterate over indices
                if let Some(indices) = doc_type_obj.get("indices") {
                    if let Some(indices_array) = indices.as_array() {
                        for index_value in indices_array {
                            // Check if index value is an object
                            if let Some(index_obj) = index_value.as_object() {
                                // Create a new default Index
                                let mut index = Index::default();

                                // Set index name
                                if let Some(name) = index_obj.get("name") {
                                    index.name = name.as_str().unwrap().to_string();
                                }

                                // Set unique
                                if let Some(unique) = index_obj.get("unique") {
                                    index.unique = unique.as_bool().unwrap();
                                }

                                // Iterate over index properties
                                if let Some(properties) = index_obj.get("properties") {
                                    if let Some(properties_array) = properties.as_array() {
                                        for prop_value in properties_array {
                                            // Check if property value is an object
                                            if let Some(prop_obj) = prop_value.as_object() {
                                                // Create a new default IndexProperties
                                                let mut index_properties = IndexProperties::default();

                                                // Set index properties name and order
                                                for (name, order) in prop_obj {
                                                    index_properties.0 = name.to_string();
                                                    index_properties.1 = order.as_str().unwrap().to_string();
                                                }

                                                // Add index properties to the Index
                                                index.properties.push(index_properties);
                                            }
                                        }
                                    }
                                }

                                // Add the index to the DocumentType
                                document_type.indices.push(index);
                            }
                        }
                    }
        
                    // Process comment
                    if let Some(comment) = doc_type_obj.get("$comment") {
                        document_type.comment = comment.as_str().unwrap().to_string();
                    }
                }
        
                // Push to document_types
                self.document_types.push(document_type);
            }
        }
    }

    fn validate(&mut self) -> Vec<String> {
        let s = &self.json_object.join(",");
        let new_s = format!("{{{}}}", s);
        let json_obj: serde_json::Value = serde_json::from_str(&new_s).unwrap();
    
        let protocol_version_validator = dpp::version::ProtocolVersionValidator::default();
        let data_contract_validator = dpp::data_contract::validation::data_contract_validator::DataContractValidator::new(Arc::new(protocol_version_validator));
        let factory = dpp::data_contract::DataContractFactory::new(1, Arc::new(data_contract_validator));
        let owner_id = Identifier::random();
        let contract_result = factory
            .create(owner_id, json_obj.clone().into(), None, None);
    
        match contract_result {
            Ok(contract) => {
                let results = contract.data_contract.validate(&contract.data_contract.to_cleaned_object().unwrap()).unwrap_or_default();
                let errors = results.errors;
                self.extract_basic_error_messages(&errors)
            },
            Err(e) => {
                self.error_messages_ai.push(format!("{}", e));
                self.error_messages_ai.clone()
            }
        }
    }
    
    /// Extracts the BasicError messages, since they are the only ones we are interested in
    fn extract_basic_error_messages(&self, errors: &[ConsensusError]) -> Vec<String> {
        let messages: Vec<String> = errors
            .iter()
            .filter_map(|error| {
                if let ConsensusError::BasicError(inner) = error {
                    if let dpp::errors::consensus::basic::basic_error::BasicError::JsonSchemaError(json_error) = inner {
                        Some(format!("JsonSchemaError: {}, Path: {}", json_error.error_summary().to_string(), json_error.instance_path().to_string()))
                    } else { 
                        Some(format!("{}", inner)) 
                    }
                } else {
                    None
                }
            })
            .collect();
    
        let messages: HashSet<String> = messages.into_iter().collect();
        let messages: Vec<String> = messages.into_iter().collect();
    
        messages
    }
}

/// Yew component functions
impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let mut default_document_type = DocumentType::default();
        default_document_type.properties.push(Property::default());
        Self {
            document_types: vec![default_document_type],
            json_object: Vec::new(),
            imported_json: String::new(),
            error_messages: Vec::new(),
            user_key: String::new(),
            prompt: String::new(),
            schema: String::new(),
            history: Vec::new(),
            temp_prompt: None,
            loading: false,
            error_messages_ai: Vec::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            // General
            Msg::AddDocumentType => {
                let mut new_document_type = DocumentType::default();
                new_document_type.properties.push(Property::default());
                self.document_types.push(new_document_type);
            }
            Msg::AddProperty(index) => {
                self.document_types[index].properties.push(Default::default());
            }
            Msg::AddIndex(index) => {
                self.document_types[index].indices.push(Index {
                    name: String::new(),
                    unique: false,
                    properties: vec![IndexProperties::default()],
                });
            }
            Msg::RemoveDocumentType(index) => {
                self.document_types.remove(index);
            }
            Msg::RemoveProperty(doc_index, prop_index) => {
                let name = self.document_types[doc_index].properties[prop_index].name.clone();
                let required = &mut self.document_types[doc_index].required;
                if let Some(index) = required.iter().position(|x| x == &name) {
                    required.remove(index);
                }
                self.document_types[doc_index].properties.remove(prop_index);
            }
            Msg::RemoveIndex(doc_index, index_index) => {
                self.document_types[doc_index].indices.remove(index_index);
            }
            Msg::AddIndexProperty(doc_index, index_index) => {
                self.document_types[doc_index].indices[index_index].properties.push(Default::default());
            }
            Msg::Submit => {
                self.json_object = Some(self.generate_json_object()).unwrap();
                self.error_messages = Some(self.validate()).unwrap();
                self.imported_json = String::new();
            }
            Msg::UpdateName(index, name) => {
                self.document_types[index].name = name;
            }
            Msg::UpdateComment(index, comment) => {
                self.document_types[index].comment = comment;
            }
            Msg::UpdatePropertyName(doc_index, prop_index, name) => {
                self.document_types[doc_index].properties[prop_index].name = name;
            }
            Msg::UpdateIndexName(doc_index, index_index, name) => {
                self.document_types[doc_index].indices[index_index].name = name;
            }
            Msg::UpdateIndexProperty(doc_index, index_index, prop_index, prop) => {
                self.document_types[doc_index].indices[index_index].properties[prop_index].0 = prop;
            }
            Msg::UpdateIndexSorting(doc_index, index_index, prop_index, sorting) => {
                self.document_types[doc_index].indices[index_index].properties[prop_index].1 = sorting;
            }
            Msg::UpdatePropertyType(doc_index, prop_index, new_property) => {
                let prop = &mut self.document_types[doc_index].properties[prop_index];
                prop.data_type = new_property.data_type;
                prop.min_length = new_property.min_length;
                prop.max_length = new_property.max_length;
                prop.pattern = new_property.pattern;
                prop.format = new_property.format;
                prop.minimum = new_property.minimum;
                prop.maximum = new_property.maximum;
                prop.byte_array = new_property.byte_array;
                prop.min_items = new_property.min_items;
                prop.max_items = new_property.max_items;
                prop.min_properties = new_property.min_properties;
                prop.max_properties = new_property.max_properties;
            }
            Msg::UpdateIndexUnique(doc_index, index_index, unique) => {
                self.document_types[doc_index].indices[index_index].unique = unique;
            }
            Msg::UpdatePropertyRequired(doc_index, prop_index, required) => {
                self.document_types[doc_index].properties[prop_index].required = required;
            }
            Msg::UpdateSystemPropertiesRequired(doc_index, system_prop, required) => {
                match system_prop {
                    0 => self.document_types[doc_index].created_at_required = required,
                    1 => self.document_types[doc_index].updated_at_required = required,
                    _ => {}
                }
            }
            Msg::UpdatePropertyDescription(doc_index, prop_index, description) => {
                self.document_types[doc_index].properties[prop_index].description = Some(description);
            }
            Msg::UpdatePropertyComment(doc_index, prop_index, comment) => {
                self.document_types[doc_index].properties[prop_index].comment = Some(comment);
            }

            // Optional property parameters
            Msg::UpdateStringPropertyMinLength(doc_index, prop_index, min_length) => {
                self.document_types[doc_index].properties[prop_index].min_length = min_length;
            }
            Msg::UpdateStringPropertyMaxLength(doc_index, prop_index, max_length) => {
                self.document_types[doc_index].properties[prop_index].max_length = max_length;
            }
            Msg::UpdateStringPropertyPattern(doc_index, prop_index, pattern) => {
                self.document_types[doc_index].properties[prop_index].pattern = Some(pattern);
            }
            Msg::UpdateStringPropertyFormat(doc_index, prop_index, format) => {
                self.document_types[doc_index].properties[prop_index].format = Some(format);
            }
            Msg::UpdateIntegerPropertyMinimum(doc_index, prop_index, minimum) => {
                self.document_types[doc_index].properties[prop_index].minimum = minimum;
            }
            Msg::UpdateIntegerPropertyMaximum(doc_index, prop_index, maximum) => {
                self.document_types[doc_index].properties[prop_index].maximum = maximum;
            }
            /* Msg::UpdateArrayPropertyByteArray(doc_index, prop_index, byte_array) => {
                self.document_types[doc_index].properties[prop_index].byte_array = Some(byte_array);
            } */
            Msg::UpdateArrayPropertyMinItems(doc_index, prop_index, min_items) => {
                self.document_types[doc_index].properties[prop_index].min_items = min_items;
            }
            Msg::UpdateArrayPropertyMaxItems(doc_index, prop_index, max_items) => {
                self.document_types[doc_index].properties[prop_index].max_items = max_items;
            }
            Msg::UpdateArrayPropertyCMT(doc_index, prop_index, cmt) => {
                self.document_types[doc_index].properties[prop_index].content_media_type = Some(cmt);
            }
            Msg::UpdateObjectPropertyMinProperties(doc_index, prop_index, min_properties) => {
                self.document_types[doc_index].properties[prop_index].min_properties = min_properties;
            }
            Msg::UpdateObjectPropertyMaxProperties(doc_index, prop_index, max_properties) => {
                self.document_types[doc_index].properties[prop_index].max_properties = max_properties;
            }

            // Recursive properties
            Msg::AddRecProperty(doc_index, prop_index) => {
                let property = Property {
                    name: String::new(),
                    data_type: DataType::String,
                    required: false,
                    rec_required: Some(Vec::new()),
                    description: None,
                    comment: None,
                    properties: None,
                    additional_properties: None,
                    byte_array: None,
                    format: None,
                    pattern: None,
                    min_length: None,
                    max_length: None,
                    minimum: None,
                    maximum: None,
                    min_items: None,
                    max_items: None,
                    content_media_type: None,
                    min_properties: None,
                    max_properties: None,
                };
    
                let document_type = self.document_types.get_mut(doc_index);
                if let Some(document_type) = document_type {
                    if let Some(properties) = document_type.properties.get_mut(prop_index).and_then(|prop| prop.properties.as_mut()) {
                        properties.push(property);
                    } else {
                        document_type.properties[prop_index].properties = Some(Box::new(vec![property]));
                    }
                }
            }  
            Msg::RemoveRecProperty(doc_index, prop_index, rec_prop_index) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec.remove(rec_prop_index);
                }
            }
            Msg::UpdateRecPropertyName(doc_index, prop_index, rec_prop_index, name) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].name = name;
                }
            }
            Msg::UpdateRecPropertyType(doc_index, prop_index, rec_prop_index, data_type) => {
                let data_type = match data_type.as_str() {
                    "String" => DataType::String,
                    "Integer" => DataType::Integer,
                    "Array" => DataType::Array,
                    "Object" => DataType::Object,
                    "Number" => DataType::Number,
                    "Boolean" => DataType::Boolean,
                    _ => unreachable!(),
                };
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].data_type = data_type;
                }
            }
            Msg::UpdateRecPropertyRequired(doc_index, prop_index, rec_prop_index, required) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].required = required;
                }
            }
            Msg::UpdateRecPropertyDescription(doc_index, prop_index, rec_prop_index, description) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].description = Some(description);
                }
            }
            Msg::UpdateRecPropertyComment(doc_index, prop_index, rec_prop_index, comment) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].comment = Some(comment);
                }
            }
            Msg::UpdateStringRecPropertyMinLength(doc_index, prop_index, rec_prop_index, min_length) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].min_length = min_length;
                }
            }
            Msg::UpdateStringRecPropertyMaxLength(doc_index, prop_index, rec_prop_index, max_length) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].max_length = max_length;
                }
            }
            Msg::UpdateStringRecPropertyPattern(doc_index, prop_index, rec_prop_index, pattern) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].pattern = Some(pattern);
                }
            }
            Msg::UpdateStringRecPropertyFormat(doc_index, prop_index, rec_prop_index, format) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].format = Some(format);
                }
            }
            Msg::UpdateIntegerRecPropertyMaximum(doc_index, prop_index, rec_prop_index, maximum) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].maximum = maximum;
                }
            }
            Msg::UpdateIntegerRecPropertyMinimum(doc_index, prop_index, rec_prop_index, minimum) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].minimum = minimum;
                }
            }
            /* Msg::UpdateArrayRecPropertyByteArray(doc_index, prop_index, rec_prop_index, byte_array) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].byte_array = Some(byte_array);
                }
            } */
            Msg::UpdateArrayRecPropertyMinItems(doc_index, prop_index, rec_prop_index, min_items) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].min_items = min_items;
                }
            }
            Msg::UpdateArrayRecPropertyMaxItems(doc_index, prop_index, rec_prop_index, max_items) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].max_items = max_items;
                }
            }
            Msg::UpdateArrayRecPropertyCMT(doc_index, prop_index, rec_prop_index, cmt) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].content_media_type = Some(cmt);
                }
            }
            Msg::UpdateObjectRecPropertyMinProperties(doc_index, prop_index, rec_prop_index, min_props) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].min_properties = min_props;
                }
            }
            Msg::UpdateObjectRecPropertyMaxProperties(doc_index, prop_index, rec_prop_index, max_props) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].max_properties = max_props;
                }
            }

            // Import
            Msg::UpdateImportedJson(import) => {
                self.imported_json = import.clone();
                self.schema = import;
            }
            Msg::Import => {
                self.parse_imported_json();
                self.json_object = Some(self.generate_json_object()).unwrap();
                self.error_messages = Some(self.validate()).unwrap();
                self.imported_json = String::new();
            }
            Msg::Clear => {
                self.json_object = vec![];
                self.imported_json = String::new();
            }
            
            // OpenAI
            Msg::UpdateUserKey(key) => {
                self.user_key = key;
            },
            Msg::UpdatePrompt(val) => {
                self.prompt = val;
            },
            Msg::GenerateSchema => {

                let user_key = self.user_key.clone();

                // Prepend the appropriate default prompt
                let prompt = if self.schema.is_empty() {
                    let first_prompt_pre = FIRST_PROMPT_PRE.to_string();
                    format!("{}{}", first_prompt_pre, self.prompt)
                } else {
                    let second_prompt_pre = SECOND_PROMPT_PRE.to_string();
                    format!("{}{}\n\n{}", second_prompt_pre, self.schema, self.prompt)
                };
    
                // Save the prompt temporarily
                self.temp_prompt = Some(self.prompt.clone());

                // Reset AI error messages
                self.error_messages_ai = Vec::new();
        
                self.loading = true;

                let callback = ctx.link().callback(Msg::ReceiveSchema);
                spawn_local(async move {
                    let result = call_openai(&prompt, &user_key).await;
                    callback.emit(result);
                });
    
                // Clear the input field
                ctx.link().send_message(Msg::ClearInput);
            },
            Msg::ClearInput => {
                self.prompt.clear();
            },
            Msg::ReceiveSchema(result) => {
                match result {
                    Ok(schema) => {
                        // Get the saved prompt and add it to the history
                        if let Some(temp_prompt) = self.temp_prompt.take() {
                            self.history.push(temp_prompt);
                        }
            
                        self.schema = schema.clone();
                        self.imported_json = schema;
                        self.parse_imported_json();
                        self.json_object = Some(self.generate_json_object()).unwrap();
                        self.error_messages = Some(self.validate()).unwrap();
                        self.imported_json = String::new();

                    },
                    Err(err) => {
                        self.error_messages_ai = vec![err.to_string()];
                    },
                }
                self.loading = false;
            },
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let onsubmit = ctx.link().callback(|event: SubmitEvent| {
            event.prevent_default();
            Msg::GenerateSchema
        });

        let s = &self.json_object.join(",");
        let new_s = format!("{{{}}}", s);
        let json_obj: serde_json::Value = serde_json::from_str(&new_s).unwrap();
        let json_pretty = serde_json::to_string_pretty(&json_obj).unwrap();
                                
        let textarea = if self.json_object.len() != 0 {
            html! {
                <textarea class="textarea-no-whitespace" id="json_output" value={if self.json_object.len() != 0 as usize {
                    serde_json::to_string(&json_obj).unwrap()
                } else { 
                    "".to_string()
                }}>
                </textarea>
            }
        } else {
            html! {}
        };

        // html
        html! {
            <main class="home">
                <br/>
                <input type="password"
                    placeholder="Paste OpenAI API key here"
                    value={self.user_key.clone()}
                    oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateUserKey(e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}/>
                <body>
                    <div class="container_ai">
                        <div class="top-section_ai">
                            <img class="logo_ai" src="https://media.dash.org/wp-content/uploads/dash-logo.svg" alt="Dash logo" width="200" height="100" />
                            <h1 class="header_ai">{"Data Contract Creator"}</h1>
                        </div>
                        <div class="content-container_ai">
                            <div class="input-container_ai">
                                <form onsubmit={onsubmit} class="form-container_ai">
                                    <div class="input-button-container_ai">
                                        <input
                                            placeholder={
                                                if self.schema.is_empty() {
                                                    "Describe your app here"
                                                } else {
                                                    "Describe any adjustments here"
                                                }
                                            }
                                            value={self.prompt.clone()}
                                            oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePrompt(e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}
                                        />
                                        <button type="submit">{"Generate"}</button>
                                    </div>
                                </form>
                                {
                                    if self.loading {
                                        html! {
                                            <div class="loader_ai"></div>
                                        }
                                    } else {
                                        html! {}
                                    }
                                }                                 
                            </div>
                            <div class="error-text_ai">
                                {
                                    if self.error_messages_ai.clone() != self.error_messages.clone() {
                                        self.error_messages_ai.clone()
                                    } else {
                                        vec!["".to_string()]
                                    }
                                }
                            </div>
                            <br/><br/>
                        </div>
                    </div>
                    <div class="columns">
                    <div class="column-left">

                        // show input fields
                        {self.view_document_types(ctx)}

                        <div class="button-container">
                            // add input fields for another document type and add one to Self::document_types
                            <button class="button2" onclick={ctx.link().callback(|_| Msg::AddDocumentType)}><span>{"+"}</span>{"Add document type"}</button>

                            // look at document_types and generate json object from it
                            <button class="button button-primary" onclick={ctx.link().callback(|_| Msg::Submit)}>{"Submit"}</button>
                        </div>
                        <div class="footnotes">
                        </div>
                    </div>
                    <div class="column-right">
                    
                        // format and display json object
                      <div class="input-container">
                            <h2>{"Contract"}</h2>
                            <h3>{if self.imported_json.len() == 0 && self.error_messages.len() != 0 {"Validation errors:"} else {""}}</h3>
                            <div>{
                                if self.imported_json.len() == 0 && self.error_messages.len() != 0 {
                                    html! {
                                        <ul class="error-text">
                                            { for self.error_messages.iter().map(|i| html! { <li>{i.clone()}</li> }) }
                                        </ul>
                                    }
                                } else if self.imported_json.len() == 0 && self.error_messages.len() == 0 &&self.json_object.len() > 0 { 
                                    html! {<p class="passed-text">{"DPP validation passing ✓"}</p>}
                                } else {
                                    html! {""}
                                }
                            }</div>                    
                            <h3>{if self.json_object.len() != 0 {"With whitespace:"} else {""}}</h3>
                            <pre>
                                <textarea class="textarea-whitespace" id="json_output" placeholder="Paste here to import" value={if self.json_object.len() == 0 {self.imported_json.clone()} else {json_pretty}} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateImportedJson(e.target_dyn_into::<web_sys::HtmlTextAreaElement>().unwrap().value()))}></textarea>
                            </pre>
                            <h3>{if self.json_object.len() != 0 {"Without whitespace:"} else {""}}</h3>
                            <pre>{textarea}</pre>
                            <p><b>
                            {
                                if serde_json::to_string(&json_obj).unwrap().len() > 2 {
                                format!("Size: {} bytes", serde_json::to_string(&json_obj).unwrap().len())
                                } else {String::from("Size: 0 bytes")}
                            }
                            </b></p>
                            <div class="button-block">
                              <button class="button-clear" onclick={ctx.link().callback(|_| Msg::Clear)}><span class="clear">{"X"}</span>{"Clear"}</button>
                              <button class="button-import" onclick={ctx.link().callback(|_| Msg::Import)}>{"Import"}</button>
                            </div>
                        
                        <br/>
                        <div class="prompt-history">
                            <h3>{if !self.history.is_empty() {"Prompt history:"} else {""}}</h3>
                            {for self.history.iter().map(|input| html! {
                                <div>{input}</div>
                            })}
                        </div>
                        </div>
                    </div>
                </div>
            </body>
        </main>
        }
    }
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<Model>::new().render();
}
