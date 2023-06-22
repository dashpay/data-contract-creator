use std::{sync::Arc, collections::HashSet};

use dpp::{prelude::Identifier, Convertible, consensus::ConsensusError};
use yew::prelude::*;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{Request, RequestInit, RequestMode, Response};

/// The main struct that holds the prompt and the schema
pub struct App {
    prompt: String,
    schema: String,
    temp_prompt: Option<String>,
    history: Vec<String>,
    loading: bool,
    error_messages: Vec<String>,
}

pub enum Msg {
    /// Updates App.prompt onchange
    UpdatePrompt(String),
    /// Sends prompt to OpenAI which generates a schema onclick
    GenerateSchema,
    /// Takes a schema and sets to App.schema
    ReceiveSchema(Result<String, anyhow::Error>),
    /// Clear the input box
    ClearInput,
}

// Prepended to the prompt if App.schema is empty
const FIRST_PROMPT_PRE: &str = r#"
Here is an example of a Dash data contract JSON schema that has one document 
type "note", which has two properties, "message" and "number", and one non-unique index 
on the "message" property:

{
  "note": {
    "type": "object",
    "indices": [
      {
        "name": "message",
        "properties": [{"message":"asc"}],
        "unique": false
      }
    ],
    "properties": {
      "message": {
        "type": "string"
      },
      "number":{
        "type": "integer"
      }
    },
    "required": ["message"],
    "additionalProperties": false
  }
}

Dash Platform data contract JSON schemas must always specify "additionalProperties":false 
for all objects including properties of type "object". They may have multiple document types, and they may use multiple properties 
within the same index. Indexes can be unique and they may only have asc sort order. 
Properties may be objects themselves as well. "maxLength" of properties who are used in an index must be defined and cannot be 
more than 63.

Generate a comprehensive Dash Platform data contract JSON schema using the context below. 
When formatting your JSON schema, use regular spaces for indentation - do not use tabs or excessive 
whitespace. The formatted schema should look similar to the example given above. Do not explain 
anything or return anything else other than a properly formatted JSON schema:

"#;

// Prepended to the prompt if App.schema is empty
const SECOND_PROMPT_PRE: &str = "
Make the following changes to this data contract JSON schema. 
Only return a formatted JSON schema. Do not explain anything or return anything other than the JSON schema:

";


impl App {
    /// Displays schema if not empty, else displays nothing
    fn render_schema(&self) -> Html {
        if self.schema.is_empty() {
            html! {}
        } else {
            html! {
                <div class="result-container">
                    <pre>{&self.schema}</pre>
                </div>
            }
        }
    }

    fn validate(&mut self) -> Result<Vec<String>, String> {
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
    
        Ok(self.extract_basic_error_messages(&errors))
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

}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            prompt: String::new(),
            schema: String::new(),
            history: Vec::new(),
            temp_prompt: None,
            loading: false,
            error_messages: Vec::new(),
        }
    }    

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
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

                let callback = ctx.link().callback(Msg::ReceiveSchema);
                spawn_local(async move {
                    let result = call_openai(&prompt).await;
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
            
                        self.schema = schema;
                        match self.validate() {
                            Ok(messages) => self.error_messages = messages,
                            Err(err) => self.error_messages.push(err),
                        };
                    },
                    Err(err) => {
                        self.error_messages.push(format!("Error: {:?}", err));
                    },
                }
                self.loading = false;
            },
        }
        true
    }
    
    fn view(&self, ctx: &Context<Self>) -> Html {
        let onsubmit = ctx.link().callback(|event: SubmitEvent| {
            event.prevent_default();  // Prevents page refresh
            Msg::GenerateSchema
        });

        html! {
            <div class="container">
                <div class="top-section">
                    <img class="logo" src="https://media.dash.org/wp-content/uploads/dash-logo.svg" alt="Dash logo" width="200" height="100" />
                    <h1 class="header">{"Data Contract Creator"}</h1>
                </div>
                <div class="content-container">
                    <div class="input-container">
                        <form onsubmit={onsubmit} class="form-container">
                            <div class="input-button-container">
                                <input
                                    placeholder={
                                        if self.schema.is_empty() {
                                            "Briefly describe a data contract."
                                        } else {
                                            "Describe any adjustments. Refresh the page to start over."
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
                                    <div class="loader">
                                        <div class="dot"></div>
                                        <div class="dot"></div>
                                        <div class="dot"></div>
                                    </div>
                                }
                            } else {
                                html! {}
                            }
                        }    
                    </div>
                    <div class="validation-container">
                        { if !self.error_messages.is_empty() { html! { <h3>{"Validation errors:"}</h3> } } else { html! {} } }
                        
                        { if !self.error_messages.is_empty() {
                                html! {
                                    <ul class="error-text">
                                        { for self.error_messages.iter().map(|i| html! { <li>{i.clone()}</li> }) }
                                    </ul>
                                }
                            } else if self.error_messages.is_empty() && !self.schema.is_empty() { 
                                html! {<p class="passed-text">{"DPP validation passing ✓"}</p>}
                            } else {
                                html! {}
                            }
                        }
                    </div>                    
                    {self.render_schema()}
                    <h3>{if !self.history.is_empty() {"Prompt history:"} else {""}}</h3>
                    {for self.history.iter().map(|input| html! {
                        <div>{input}</div>
                    })}
                </div>
            </div>
        }
    }    
}

/// Calls OpenAI
pub async fn call_openai(prompt: &str) -> Result<String, anyhow::Error> {
    let params = serde_json::json!({
        "model": "text-davinci-003",
        "prompt": prompt,
        "max_tokens": 3000,
        "temperature": 0.3
    });
    let params = params.to_string();

    let mut opts = RequestInit::new();
    let headers = web_sys::Headers::new().unwrap();
    headers.append("Authorization", "Bearer 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").unwrap();
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
    let schema = json["choices"][0]["text"].as_str().unwrap_or("");

    Ok(schema.trim().to_string())
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    yew::Renderer::<App>::new().render();
}