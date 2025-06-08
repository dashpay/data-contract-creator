use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::HtmlInputElement;
use yew::events::{MouseEvent, SubmitEvent};
use yew::prelude::*;

use crate::services::{JsonGenerator, JsonParser, OpenAiService, ValidationService};
use crate::types::{DataType, DocumentType, Index, Property, ValidationError};

/// Main application state
pub struct App {
    /// Document types being edited
    pub document_types: Vec<DocumentType>,

    /// Generated JSON output
    json_output: String,

    /// Validation errors
    validation_errors: Vec<ValidationError>,

    /// AI prompt input
    ai_prompt: String,

    /// Whether AI request is in progress  
    ai_loading: bool,

    /// AI error messages
    ai_errors: Vec<String>,

    /// Prompt history
    prompt_history: Vec<String>,

    /// Import JSON text
    import_json: String,

    /// Whether to show formatted JSON
    show_formatted: bool,

    /// Whether validation has been explicitly requested
    validation_requested: bool,

    /// Whether the compact JSON popup is visible
    show_compact_popup: bool,
    
    /// Track which property optional sections are expanded
    pub expanded_property_options: std::collections::HashSet<(usize, usize)>,
}

/// Messages for app state updates
#[derive(Debug)]
pub enum AppMsg {
    // Document type operations
    AddDocumentType,
    RemoveDocumentType(usize),
    UpdateDocumentTypeName(usize, String),
    UpdateDocumentTypeComment(usize, String),
    UpdateDocumentTypeDescription(usize, String),
    UpdateDocumentTypeKeywords(usize, String),
    UpdateDocumentTypeCreatedAt(usize, bool),
    UpdateDocumentTypeUpdatedAt(usize, bool),
    UpdateDocumentTypeAdditionalProperties(usize, bool),

    // Property operations
    AddProperty(usize),
    RemoveProperty(usize, usize),
    UpdatePropertyName(usize, usize, String),
    UpdatePropertyType(usize, usize, DataType),
    UpdatePropertyRequired(usize, usize, bool),
    UpdatePropertyDescription(usize, usize, String),

    // Property validation parameters
    UpdatePropertyMinLength(usize, usize, String),
    UpdatePropertyMaxLength(usize, usize, String),
    UpdatePropertyPattern(usize, usize, String),
    UpdatePropertyFormat(usize, usize, String),
    UpdatePropertyMinimum(usize, usize, String),
    UpdatePropertyMaximum(usize, usize, String),
    UpdatePropertyMinItems(usize, usize, String),
    UpdatePropertyMaxItems(usize, usize, String),
    UpdatePropertyContentMediaType(usize, usize, String),

    // Index operations
    AddIndex(usize),
    RemoveIndex(usize, usize),
    UpdateIndexName(usize, usize, String),
    UpdateIndexUnique(usize, usize, bool),
    AddIndexProperty(usize, usize),
    RemoveIndexProperty(usize, usize, usize),
    UpdateIndexPropertyField(usize, usize, usize, String),

    // AI operations
    UpdateAiPrompt(String),
    GenerateWithAi,
    AiGenerationComplete(String),
    AiGenerationError(String),

    // Import/Export operations
    UpdateImportJson(String),
    ImportJson,
    ExportJson,
    ToggleJsonFormat,
    Clear,

    // Validation
    ValidateContract,
    ValidationComplete(Vec<ValidationError>),

    // Popup operations
    ShowCompactPopup,
    HideCompactPopup,
    PopupContentClick, // No-op message for preventing popup close
    
    // Toggle optional fields visibility
    TogglePropertyOptions(usize, usize),
}

impl Component for App {
    type Message = AppMsg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            document_types: vec![DocumentType::default()],
            json_output: String::new(),
            validation_errors: Vec::new(),
            ai_prompt: String::new(),
            ai_loading: false,
            ai_errors: Vec::new(),
            prompt_history: Vec::new(),
            import_json: String::new(),
            show_formatted: true,
            validation_requested: false,
            show_compact_popup: false,
            expanded_property_options: std::collections::HashSet::new(),
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMsg::AddDocumentType => {
                self.document_types.push(DocumentType::default());
                self.update_json_output();
                true
            }

            AppMsg::RemoveDocumentType(index) => {
                if self.document_types.len() > 1 && index < self.document_types.len() {
                    self.document_types.remove(index);
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdateDocumentTypeName(index, name) => {
                if let Some(doc_type) = self.document_types.get_mut(index) {
                    doc_type.name = name;
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdateDocumentTypeComment(index, comment) => {
                if let Some(doc_type) = self.document_types.get_mut(index) {
                    doc_type.comment = comment;
                }
                true
            }

            AppMsg::UpdateDocumentTypeDescription(index, description) => {
                if let Some(doc_type) = self.document_types.get_mut(index) {
                    doc_type.description = description;
                }
                true
            }

            AppMsg::UpdateDocumentTypeKeywords(index, keywords) => {
                if let Some(doc_type) = self.document_types.get_mut(index) {
                    doc_type.keywords = keywords;
                }
                true
            }

            AppMsg::UpdateDocumentTypeCreatedAt(index, required) => {
                if let Some(doc_type) = self.document_types.get_mut(index) {
                    doc_type.created_at_required = required;
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdateDocumentTypeUpdatedAt(index, required) => {
                if let Some(doc_type) = self.document_types.get_mut(index) {
                    doc_type.updated_at_required = required;
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdateDocumentTypeAdditionalProperties(index, additional) => {
                if let Some(doc_type) = self.document_types.get_mut(index) {
                    doc_type.additionalProperties = additional;
                    self.update_json_output();
                }
                true
            }

            AppMsg::AddProperty(doc_index) => {
                if let Some(doc_type) = self.document_types.get_mut(doc_index) {
                    let position = doc_type.properties.len() as u64;
                    let mut property = Property::default();
                    property.position = position;
                    doc_type.add_property(property);
                    self.update_json_output();
                    
                    // Scroll to show the new property
                    if let Some(window) = web_sys::window() {
                        let document = window.document().unwrap();
                        let closure = wasm_bindgen::closure::Closure::once(Box::new(move || {
                            if let Ok(elements) = document.query_selector_all(".property-section") {
                                let length = elements.length();
                                if length > 0 {
                                    if let Some(last_element) = elements.item(length - 1) {
                                        if let Ok(element) = last_element.dyn_into::<web_sys::HtmlElement>() {
                                            // Scroll element into view
                                            element.scroll_into_view_with_bool(true);
                                            
                                            // Then adjust scroll position to account for sticky header
                                            if let Some(window_inner) = web_sys::window() {
                                                let current_scroll = window_inner.scroll_y().unwrap_or(0.0);
                                                // Scroll up by 100px to ensure property is visible below sticky header
                                                window_inner.scroll_to_with_x_and_y(0.0, current_scroll - 100.0);
                                            }
                                        }
                                    }
                                }
                            }
                        }) as Box<dyn FnOnce()>);
                        
                        window.set_timeout_with_callback_and_timeout_and_arguments_0(
                            closure.as_ref().unchecked_ref(),
                            100
                        ).unwrap();
                        
                        closure.forget();
                    }
                }
                true
            }

            AppMsg::RemoveProperty(doc_index, prop_index) => {
                if let Some(doc_type) = self.document_types.get_mut(doc_index) {
                    doc_type.remove_property(prop_index);
                    // Update positions
                    for (i, prop) in doc_type.properties.iter_mut().enumerate() {
                        prop.position = i as u64;
                    }
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyName(doc_index, prop_index, name) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.name = name;
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyType(doc_index, prop_index, data_type) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.data_type = data_type;
                    property.clear_invalid_parameters();
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyRequired(doc_index, prop_index, required) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.required = required;
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyDescription(doc_index, prop_index, description) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.description = if description.is_empty() {
                        None
                    } else {
                        Some(description)
                    };
                }
                true
            }

            // Property validation parameter updates
            AppMsg::UpdatePropertyMinLength(doc_index, prop_index, value) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.min_length = value.parse().ok();
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyMaxLength(doc_index, prop_index, value) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.max_length = value.parse().ok();
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyPattern(doc_index, prop_index, pattern) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.pattern = if pattern.is_empty() {
                        None
                    } else {
                        Some(pattern)
                    };
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyFormat(doc_index, prop_index, format) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.format = if format.is_empty() {
                        None
                    } else {
                        Some(format)
                    };
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyMinimum(doc_index, prop_index, value) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.minimum = value.parse().ok();
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyMaximum(doc_index, prop_index, value) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.maximum = value.parse().ok();
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyMinItems(doc_index, prop_index, value) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.min_items = value.parse().ok();
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyMaxItems(doc_index, prop_index, value) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.max_items = value.parse().ok();
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdatePropertyContentMediaType(doc_index, prop_index, content_type) => {
                if let Some(property) = self.get_property_mut(doc_index, prop_index) {
                    property.content_media_type = if content_type.is_empty() {
                        None
                    } else {
                        Some(content_type)
                    };
                    self.update_json_output();
                }
                true
            }

            // Index operations
            AppMsg::AddIndex(doc_index) => {
                if let Some(doc_type) = self.document_types.get_mut(doc_index) {
                    doc_type.add_index(Index::default());
                    self.update_json_output();
                    
                    // Scroll to show the new index
                    if let Some(window) = web_sys::window() {
                        let document = window.document().unwrap();
                        let closure = wasm_bindgen::closure::Closure::once(Box::new(move || {
                            if let Ok(elements) = document.query_selector_all(".index-section") {
                                let length = elements.length();
                                if length > 0 {
                                    if let Some(last_element) = elements.item(length - 1) {
                                        if let Ok(element) = last_element.dyn_into::<web_sys::HtmlElement>() {
                                            // Scroll element into view
                                            element.scroll_into_view_with_bool(true);
                                            
                                            // Then adjust scroll position to account for sticky header
                                            if let Some(window_inner) = web_sys::window() {
                                                let current_scroll = window_inner.scroll_y().unwrap_or(0.0);
                                                // Scroll up by 100px to ensure index is visible below sticky header
                                                window_inner.scroll_to_with_x_and_y(0.0, current_scroll - 100.0);
                                            }
                                        }
                                    }
                                }
                            }
                        }) as Box<dyn FnOnce()>);
                        
                        window.set_timeout_with_callback_and_timeout_and_arguments_0(
                            closure.as_ref().unchecked_ref(),
                            100
                        ).unwrap();
                        
                        closure.forget();
                    }
                }
                true
            }

            AppMsg::RemoveIndex(doc_index, index_index) => {
                if let Some(doc_type) = self.document_types.get_mut(doc_index) {
                    doc_type.remove_index(index_index);
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdateIndexName(doc_index, index_index, name) => {
                if let Some(index) = self.get_index_mut(doc_index, index_index) {
                    index.name = name;
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdateIndexUnique(doc_index, index_index, unique) => {
                if let Some(index) = self.get_index_mut(doc_index, index_index) {
                    index.unique = unique;
                    self.update_json_output();
                }
                true
            }

            AppMsg::AddIndexProperty(doc_index, index_index) => {
                if let Some(index) = self.get_index_mut(doc_index, index_index) {
                    index.add_property(String::new());
                    self.update_json_output();
                }
                true
            }

            AppMsg::RemoveIndexProperty(doc_index, index_index, prop_index) => {
                if let Some(index) = self.get_index_mut(doc_index, index_index) {
                    index.remove_property(prop_index);
                    self.update_json_output();
                }
                true
            }

            AppMsg::UpdateIndexPropertyField(doc_index, index_index, prop_index, field) => {
                if let Some(index) = self.get_index_mut(doc_index, index_index) {
                    if let Some(index_prop) = index.properties.get_mut(prop_index) {
                        index_prop.set_field(field);
                        self.update_json_output();
                    }
                }
                true
            }

            // AI operations
            AppMsg::UpdateAiPrompt(prompt) => {
                self.ai_prompt = prompt;
                true
            }

            AppMsg::GenerateWithAi => {
                if !self.ai_prompt.trim().is_empty() && !self.ai_loading {
                    self.ai_loading = true;
                    self.ai_errors.clear();

                    let prompt = self.ai_prompt.clone();
                    let existing_schema = if self.json_output.trim().is_empty() {
                        None
                    } else {
                        Some(self.json_output.clone())
                    };

                    let link = ctx.link().clone();
                    spawn_local(async move {
                        match OpenAiService::generate_contract(&prompt, existing_schema.as_deref())
                            .await
                        {
                            Ok(schema) => {
                                link.send_message(AppMsg::AiGenerationComplete(schema));
                            }
                            Err(e) => {
                                link.send_message(AppMsg::AiGenerationError(e.to_string()));
                            }
                        }
                    });
                }
                true
            }

            AppMsg::AiGenerationComplete(schema) => {
                self.ai_loading = false;
                self.prompt_history.push(self.ai_prompt.clone());
                self.ai_prompt.clear();

                // Parse the generated schema
                match JsonParser::parse_contract(&schema) {
                    Ok(document_types) => {
                        self.document_types = document_types;
                        self.validation_requested = true; // AI generation should trigger validation
                        self.update_json_output();
                    }
                    Err(e) => {
                        self.ai_errors
                            .push(format!("Failed to parse generated schema: {}", e));
                    }
                }
                true
            }

            AppMsg::AiGenerationError(error) => {
                self.ai_loading = false;
                self.ai_errors.push(error);
                true
            }

            // Import/Export operations
            AppMsg::UpdateImportJson(json) => {
                self.import_json = json;
                true
            }

            AppMsg::ImportJson => {
                if !self.import_json.trim().is_empty() {
                    match JsonParser::parse_contract(&self.import_json) {
                        Ok(document_types) => {
                            self.document_types = document_types;
                            self.validation_requested = true; // Import should trigger validation
                            self.update_json_output();
                            self.import_json.clear();
                        }
                        Err(e) => {
                            self.ai_errors.push(format!("Import failed: {}", e));
                        }
                    }
                }
                true
            }

            AppMsg::ExportJson => {
                // Copy to clipboard would be implemented here
                log::info!("Export functionality would copy JSON to clipboard");
                true
            }

            AppMsg::ToggleJsonFormat => {
                self.show_formatted = !self.show_formatted;
                self.update_json_output();
                true
            }

            AppMsg::Clear => {
                self.document_types = vec![DocumentType::default()];
                self.json_output.clear();
                self.validation_errors.clear();
                self.import_json.clear();
                self.validation_requested = false; // Reset validation state
                self.update_json_output();
                true
            }

            // Validation
            AppMsg::ValidateContract => {
                // First update the JSON output
                self.update_json_output();

                // Mark that validation has been explicitly requested AFTER update
                self.validation_requested = true;

                if !self.json_output.trim().is_empty() {
                    let json = self.json_output.clone();
                    let link = ctx.link().clone();

                    spawn_local(async move {
                        let errors = match ValidationService::validate_schema(&json) {
                            Ok(errors) => errors,
                            Err(e) => vec![ValidationError::schema_error("".to_string(), e)],
                        };

                        link.send_message(AppMsg::ValidationComplete(errors));
                    });
                }
                true
            }

            AppMsg::ValidationComplete(errors) => {
                self.validation_errors = errors;
                true
            }

            AppMsg::ShowCompactPopup => {
                self.show_compact_popup = true;
                true
            }

            AppMsg::HideCompactPopup => {
                self.show_compact_popup = false;
                true
            }

            AppMsg::PopupContentClick => {
                // No-op: prevents popup from closing when clicking inside content
                false
            }
            
            AppMsg::TogglePropertyOptions(doc_index, prop_index) => {
                let key = (doc_index, prop_index);
                if self.expanded_property_options.contains(&key) {
                    self.expanded_property_options.remove(&key);
                } else {
                    self.expanded_property_options.insert(key);
                }
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        html! {
            <main class="home">
                <body>
                    { self.view_header() }
                    { self.view_ai_section(ctx) }
                    <div class="columns">
                        <div class="column-left">
                            <div class="column-text">
                                <img src="https://media.dash.org/wp-content/uploads/icon-left.svg" />
                                <p>{"Use the left column to build, edit, and submit a data contract."}</p>
                            </div>
                            { self.view_form_section(ctx) }
                            <div class="button-container">
                                <button class="button2" onclick={ctx.link().callback(|_| AppMsg::AddDocumentType)}>
                                    <span>{"+"}</span>{"Add document type"}
                                </button>
                            </div>
                            <div class="footnotes"></div>
                        </div>
                        <div class="column-right">
                            <div class="column-text">
                                <img src="https://media.dash.org/wp-content/uploads/icon-left.svg" class="rotate-180" />
                                <p>{"Use the right column to copy the generated data contract to your clipboard or import a contract."}</p>
                            </div>
                            { self.view_output_section(ctx) }
                        </div>
                    </div>
                    { self.view_footer() }
                    { self.view_compact_popup(ctx) }
                </body>
            </main>
        }
    }
}

// Implementation continues in next part due to length...
impl App {
    /// Helper to get mutable reference to a property
    fn get_property_mut(&mut self, doc_index: usize, prop_index: usize) -> Option<&mut Property> {
        self.document_types
            .get_mut(doc_index)?
            .properties
            .get_mut(prop_index)
    }

    /// Helper to get mutable reference to an index
    fn get_index_mut(&mut self, doc_index: usize, index_index: usize) -> Option<&mut Index> {
        self.document_types
            .get_mut(doc_index)?
            .indices
            .get_mut(index_index)
    }

    /// Updates the JSON output and resets validation state
    fn update_json_output(&mut self) {
        let json_value = JsonGenerator::generate_contract(&self.document_types);

        let new_json_output = if self.show_formatted {
            serde_json::to_string_pretty(&json_value).unwrap_or_default()
        } else {
            serde_json::to_string(&json_value).unwrap_or_default()
        };

        // Only reset validation if the JSON actually changed
        if new_json_output != self.json_output {
            self.reset_validation();
        }

        self.json_output = new_json_output;
    }

    /// Resets validation state when contract is edited
    fn reset_validation(&mut self) {
        self.validation_requested = false;
        self.validation_errors.clear();
    }

    fn view_header(&self) -> Html {
        html! {
            <div class="top-section_ai">
                <img class="logo_ai" src="https://media.dash.org/wp-content/uploads/dash-logo.svg" alt="Dash logo" width="100" height="50" />
                <h1 class="header_ai">{"Data Contract Creator"}</h1>
            </div>
        }
    }

    fn view_ai_section(&self, ctx: &Context<Self>) -> Html {
        let on_prompt_change = ctx.link().callback(|e: InputEvent| {
            let target = e.target().expect("Event should have target");
            let input = target
                .dyn_into::<HtmlInputElement>()
                .expect("Target should be input element");
            AppMsg::UpdateAiPrompt(input.value())
        });

        let onsubmit = ctx.link().callback(|event: SubmitEvent| {
            event.prevent_default();
            AppMsg::GenerateWithAi
        });

        html! {
            <div class="container_ai">
                <div class="content-container_ai">
                    <div class="input-container_ai">
                        <p>{"Generate a data contract using AI here or by filling out the form below."}</p>
                        <form onsubmit={onsubmit}>
                            { if self.json_output.is_empty() {
                                html! { <label class="padded-label">{"  Project description"}</label> }
                            } else {
                                html! { <label class="padded-label">{"  Adjustments to existing contract"}</label> }
                            }}
                            <div class="input-button-container_ai">
                                <input
                                    value={self.ai_prompt.clone()}
                                    oninput={on_prompt_change}
                                />
                                <button type="submit">{"Generate"}</button>
                            </div>
                        </form>
                    </div>
                </div>
                {
                    if self.ai_loading {
                        html! {
                            <div class="loader_ai"></div>
                        }
                    } else {
                        html! {}
                    }
                }
                {
                    if !self.ai_errors.is_empty() {
                        html! {<div class="error-text_ai">{for self.ai_errors.iter().map(|e| html!{<>{e}</>})}</div>}
                    } else {
                        html! {}
                    }
                }
            </div>
        }
    }

    fn view_form_section(&self, ctx: &Context<Self>) -> Html {
        self.view_full_form_section(ctx)
    }

    fn view_output_section(&self, ctx: &Context<Self>) -> Html {
        let json_pretty = if self.show_formatted {
            serde_json::to_string_pretty(
                &serde_json::from_str::<serde_json::Value>(&self.json_output).unwrap_or_default(),
            )
            .unwrap_or_default()
        } else {
            self.json_output.clone()
        };

        html! {
            <div class="input-container">
                <h2>{ "Contract" }</h2>

                <div>{
                    if self.import_json.is_empty() && !self.validation_errors.is_empty() {
                        html! {
                            <div>
                                { for self.validation_errors.iter().map(|error| html! {
                                    <p class="error-text">{ error.display_message() }</p>
                                }) }
                            </div>
                        }
                    } else if self.validation_errors.is_empty() && self.validation_requested {
                        html! { <p class="passed-text">{ "DPP validation passing ✓" }</p> }
                    } else {
                        html! {}
                    }
                }</div>

                <pre>
                    <textarea
                        class="textarea-whitespace"
                        id="json_output"
                        placeholder="Paste here to import"
                        value={if self.json_output.is_empty() { self.import_json.clone() } else { json_pretty }}
                        oninput={ctx.link().callback(|e: InputEvent| {
                            let target = e.target().expect("Event should have target");
                            let textarea = target.dyn_into::<web_sys::HtmlTextAreaElement>().expect("Target should be textarea");
                            AppMsg::UpdateImportJson(textarea.value())
                        })}
                    ></textarea>
                </pre>


                <p>{
                    if self.json_output.len() > 2 {
                        format!("Size: {} bytes", self.json_output.len())
                    } else {
                        "Size: 0 bytes".to_string()
                    }
                }</p>

                <div class="button-block">
                    <button class="button-clear" onclick={ctx.link().callback(|_| AppMsg::Clear)}>
                        <span class="clear">{ "X" }</span>{ "Clear" }
                    </button>
                    <button class="button-import" onclick={ctx.link().callback(|_| AppMsg::ImportJson)}>
                        <img src="https://media.dash.org/wp-content/uploads/arrow.down_.square.fill_.svg"/>
                        { "Import" }
                    </button>
                    { if !self.json_output.is_empty() {
                        html! {
                            <button class="button-compact" onclick={ctx.link().callback(|_| AppMsg::ShowCompactPopup)}>
                                { "Compact" }
                            </button>
                        }
                    } else {
                        html! {}
                    }}
                    <button class="button button-primary" onclick={ctx.link().callback(|_| AppMsg::ValidateContract)}>
                        { "Validate" }
                    </button>
                </div>

                <div class="prompt-history">
                    { if !self.prompt_history.is_empty() {
                        html! { <h3>{ "Prompt history:" }</h3> }
                    } else {
                        html! {}
                    }}
                    { for self.prompt_history.iter().map(|prompt| html! {
                        <div>{ prompt }</div>
                    }) }
                </div>
            </div>
        }
    }

    fn view_footer(&self) -> Html {
        html! {
            <footer>
                <a href="https://github.com/dashpay/data-contract-creator" target="_blank">
                    <div class="icon-el github"></div>
                </a>
                <p>{ "© 2023 Dashpay" }</p>
            </footer>
        }
    }

    fn view_compact_popup(&self, ctx: &Context<Self>) -> Html {
        if !self.show_compact_popup {
            return html! {};
        }

        // Generate compact JSON without whitespace
        let json_value = JsonGenerator::generate_contract(&self.document_types);
        let compact_json = serde_json::to_string(&json_value).unwrap_or_default();

        html! {
            <div class="popup-overlay" onclick={ctx.link().callback(|_| AppMsg::HideCompactPopup)}>
                <div class="popup-content" onclick={ctx.link().callback(|e: MouseEvent| {
                    e.stop_propagation();
                    AppMsg::PopupContentClick
                })}>
                    <div class="popup-header">
                        <h3>{ "Compact JSON (Without Whitespace)" }</h3>
                        <button class="popup-close" onclick={ctx.link().callback(|_| AppMsg::HideCompactPopup)}>
                            { "×" }
                        </button>
                    </div>
                    <div class="popup-body">
                        <textarea 
                            class="popup-textarea" 
                            readonly=true 
                            value={compact_json.clone()}
                        ></textarea>
                        <p class="popup-size">
                            { format!("Size: {} bytes", compact_json.len()) }
                        </p>
                    </div>
                </div>
            </div>
        }
    }
}
