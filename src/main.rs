//! Dash Platform Data Contract Creator

use std::{collections::{HashMap, HashSet}, sync::Arc};
use serde::{Serialize, Deserialize};
use yew::{prelude::*, html, Component, Html, Event, InputEvent, FocusEvent, TargetCast};
use serde_json::{json, Map, Value};
use dpp::{self, consensus::ConsensusError, prelude::Identifier, Convertible};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Request, RequestInit, RequestMode, Response, HtmlSelectElement};

const OPENAI_API_KEY: &str = "your key here" ;

// Prepended to the prompt if App.schema is empty
const FIRST_PROMPT_PRE: &str = r#"
Here is an example of a Dash data contract JSON schema that has one document 
type "note", which has two properties, "message" and "number", and one non-unique index 
on the "message" property:

{"note":{"type":"object","indices":[{"name":"message","properties":[{"message":"asc"}],"unique":false}],"properties":{"message":{"type":"string"},"number":{"type":"integer"}},"required":["message"],"additionalProperties":false}}

Dash Platform data contract JSON schemas must always specify "additionalProperties":false 
for all objects including properties of type "object". They may have multiple document types, 
and they may use multiple properties within the same index (compound indexes). Compound index properties are formatted like so: ""properties":[{"prop1": "asc"},{"prop2": "asc"}]".
If an index uses a nested property, it must specify the path, for example "outer_prop.inner_prop" rather than just "inner_prop". 
Indexes can be unique and they may only have asc sort order. "maxLength" of properties who are 
used in an index must be defined and cannot be more than 63.

Generate a comprehensive Dash Platform data contract JSON schema using the context below. 
When formatting your JSON schema, use 2 spaces for indentation. Don't use tabs or more than 2 spaces. Do not explain 
anything or return anything else other than a properly formatted JSON schema:

"#;

// Prepended to the prompt if App.schema is empty
const SECOND_PROMPT_PRE: &str = r#"
Dash Platform data contract JSON schemas must always specify "additionalProperties":false 
for all objects including properties of type "object". They may have multiple document types, 
and they may use multiple properties within the same index (compound indexes). Compound index properties are formatted like so: ""properties":[{"prop1": "asc"},{"prop2": "asc"}]".
If an index uses a nested property, it must specify the path, for example "outer_prop.inner_prop" rather than just "inner_prop". 
Indexes can be unique and they may only have asc sort order. "maxLength" of properties who are 
used in an index must be defined and cannot be more than 63.

Make the following changes to this Dash Platform data contract JSON schema. 
Only return a formatted JSON schema. Do not explain anything or return anything other than the JSON schema:

"#;

/// Calls OpenAI
pub async fn call_openai(prompt: &str) -> Result<String, anyhow::Error> {
    let params = serde_json::json!({
        "model": "text-davinci-003",
        "prompt": prompt,
        "max_tokens": 3000,
        "temperature": 0.5
    });
    let params = params.to_string();

    let mut opts = RequestInit::new();
    let headers = web_sys::Headers::new().unwrap();

    headers.append("Authorization", &format!("Bearer {}", OPENAI_API_KEY)).unwrap();
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
    
    let text = JsFuture::from(response.text().unwrap()).await;
    
    let text = match text {
        Ok(txt) => txt,
        Err(err) => return Err(anyhow::anyhow!(err.as_string().unwrap_or("Failed to get text from response".to_string()))),
    };

    let text: String = match text.as_string() {
        Some(txt) => txt,
        None => return Err(anyhow::anyhow!("Failed to convert JsValue to String")),
    };
    
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let schema_text = json["choices"][0]["text"].as_str().unwrap_or("");

    let start = schema_text.find('{');
    let end = schema_text.rfind('}');

    match (start, end) {
        (Some(start), Some(end)) => {
            let schema_json = &schema_text[start..=end];
            match serde_json::from_str::<serde_json::Value>(schema_json) {
                Ok(_) => Ok(schema_json.to_string()),
                Err(_) => Err(anyhow::anyhow!("Extracted text is not valid JSON")),
            }
        }
        _ => Err(anyhow::anyhow!("No valid JSON found in the returned text.")),
    }
}

/// Document type struct
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(non_snake_case)]
struct DocumentType {
    name: String,
    properties: Vec<Property>,
    indices: Vec<Index>,
    required: Vec<String>,
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

/// Property data types
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

/// Model is the umbrella structs that contains all the document types 
/// for the contract and the vector of strings comprising the json object to be output
struct Model {
    /// A vector of document types
    document_types: Vec<DocumentType>,
    /// Each full document type is a single string in json_object
    json_object: Vec<String>,
    /// A string containing a full data contract
    imported_json: String,
    /// DPP validation error messages
    error_messages: Vec<String>,

    // OpenAI
    prompt: String,
    schema: String,
    temp_prompt: Option<String>,
    history: Vec<String>,
    loading: bool,
    error_messages_ai: Vec<String>,
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
    UpdatePropertyDescription(usize, usize, String),
    UpdatePropertyComment(usize, usize, String),
    UpdateIndexProperty(usize, usize, usize, String),

    // Optional property parameters
    UpdateStringPropertyMinLength(usize, usize, u32),
    UpdateStringPropertyMaxLength(usize, usize, u32),
    UpdateStringPropertyPattern(usize, usize, String),
    UpdateStringPropertyFormat(usize, usize, String),
    UpdateIntegerPropertyMinimum(usize, usize, i32),
    UpdateIntegerPropertyMaximum(usize, usize, i32),
    UpdateArrayPropertyByteArray(usize, usize, bool),
    UpdateArrayPropertyMinItems(usize, usize, u32),
    UpdateArrayPropertyMaxItems(usize, usize, u32),
    UpdateObjectPropertyMinProperties(usize, usize, u32),
    UpdateObjectPropertyMaxProperties(usize, usize, u32),

    // Recursive properties
    AddRecProperty(usize, usize),
    RemoveRecProperty(usize, usize, usize),
    UpdateRecPropertyType(usize, usize, usize, String),
    UpdateRecPropertyName(usize, usize, usize, String),
    UpdateRecPropertyRequired(usize, usize, usize, bool),
    UpdateRecPropertyDescription(usize, usize, usize, String),
    UpdateRecPropertyComment(usize, usize, usize, String),
    UpdateStringRecPropertyMinLength(usize, usize, usize, u32),
    UpdateStringRecPropertyMaxLength(usize, usize, usize, u32),
    UpdateStringRecPropertyPattern(usize, usize, usize, String),
    UpdateStringRecPropertyFormat(usize, usize, usize, String),
    UpdateIntegerRecPropertyMaximum(usize, usize, usize, i32),
    UpdateIntegerRecPropertyMinimum(usize, usize, usize, i32),
    UpdateArrayRecPropertyByteArray(usize, usize, usize, bool),
    UpdateArrayRecPropertyMinItems(usize, usize, usize, u32),
    UpdateArrayRecPropertyMaxItems(usize, usize, usize, u32),
    UpdateObjectRecPropertyMaxProperties(usize, usize, usize, u32),
    UpdateObjectRecPropertyMinProperties(usize, usize, usize, u32),

    // Import
    Import,
    UpdateImportedJson(String),
    Clear,

    // OpenAI
    /// Updates App.prompt onchange
    UpdatePrompt(String),
    /// Sends prompt to OpenAI which generates a schema onclick
    GenerateSchema,
    /// Takes a schema and sets to App.schema
    ReceiveSchema(Result<String, anyhow::Error>),
    /// Clear the input box after submission and response
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
            byte_array: None,
            min_items: None,
            max_items: None,
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
                    <h3>{"Properties"}</h3>
                    <table>
                        <tbody>
                            {for (0..self.document_types[index].properties.len()).map(|i| self.view_property(index, i, ctx))}
                            <tr>
                                <td><button class="button" onclick={ctx.link().callback(move |_| Msg::AddProperty(index))}>{"Add property"}</button></td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                <div>
                    <h3>{"Indices"}</h3>
                    <table>
                        <tbody>
                            {for (0..self.document_types[index].indices.len()).map(|i| self.view_index(index, i, ctx))}
                            <tr>
                                <td><button class="button" onclick={ctx.link().callback(move |_| Msg::AddIndex(index))}>{"Add index"}</button></td>
                            </tr>
                        </tbody>
                    </table>
                </div>
                <div>
                    <h3>{"Comment"}</h3>
                    <input type="text2" placeholder="Comment" value={self.document_types[index].comment.clone()} onblur={ctx.link().callback(move |e: FocusEvent| Msg::UpdateComment(index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                </div>
                <br/>
                <div>
                <button class="button" onclick={ctx.link().callback(move |_| Msg::RemoveDocumentType(index))}>{format!("Remove document type {}", index+1)}</button>
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
                <tr>
                    <th>{format!("Property {} name", prop_index+1)}</th>
                    <th>{"Type"}</th>
                    <th>{"Required"}</th>
                </tr>
                <tr>
                    <td><input type="text3" placeholder={format!("Property {} name", prop_index+1)} value={self.document_types[doc_index].properties[prop_index].name.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePropertyName(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                    <td>
                        <select value={selected_data_type.clone()} onchange={ctx.link().callback(move |e: Event| {
                            let selected_data_type = e.target_dyn_into::<HtmlSelectElement>().unwrap().value();
                            let new_property = default_additional_properties(selected_data_type.as_str());
                            Msg::UpdatePropertyType(doc_index, prop_index, new_property)
                        })}>
                            {for data_type_options.iter().map(|option| html! {
                                <option value={String::from(*option)} selected={&String::from(*option)==&selected_data_type}>{String::from(*option)}</option>
                            })}
                        </select>
                    </td>
                    <td><input type="checkbox" checked={self.document_types[doc_index].properties[prop_index].required} onchange={ctx.link().callback(move |e: Event| Msg::UpdatePropertyRequired(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} /></td>
                    <td><button class="button" onclick={ctx.link().callback(move |_| Msg::RemoveProperty(doc_index, prop_index))}>{"Remove"}</button></td>
                </tr>
                <p><b>{if selected_data_type != String::from("Object") { "Optional property parameters:" } else {""}}</b></p>
                <tr>
                    <td colspan="4">
                        {additional_properties}
                        <tr>
                            <td><label>{"Description: "}</label></td>
                            <td><input type="text3" value={self.document_types[doc_index].properties[prop_index].description.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePropertyDescription(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                        </tr>
                        <tr>
                            <td><label>{"Comment: "}</label></td>
                            <td><input type="text3" value={self.document_types[doc_index].properties[prop_index].comment.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePropertyComment(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                        </tr>
                        <p></p>
                    </td>
                </tr>
            </>
        }
    }

    fn render_additional_properties(&self, data_type: &String, doc_index: usize, prop_index: usize, ctx: &yew::Context<Self>) -> Html {
        let property = &self.document_types[doc_index].properties[prop_index];
        match data_type.as_str() {
            "String" => html! {
                <>
                <tr>
                    <td><label>{"Min length: "}</label></td>
                    <td><input type="number" value={property.min_length.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringPropertyMinLength(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                </tr>
                <tr>
                    <td><label>{"Max length: "}</label></td>
                    <td><input type="number" value={property.max_length.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringPropertyMaxLength(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                </tr>
                <tr>
                    <td><label>{"RE2 pattern: "}</label></td>
                    <td><input type="text3" value={property.pattern.clone().unwrap_or_default()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringPropertyPattern(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                </tr>
                <tr>
                    <td><label>{"Format: "}</label></td>
                    <td><input type="text3" value={property.format.clone().unwrap_or_default()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringPropertyFormat(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                </tr>
                </>
            },
            "Integer" => html! {
                <>
                <tr>
                    <td><label>{"Minimum: "}</label></td>
                    <td><input type="number" value={property.minimum.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIntegerPropertyMinimum(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as i32))} /></td>
                </tr>
                <tr>
                    <td><label>{"Maximum: "}</label></td>
                    <td><input type="number" value={property.maximum.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIntegerPropertyMaximum(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as i32))} /></td>
                </tr>
                </>
            },
            "Array" => html! {
                <>
                <tr>
                    <td><label>{"Byte array: "}</label></td>
                    <td><input type="checkbox" checked={property.byte_array.unwrap_or(false)} onchange={ctx.link().callback(move |e: Event| Msg::UpdateArrayPropertyByteArray(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} /></td>
                </tr>
                <tr>
                    <td><label>{"Min items: "}</label></td>
                    <td><input type="number" value={property.min_items.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateArrayPropertyMinItems(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                </tr>
                <tr>
                    <td><label>{"Max items: "}</label></td>
                    <td><input type="number" value={property.max_items.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateArrayPropertyMaxItems(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                </tr>
                </>
            },
            "Object" => html! {
                <>
                <tr>
                    <td colspan="4">
                    {for self.document_types[doc_index].properties[prop_index].properties.as_ref().unwrap_or(&Box::new(Vec::new())).iter().enumerate().map(|(i, _)| self.view_recursive_property(doc_index, prop_index, i, ctx))}
                    </td>
                </tr>
                <tr>
                    <td><button class="button" onclick={ctx.link().callback(move |_| Msg::AddRecProperty(doc_index, prop_index))}>{"Add inner property"}</button></td>
                </tr>
                <p><b>{"Optional property parameters:"}</b></p>
                <tr>
                    <td><label>{"Min properties: "}</label></td>
                    <td><input type="number" value={property.min_properties.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateObjectPropertyMinProperties(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                </tr>
                <tr>
                    <td><label>{"Max properties: "}</label></td>
                    <td><input type="number" value={property.max_properties.map(|n| n.to_string()).unwrap_or_else(|| "".to_owned())} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateObjectPropertyMaxProperties(doc_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                </tr>
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
                <tr>
                    <th>{format!("Inner property {} name", recursive_prop_index+1)}</th>
                    <th>{"Type"}</th>
                    <th>{"Required"}</th>
                </tr>
                <tr>
                    <td>
                        <input type="text3" placeholder={format!("Inner property {} name", recursive_prop_index+1)} value={match &self.document_types[doc_index].properties[prop_index].properties {
                            Some(properties) => properties.get(recursive_prop_index).map(|property| property.name.clone()).unwrap_or_default(),
                            None => String::new(),
                        }} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateRecPropertyName(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} />
                    </td>
                    <td>
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
                    </td>
                    <td>
                        <input type="checkbox" checked={match &self.document_types[doc_index].properties[prop_index].properties {
                            Some(properties) => properties.get(recursive_prop_index).map(|property| property.required).unwrap_or(false),
                            None => false,
                        }} onchange={ctx.link().callback(move |e: Event| Msg::UpdateRecPropertyRequired(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} />
                    </td>
                    <td>
                        <button class="button" onclick={ctx.link().callback(move |_| Msg::RemoveRecProperty(doc_index, prop_index, recursive_prop_index))}>{"Remove"}</button>
                    </td>
                </tr>
                <p><b>{"Optional property parameters:"}</b></p>
                <tr>
                    <td colspan="4">
                        <table>
                            {self.rec_render_additional_properties(&selected_data_type, doc_index, prop_index, recursive_prop_index, ctx)}
                            <tr>
                                <td><label>{"Description: "}</label></td>
                                <td><input type="text3" value={if let Some(properties) = &self.document_types.get(doc_index).and_then(|doc| doc.properties.get(prop_index).and_then(|prop| prop.properties.clone())) {
                                    properties.get(recursive_prop_index).and_then(|prop| prop.description.clone()).unwrap_or_default()
                                } else {
                                    "".to_string()
                                }} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateRecPropertyDescription(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                            </tr>
                            <tr>
                                <td><label>{"Comment: "}</label></td><td><input type="text3" value={if let Some(properties) = &self.document_types.get(doc_index).and_then(|doc| doc.properties.get(prop_index).and_then(|prop| prop.properties.clone())) {
                                    properties.get(recursive_prop_index).and_then(|prop| prop.comment.clone()).unwrap_or_default()
                                } else {
                                    "".to_string()
                                }} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateRecPropertyComment(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                            </tr>
                            <p></p>
                        </table>
                    </td>
                </tr>
            </>
        }
    }

    fn rec_render_additional_properties(&self, data_type: &String, doc_index: usize, prop_index: usize, recursive_prop_index: usize, ctx: &yew::Context<Self>) -> Html {
        match data_type.as_str() {
            "String" => {
                if let Some(_props) = &self.document_types[doc_index].properties[prop_index].properties {
                    html! {
                        <>
                        <tr>
                            <td><label>{"Min length: "}</label></td>
                            <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringRecPropertyMinLength(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                        </tr>
                        <tr>
                            <td><label>{"Max length: "}</label></td>
                            <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringRecPropertyMaxLength(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as u32))} /></td>
                        </tr>
                        <tr>
                            <td><label>{"RE2 pattern: "}</label></td>
                            <td><input type="text3" oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringRecPropertyPattern(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                        </tr>
                        <tr>
                            <td><label>{"Format: "}</label></td>
                            <td><input type="text3" oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateStringRecPropertyFormat(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                        </tr>
                        </>
                    }
                } else {
                    html! {}
                }
            },
            "Integer" => {
                if let Some(_props) = &self.document_types[doc_index].properties[prop_index].properties {
                    html! {
                        <>
                        <tr>
                            <td><label>{"Minimum: "}</label></td>
                            <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIntegerRecPropertyMinimum(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as i32))} /></td>
                        </tr>
                        <tr>
                            <td><label>{"Maximum: "}</label></td>
                            <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIntegerRecPropertyMaximum(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number() as i32))} /></td>
                        </tr>
                        </>
                    }
                } else {
                    html! {}
                }
            },
            "Array" => {
                let properties = self.document_types[doc_index].properties.get(prop_index).and_then(|p| p.properties.as_ref());
                let byte_array = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.byte_array);
                let max_items = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.max_items);
                let min_items = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.min_items);
            
                html! {
                    <>
                    <tr>
                        <td><label>{"Byte array: "}</label></td>
                        <td><input type="checkbox" checked={byte_array.unwrap_or(false)} onchange={ctx.link().callback(move |e: Event| Msg::UpdateArrayRecPropertyByteArray(doc_index, prop_index, recursive_prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} /></td>
                    </tr>
                    <tr>
                        <td><label>{"Min items: "}</label></td>
                        <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number();
                            let value = match value {
                                v if v.is_finite() => Some(v as u32),
                                _ => None,
                            };
                            Msg::UpdateArrayRecPropertyMinItems(doc_index, prop_index, recursive_prop_index, value.unwrap_or(0))
                        })} value={min_items.map(|n| n.to_string()).unwrap_or_default().to_owned()} /></td>
                    </tr>
                    <tr>
                        <td><label>{"Max items: "}</label></td>
                        <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number();
                            let value = match value {
                                v if v.is_finite() => Some(v as u32),
                                _ => None,
                            };
                            Msg::UpdateArrayRecPropertyMaxItems(doc_index, prop_index, recursive_prop_index, value.unwrap_or(0))
                        })} value={max_items.map(|n| n.to_string()).unwrap_or_default().to_owned()} /></td>
                    </tr>
                    </>
                }
            },            
            "Object" => {
                let properties = self.document_types[doc_index].properties.get(prop_index).and_then(|p| p.properties.as_ref());
                let min_props = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.min_properties);
                let max_props = properties.and_then(|p| p.get(recursive_prop_index)).and_then(|p| p.max_properties);
            
                html! {
                    <>
                    <tr>
                        <td><label>{"Min properties: "}</label></td>
                        <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number();
                            let value = match value {
                                v if v.is_finite() => Some(v as u32),
                                _ => None,
                            };
                            Msg::UpdateObjectRecPropertyMinProperties(doc_index, prop_index, recursive_prop_index, value.unwrap_or(0))
                        })} value={min_props.map(|n| n.to_string()).unwrap_or_default().to_owned()} /></td>
                    </tr>
                    <tr>
                        <td><label>{"Max properties: "}</label></td>
                        <td><input type="number" oninput={ctx.link().callback(move |e: InputEvent| {
                            let value = e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value_as_number();
                            let value = match value {
                                v if v.is_finite() => Some(v as u32),
                                _ => None,
                            };
                            Msg::UpdateObjectRecPropertyMaxProperties(doc_index, prop_index, recursive_prop_index, value.unwrap_or(0))
                        })} value={max_props.map(|n| n.to_string()).unwrap_or_default().to_owned()} /></td>
                    </tr>
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
            <tr>
                <th>{format!("Index {} name", index_index+1)}</th>
                <th>{"Unique"}</th>
                <th>{""}</th>
            </tr>
            <tr>
                <td><input type="text3" placeholder={format!("Index {} name", index_index+1)} value={self.document_types[doc_index].indices[index_index].name.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIndexName(doc_index, index_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                <td><input type="checkbox" checked={self.document_types[doc_index].indices[index_index].unique} onchange={ctx.link().callback(move |e: Event| Msg::UpdateIndexUnique(doc_index, index_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().checked()))} /></td>
                <td><button class="button" onclick={ctx.link().callback(move |_| Msg::RemoveIndex(doc_index, index_index))}>{"Remove"}</button></td>
            </tr>
            <tr>
                <td colspan="3">
                    <table>
                        <tbody>
                            <p><b>{"Index properties:"}</b></p>
                            <div>{for (0..self.document_types[doc_index].indices[index_index].properties.len()).map(|i| self.view_index_properties(doc_index, index_index, i, ctx))}</div>
                        </tbody>
                    </table>
                </td>
            </tr>
            <p></p>
            <tr>
                <td colspan="2"><button class="button" onclick={ctx.link().callback(move |_| Msg::AddIndexProperty(doc_index, index_index))}>{"Add index property"}</button></td>
            </tr>
            <p></p>
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
            <tr class="row">
                <td class="label-column">{format!("Property {}: ", prop_index+1)}</td>
                <td class="input-column"><input type="text3" value={self.document_types[doc_index].indices[index_index].properties[prop_index].0.clone()} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateIndexProperty(doc_index, index_index, prop_index, e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))} /></td>
                <td class="select-column"><select value={current_sort} onchange={ctx.link().callback(move |e: Event| Msg::UpdateIndexSorting(doc_index, index_index, prop_index, match e.target_dyn_into::<HtmlSelectElement>().unwrap().value().as_str() {
                    "Ascending" => String::from("asc"),
                    "Descending" => String::from("desc"),
                    _ => panic!("Invalid data type selected"),
                }))}>
                    {for sorting_options.iter().map(|option| html! {
                        <option value={String::from(*option)} selected={&String::from(*option)==current_sort}>{String::from(*option)}</option>
                    })}
                </select></td>
            </tr>
        )
    }

    fn generate_json_object(&mut self) -> Vec<String> {
        println!("generate_json_object: start");
        let mut json_arr = Vec::new();
        for doc_type in &mut self.document_types {
            println!("generate_json_object: iterating document types");
            let mut props_map = Map::new();
            for prop in &mut doc_type.properties {
                println!("generate_json_object: iterating properties");
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
                if prop.min_length.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    prop_obj.insert("minLength".to_owned(), json!(prop.min_length));
                }
                if prop.max_length.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    prop_obj.insert("maxLength".to_owned(), json!(prop.max_length));
                }
                if prop.pattern.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    prop_obj.insert("pattern".to_owned(), json!(prop.pattern));
                }
                if prop.format.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    prop_obj.insert("format".to_owned(), json!(prop.format));
                }
                if prop.minimum.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    prop_obj.insert("minimum".to_owned(), json!(prop.minimum));
                }
                if prop.maximum.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    prop_obj.insert("maximum".to_owned(), json!(prop.maximum));
                }
                if let Some(byte_array) = prop.byte_array {
                    prop_obj.insert("byteArray".to_owned(), json!(byte_array));
                }
                if prop.min_items.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    prop_obj.insert("minItems".to_owned(), json!(prop.min_items));
                }
                if prop.max_items.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    prop_obj.insert("maxItems".to_owned(), json!(prop.max_items));
                }
                if prop.data_type == DataType::Object {
                    let rec_props_map = Self::generate_nested_properties(prop);
                    prop_obj.insert("properties".to_owned(), json!(rec_props_map));
                    }
                if prop.min_properties.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    prop_obj.insert("minProperties".to_owned(), json!(prop.min_properties));
                }
                if prop.max_properties.as_ref().map(|c| *c).unwrap_or(0) > 0 {
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
                if rec_prop.min_length.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("minLength".to_owned(), json!(rec_prop.min_length));
                }
                if rec_prop.max_length.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("maxLength".to_owned(), json!(rec_prop.max_length));
                }
                if rec_prop.pattern.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("pattern".to_owned(), json!(rec_prop.pattern));
                }
                if rec_prop.format.as_ref().map(|c| c.len()).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("format".to_owned(), json!(rec_prop.format));
                }
                if rec_prop.minimum.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("minimum".to_owned(), json!(rec_prop.minimum));
                }
                if rec_prop.maximum.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("maximum".to_owned(), json!(rec_prop.maximum));
                }
                if let Some(byte_array) = rec_prop.byte_array {
                    rec_prop_obj.insert("byteArray".to_owned(), json!(byte_array));
                }
                if rec_prop.min_items.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("minItems".to_owned(), json!(rec_prop.min_items));
                }
                if rec_prop.max_items.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("maxItems".to_owned(), json!(rec_prop.max_items));
                }
                if rec_prop.data_type == DataType::Object {
                    rec_prop_obj.insert("properties".to_owned(), json!({}));
                }
                if rec_prop.min_properties.as_ref().map(|c| *c).unwrap_or(0) > 0 {
                    rec_prop_obj.insert("minProperties".to_owned(), json!(rec_prop.min_properties));
                }
                if rec_prop.max_properties.as_ref().map(|c| *c).unwrap_or(0) > 0 {
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
                                                if let Some(min_length) = nested_prop_obj.get("minLength") {
                                                    nested_property.min_length = min_length.as_u64().map(|num| num as u32);
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
        let contract = factory
            .create(owner_id, json_obj.clone().into(), None, None)
            .expect("data in fixture should be correct");
        let results = contract.data_contract.validate(&contract.data_contract.to_cleaned_object().unwrap()).unwrap_or_default();
        let errors = results.errors;
        self.extract_basic_error_messages(&errors)
    }

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

    // OpenAI
    fn validate_ai(&mut self) -> Result<Vec<String>, String> {
        let json_obj: serde_json::Value = match serde_json::from_str(&self.schema) {
            Ok(json) => json,
            Err(e) => return Err(format!("Error parsing schema: {}. Suggest refreshing.", e)),
        };
    
        let protocol_version_validator = dpp::version::ProtocolVersionValidator::default();
        let data_contract_validator = dpp::data_contract::validation::data_contract_validator::DataContractValidator::new(Arc::new(protocol_version_validator));
        let factory = dpp::data_contract::DataContractFactory::new(1, Arc::new(data_contract_validator));
        let owner_id = Identifier::random();
        let contract = match factory.create(owner_id, json_obj.clone().into(), None, None) {
            Ok(contract) => contract,
            Err(e) => return Err(format!("Error creating contract: {}", e)),
        };
    
        let results = contract.data_contract.validate(
            &contract.data_contract.to_cleaned_object().expect("Descriptive error message")
        );
        let errors = results.unwrap_or_default().errors;
    
        Ok(self.extract_basic_error_messages_ai(&errors))
    }    

    fn extract_basic_error_messages_ai(&self, errors: &[ConsensusError]) -> Vec<String> {
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

impl Component for Model {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        let mut default_document_type = DocumentType::default();
        default_document_type.properties.push(Property::default());
        Self {
            document_types: vec![default_document_type],
            json_object: vec![],
            imported_json: String::new(),
            error_messages: vec![],
            prompt: String::new(),
            schema: String::new(),
            history: Vec::new(),
            temp_prompt: None,
            loading: false,
            error_messages_ai: Vec::new(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
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
            Msg::UpdatePropertyDescription(doc_index, prop_index, description) => {
                self.document_types[doc_index].properties[prop_index].description = Some(description);
            }
            Msg::UpdatePropertyComment(doc_index, prop_index, comment) => {
                self.document_types[doc_index].properties[prop_index].comment = Some(comment);
            }

            // Optional property parameters
            Msg::UpdateStringPropertyMinLength(doc_index, prop_index, min_length) => {
                self.document_types[doc_index].properties[prop_index].min_length = Some(min_length);
            }
            Msg::UpdateStringPropertyMaxLength(doc_index, prop_index, max_length) => {
                self.document_types[doc_index].properties[prop_index].max_length = Some(max_length);
            }
            Msg::UpdateStringPropertyPattern(doc_index, prop_index, pattern) => {
                self.document_types[doc_index].properties[prop_index].pattern = Some(pattern);
            }
            Msg::UpdateStringPropertyFormat(doc_index, prop_index, format) => {
                self.document_types[doc_index].properties[prop_index].format = Some(format);
            }
            Msg::UpdateIntegerPropertyMinimum(doc_index, prop_index, minimum) => {
                self.document_types[doc_index].properties[prop_index].minimum = Some(minimum);
            }
            Msg::UpdateIntegerPropertyMaximum(doc_index, prop_index, maximum) => {
                self.document_types[doc_index].properties[prop_index].maximum = Some(maximum);
            }
            Msg::UpdateArrayPropertyByteArray(doc_index, prop_index, byte_array) => {
                self.document_types[doc_index].properties[prop_index].byte_array = Some(byte_array);
            }
            Msg::UpdateArrayPropertyMinItems(doc_index, prop_index, min_items) => {
                self.document_types[doc_index].properties[prop_index].min_items = Some(min_items);
            }
            Msg::UpdateArrayPropertyMaxItems(doc_index, prop_index, max_items) => {
                self.document_types[doc_index].properties[prop_index].max_items = Some(max_items);
            }
            Msg::UpdateObjectPropertyMinProperties(doc_index, prop_index, min_properties) => {
                self.document_types[doc_index].properties[prop_index].min_properties = Some(min_properties);
            }
            Msg::UpdateObjectPropertyMaxProperties(doc_index, prop_index, max_properties) => {
                self.document_types[doc_index].properties[prop_index].max_properties = Some(max_properties);
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
                    property_vec[rec_prop_index].min_length = Some(min_length);
                }
            }
            Msg::UpdateStringRecPropertyMaxLength(doc_index, prop_index, rec_prop_index, max_length) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].max_length = Some(max_length);
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
                    property_vec[rec_prop_index].maximum = Some(maximum);
                }
            }
            Msg::UpdateIntegerRecPropertyMinimum(doc_index, prop_index, rec_prop_index, minimum) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].minimum = Some(minimum);
                }
            }
            Msg::UpdateArrayRecPropertyByteArray(doc_index, prop_index, rec_prop_index, byte_array) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].byte_array = Some(byte_array);
                }
            }
            Msg::UpdateArrayRecPropertyMinItems(doc_index, prop_index, rec_prop_index, min_items) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].min_items = Some(min_items);
                }
            }
            Msg::UpdateArrayRecPropertyMaxItems(doc_index, prop_index, rec_prop_index, max_items) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].max_items = Some(max_items);
                }
            }
            Msg::UpdateObjectRecPropertyMinProperties(doc_index, prop_index, rec_prop_index, min_props) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].min_properties = Some(min_props);
                }
            }
            Msg::UpdateObjectRecPropertyMaxProperties(doc_index, prop_index, rec_prop_index, max_props) => {
                if let Some(property_vec) = self.document_types[doc_index].properties[prop_index].properties.as_mut() {
                    property_vec[rec_prop_index].max_properties = Some(max_props);
                }
            }

            // Import
            Msg::UpdateImportedJson(import) => {
                self.imported_json = import;
            }
            Msg::Import => {
                self.parse_imported_json();
            }
            Msg::Clear => {
                self.json_object = vec![];
                self.imported_json = String::new();
            }
            
            // OpenAI
            Msg::UpdatePrompt(val) => {
                self.prompt = val;
            },
            Msg::GenerateSchema => {
                let prompt = if self.schema.is_empty() {
                    let first_prompt_pre = FIRST_PROMPT_PRE.to_string();
                    format!("{}{}", first_prompt_pre, self.prompt)
                } else {
                    let second_prompt_pre = SECOND_PROMPT_PRE.to_string();
                    format!("{}{}\n\n{}", second_prompt_pre, self.schema, self.prompt)
                };
    
                // Save the prompt temporarily
                self.temp_prompt = Some(self.prompt.clone());
        
                self.loading = true;

                let callback = _ctx.link().callback(Msg::ReceiveSchema);
                spawn_local(async move {
                    let result = call_openai(&prompt).await;
                    callback.emit(result);
                });
    
                // Clear the input field
                _ctx.link().send_message(Msg::ClearInput);
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
        
                        match self.validate_ai() {
                            Ok(messages) => self.error_messages_ai = messages,
                            Err(err) => self.error_messages_ai.push(err),
                        };
                    },
                    Err(err) => {
                        self.error_messages_ai.push(format!("Error: {:?}", err));
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
                <textarea class="textarea" id="json_output" value={if self.json_object.len() != 0 as usize {
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
                                            "Briefly describe a data contract."
                                        } else {
                                            "Describe any adjustments. Refresh the page to start new."
                                        }
                                    }
                                    oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdatePrompt(e.target_dyn_into::<web_sys::HtmlInputElement>().unwrap().value()))}
                                />
                                <button type="submit">{"Generate"}</button>
                            </div>
                        </form>
                        {
                            if self.loading {
                                html! {
                                    <div class="loader_ai">
                                        <div class="dot_ai"></div>
                                        <div class="dot_ai"></div>
                                        <div class="dot_ai"></div>
                                    </div>
                                }
                            } else {
                                html! {}
                            }
                        }    
                    </div>
                    <h3>{if !self.history.is_empty() {"Prompt history:"} else {""}}</h3>
                    {for self.history.iter().map(|input| html! {
                        <div>{input}</div>
                    })}
                </div>
            </div>
            <body>
            <div class="column-left">

                // show input fields
                <p class="input-fields">{self.view_document_types(ctx)}</p>

                <div class="button-container">
                    // add input fields for another document type and add one to Self::document_types
                    <button class="button2" onclick={ctx.link().callback(|_| Msg::AddDocumentType)}>{"Add document type"}</button><br/>

                    // look at document_types and generate json object from it
                    <button class="button button-primary" onclick={ctx.link().callback(|_| Msg::Submit)}>{"Submit"}</button>
                </div>
                <div class="footnotes">
                </div>
            </div>
            <div class="column-right">
            
                // format and display json object
                <p class="output-container">
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
                    <textarea class="textarea" id="json_output" placeholder="Paste here to import" value={if self.json_object.len() == 0 {self.imported_json.clone()} else {json_pretty}} oninput={ctx.link().callback(move |e: InputEvent| Msg::UpdateImportedJson(e.target_dyn_into::<web_sys::HtmlTextAreaElement>().unwrap().value()))}></textarea>
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
                    <div><button class="button-import" onclick={ctx.link().callback(|_| Msg::Import)}>{"Import"}</button></div>
                    <div><button class="button-clear" onclick={ctx.link().callback(|_| Msg::Clear)}>{"Clear"}</button></div>
                </p>
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
