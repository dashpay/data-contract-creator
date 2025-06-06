// View implementation methods for the App component
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::prelude::*;
use yew::events::Event;

use super::app::{App, AppMsg};
use crate::types::DataType;

impl App {
    pub fn view_full_form_section(&self, ctx: &Context<Self>) -> Html {
        html! {
            <>
                { self.view_document_types(ctx) }
            </>
        }
    }

    fn view_document_types(&self, ctx: &Context<Self>) -> Html {
        html! {
            <div>
                { for self.document_types.iter().enumerate().map(|(i, _)| {
                    self.view_document_type_full(ctx, i)
                }) }
            </div>
        }
    }

    fn view_document_type_full(&self, ctx: &Context<Self>, index: usize) -> Html {
        let doc_type = &self.document_types[index];

        html! {
            <>
                <div class="input-container">
                    <div class="doc-block">
                        <input
                            class="doc-name-input"
                            type="text"
                            placeholder="Enter document type name"
                            value={doc_type.name.clone()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                                let target = e.target().expect("Event should have target");
                                let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                AppMsg::UpdateDocumentTypeName(index, input.value())
                            })}
                        />
                        <button
                            class="button remove"
                            onclick={ctx.link().callback(move |_| AppMsg::RemoveDocumentType(index))}
                        >
                            <img src="https://media.dash.org/wp-content/uploads/trash-icon.svg"/>
                        </button>
                    </div>
                    
                    <div class="doc-content">

                        <div>
                        <div class="form-line">
                            <h3>{ "Properties" }</h3>
                            { for doc_type.properties.iter().enumerate().map(|(i, _)| {
                                self.view_property_full(ctx, index, i)
                            }) }
                            <div class="add-index">
                                <button
                                    class="button property"
                                    onclick={ctx.link().callback(move |_| AppMsg::AddProperty(index))}
                                >
                                    <span class="plus">{ "+" }</span>
                                    { "Add property" }
                                </button>
                            </div>
                        </div>

                        <div class="forms-line-checkboxes">
                            <label class="container-checkbox second-checkbox">
                                { "Require $createdAt   " }
                                <input
                                    type="checkbox"
                                    checked={doc_type.created_at_required}
                                    onchange={ctx.link().callback(move |e: Event| {
                                        let target = e.target().expect("Event should have target");
                                        let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                        AppMsg::UpdateDocumentTypeCreatedAt(index, input.checked())
                                    })}
                                />
                                <span class="checkmark"></span>
                            </label>
                            <label class="container-checkbox second-checkbox">
                                { "Require $updatedAt   " }
                                <input
                                    type="checkbox"
                                    checked={doc_type.updated_at_required}
                                    onchange={ctx.link().callback(move |e: Event| {
                                        let target = e.target().expect("Event should have target");
                                        let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                        AppMsg::UpdateDocumentTypeUpdatedAt(index, input.checked())
                                    })}
                                />
                                <span class="checkmark"></span>
                            </label>
                        </div>
                    </div>

                    <div>
                        <h3>{ "Indices" }</h3>
                        { for doc_type.indices.iter().enumerate().map(|(i, _)| {
                            self.view_index_full(ctx, index, i)
                        }) }
                        <div class="forms-line">
                            <div class="add-index">
                                <button
                                    class="button property"
                                    onclick={ctx.link().callback(move |_| AppMsg::AddIndex(index))}
                                >
                                    <span class="plus">{ "+" }</span>
                                    { "Add index" }
                                </button>
                            </div>
                        </div>
                    </div>

                    <div>
                        <h3>{ "Description" }</h3>
                        <textarea
                            class="textarea-description"
                            placeholder="Describe what this document type represents..."
                            value={doc_type.description.clone()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                                let target = e.target().expect("Event should have target");
                                let textarea = target.dyn_into::<web_sys::HtmlTextAreaElement>().expect("Target should be textarea element");
                                AppMsg::UpdateDocumentTypeDescription(index, textarea.value())
                            })}
                        ></textarea>
                    </div>

                    <div>
                        <h3>{ "Keywords" }</h3>
                        <input
                            type="text2"
                            placeholder="Enter keywords separated by commas..."
                            value={doc_type.keywords.clone()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                                let target = e.target().expect("Event should have target");
                                let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                AppMsg::UpdateDocumentTypeKeywords(index, input.value())
                            })}
                        />
                    </div>

                    <div>
                        <h3>{ "Comment" }</h3>
                        <input
                            type="text2"
                            placeholder="Internal comment for developers..."
                            value={doc_type.comment.clone()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                                let target = e.target().expect("Event should have target");
                                let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                AppMsg::UpdateDocumentTypeComment(index, input.value())
                            })}
                        />
                    </div>
                </div>
                    </div>
                <br/>
            </>
        }
    }

    fn view_property_full(&self, ctx: &Context<Self>, doc_index: usize, prop_index: usize) -> Html {
        let property = &self.document_types[doc_index].properties[prop_index];
        let data_type_options = vec!["String", "Integer", "Array", "Object", "Number", "Boolean"];
        let selected_data_type = match property.data_type {
            DataType::String => "String",
            DataType::Integer => "Integer",
            DataType::Array => "Array",
            DataType::Object => "Object",
            DataType::Number => "Number",
            DataType::Boolean => "Boolean",
        };
        let is_expanded = self.expanded_property_options.contains(&(doc_index, prop_index));

        html! {
            <div class="property-section">
                <div class="properties-block">
                    <input
                        class="name-input-header"
                        type="text"
                        placeholder="Enter property name"
                        value={property.name.clone()}
                        oninput={ctx.link().callback(move |e: InputEvent| {
                            let target = e.target().expect("Event should have target");
                            let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                            AppMsg::UpdatePropertyName(doc_index, prop_index, input.value())
                        })}
                    />
                    <button
                        class="button remove"
                        onclick={ctx.link().callback(move |_| AppMsg::RemoveProperty(doc_index, prop_index))}
                    >
                        <img src="https://media.dash.org/wp-content/uploads/trash-icon.svg"/>
                    </button>
                </div>

                <div class="forms-line-names">
                    <div class="form-headers-type">
                        <label>{ "Type" }</label>
                        <select
                            value={selected_data_type}
                            onchange={ctx.link().callback(move |e: Event| {
                                let target = e.target().expect("Event should have target");
                                let select = target.dyn_into::<HtmlSelectElement>().expect("Target should be select element");
                                let data_type = match select.value().as_str() {
                                    "String" => DataType::String,
                                    "Integer" => DataType::Integer,
                                    "Array" => DataType::Array,
                                    "Object" => DataType::Object,
                                    "Number" => DataType::Number,
                                    "Boolean" => DataType::Boolean,
                                    _ => DataType::String,
                                };
                                AppMsg::UpdatePropertyType(doc_index, prop_index, data_type)
                            })}
                        >
                            { for data_type_options.iter().map(|option| html! {
                                <option value={*option} selected={*option == selected_data_type}>{ *option }</option>
                            }) }
                        </select>
                    </div>

                    <div class="form-headers checkbox-block">
                        <label>{ "Required" }</label>
                        <label class="container-checkbox">
                            <input
                                type="checkbox"
                                checked={property.required}
                                onchange={ctx.link().callback(move |e: Event| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdatePropertyRequired(doc_index, prop_index, input.checked())
                                })}
                            />
                            <span class="checkmark"></span>
                        </label>
                    </div>
                </div>

                { if property.data_type != DataType::Object {
                    html! {
                        <div class="optional-fields-section">
                            <button 
                                class="optional-fields-toggle"
                                onclick={ctx.link().callback(move |_| AppMsg::TogglePropertyOptions(doc_index, prop_index))}
                            >
                                <span class={if is_expanded { "arrow-down" } else { "arrow-right" }}>{ "▶" }</span>
                                { "Optional fields" }
                            </button>
                            
                            { if is_expanded {
                                html! {
                                    <div class="optional-fields-content">
                                        { self.render_additional_properties(ctx, doc_index, prop_index) }
                                        
                                        <div class="forms-line">
                                            <label>{ "Description " }</label>
                                            <input
                                                type="text3"
                                                value={property.description.clone().unwrap_or_default()}
                                                oninput={ctx.link().callback(move |e: InputEvent| {
                                                    let target = e.target().expect("Event should have target");
                                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                                    AppMsg::UpdatePropertyDescription(doc_index, prop_index, input.value())
                                                })}
                                            />
                                        </div>

                                        <div class="forms-line">
                                            <label>{ "Comment " }</label>
                                            <input
                                                type="text3"
                                                value={property.comment.clone().unwrap_or_default()}
                                                oninput={ctx.link().callback(move |e: InputEvent| {
                                                    let target = e.target().expect("Event should have target");
                                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                                    // Note: We don't have UpdatePropertyComment in our AppMsg, so this is a no-op for now
                                                    AppMsg::UpdatePropertyDescription(doc_index, prop_index, input.value())
                                                })}
                                            />
                                        </div>
                                    </div>
                                }
                            } else {
                                html! {}
                            }}
                        </div>
                    }
                } else {
                    html! {
                        <>
                            <h4>
                                { if !property.name.is_empty() {
                                    format!("{} property optional fields", property.name)
                                } else {
                                    format!("Property {} optional fields", prop_index + 1)
                                }}
                            </h4>
                            <div class="forms-line">
                                { self.render_additional_properties(ctx, doc_index, prop_index) }
                                
                                <div class="forms-line">
                                    <label>{ "Description " }</label>
                                    <input
                                        type="text3"
                                        value={property.description.clone().unwrap_or_default()}
                                        oninput={ctx.link().callback(move |e: InputEvent| {
                                            let target = e.target().expect("Event should have target");
                                            let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                            AppMsg::UpdatePropertyDescription(doc_index, prop_index, input.value())
                                        })}
                                    />
                                </div>

                                <div class="forms-line">
                                    <label>{ "Comment " }</label>
                                    <input
                                        type="text3"
                                        value={property.comment.clone().unwrap_or_default()}
                                        oninput={ctx.link().callback(move |e: InputEvent| {
                                            let target = e.target().expect("Event should have target");
                                            let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                            AppMsg::UpdatePropertyDescription(doc_index, prop_index, input.value())
                                        })}
                                    />
                                </div>
                                <p></p>
                            </div>
                        </>
                    }
                }}
            </div>
        }
    }

    fn render_additional_properties(
        &self,
        ctx: &Context<Self>,
        doc_index: usize,
        prop_index: usize,
    ) -> Html {
        let property = &self.document_types[doc_index].properties[prop_index];

        match property.data_type {
            DataType::String => html! {
                <>
                    <div class="forms-line number-block">
                        <div class="forms-line min">
                            <label>{ "Min length " }</label>
                            <input
                                type="number"
                                value={property.min_length.map(|n| n.to_string()).unwrap_or_default()}
                                oninput={ctx.link().callback(move |e: InputEvent| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdatePropertyMinLength(doc_index, prop_index, input.value())
                                })}
                            />
                        </div>
                        <div class="forms-line max">
                            <label>{ "Max length " }</label>
                            <input
                                type="number"
                                value={property.max_length.map(|n| n.to_string()).unwrap_or_default()}
                                oninput={ctx.link().callback(move |e: InputEvent| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdatePropertyMaxLength(doc_index, prop_index, input.value())
                                })}
                            />
                        </div>
                    </div>
                    <div class="forms-line">
                        <label>{ "RE2 pattern " }</label>
                        <input
                            type="text3"
                            value={property.pattern.clone().unwrap_or_default()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                                let target = e.target().expect("Event should have target");
                                let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                AppMsg::UpdatePropertyPattern(doc_index, prop_index, input.value())
                            })}
                        />
                    </div>
                    <div class="forms-line">
                        <label>{ "Format " }</label>
                        <input
                            type="text3"
                            value={property.format.clone().unwrap_or_default()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                                let target = e.target().expect("Event should have target");
                                let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                AppMsg::UpdatePropertyFormat(doc_index, prop_index, input.value())
                            })}
                        />
                    </div>
                </>
            },
            DataType::Integer | DataType::Number => html! {
                <>
                    <div class="forms-line number-block">
                        <div class="forms-line min">
                            <label>{ "Minimum " }</label>
                            <input
                                type="number"
                                value={property.minimum.map(|n| n.to_string()).unwrap_or_default()}
                                oninput={ctx.link().callback(move |e: InputEvent| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdatePropertyMinimum(doc_index, prop_index, input.value())
                                })}
                            />
                        </div>
                        <div class="forms-line max">
                            <label>{ "Maximum " }</label>
                            <input
                                type="number"
                                value={property.maximum.map(|n| n.to_string()).unwrap_or_default()}
                                oninput={ctx.link().callback(move |e: InputEvent| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdatePropertyMaximum(doc_index, prop_index, input.value())
                                })}
                            />
                        </div>
                    </div>
                </>
            },
            DataType::Array => html! {
                <>
                    <div class="forms-line number-block">
                        <div class="forms-line min">
                            <label>{ "Min items " }</label>
                            <input
                                type="number"
                                value={property.min_items.map(|n| n.to_string()).unwrap_or_default()}
                                oninput={ctx.link().callback(move |e: InputEvent| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdatePropertyMinItems(doc_index, prop_index, input.value())
                                })}
                            />
                        </div>
                        <div class="forms-line max">
                            <label>{ "Max items " }</label>
                            <input
                                type="number"
                                value={property.max_items.map(|n| n.to_string()).unwrap_or_default()}
                                oninput={ctx.link().callback(move |e: InputEvent| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdatePropertyMaxItems(doc_index, prop_index, input.value())
                                })}
                            />
                        </div>
                    </div>
                    <div class="forms-line">
                        <label>{ "Content media type " }</label>
                        <input
                            type="text3"
                            value={property.content_media_type.clone().unwrap_or_default()}
                            oninput={ctx.link().callback(move |e: InputEvent| {
                                let target = e.target().expect("Event should have target");
                                let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                AppMsg::UpdatePropertyContentMediaType(doc_index, prop_index, input.value())
                            })}
                        />
                    </div>
                </>
            },
            DataType::Object => html! {
                <>
                    <h4 class="black">
                        { if !property.name.is_empty() {
                            format!("{} inner properties", property.name)
                        } else {
                            format!("Property {} inner properties", prop_index + 1)
                        }}
                    </h4>
                    <div class="forms-line">
                        { "TODO: Nested properties support" }
                    </div>
                    <div class="forms-line">
                        <button class="button">{ "Add inner property" }</button>
                    </div>
                    <h4>
                        { if !property.name.is_empty() {
                            format!("{} property optional fields", property.name)
                        } else {
                            format!("Property {} optional fields", prop_index + 1)
                        }}
                    </h4>
                    <div class="forms-line number-block">
                        <div class="forms-line min">
                            <label>{ "Min properties " }</label>
                            <input
                                type="number"
                                value={property.min_properties.map(|n| n.to_string()).unwrap_or_default()}
                            />
                        </div>
                        <div class="forms-line max">
                            <label>{ "Max properties " }</label>
                            <input
                                type="number"
                                value={property.max_properties.map(|n| n.to_string()).unwrap_or_default()}
                            />
                        </div>
                    </div>
                </>
            },
            DataType::Boolean => html! {
                <>
                </>
            },
        }
    }

    fn view_index_full(&self, ctx: &Context<Self>, doc_index: usize, index_index: usize) -> Html {
        let index = &self.document_types[doc_index].indices[index_index];

        html! {
            <div class="index-section">
                <div class="properties-block">
                    <input
                        class="name-input-header"
                        type="text"
                        placeholder="Enter index name"
                        value={index.name.clone()}
                        oninput={ctx.link().callback(move |e: InputEvent| {
                            let target = e.target().expect("Event should have target");
                            let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                            AppMsg::UpdateIndexName(doc_index, index_index, input.value())
                        })}
                    />
                    <button
                        class="button remove"
                        onclick={ctx.link().callback(move |_| AppMsg::RemoveIndex(doc_index, index_index))}
                    >
                        <img src="https://media.dash.org/wp-content/uploads/trash-icon.svg"/>
                    </button>
                </div>
                
                <div class="forms-line-names">
                    <div class="form-headers checkbox-block">
                        <label>{ "Unique" }</label>
                        <label class="container-checkbox">
                            <input
                                type="checkbox"
                                checked={index.unique}
                                onchange={ctx.link().callback(move |e: Event| {
                                    let target = e.target().expect("Event should have target");
                                    let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                                    AppMsg::UpdateIndexUnique(doc_index, index_index, input.checked())
                                })}
                            />
                            <span class="checkmark"></span>
                        </label>
                    </div>
                </div>

                <div class="forms-line">
                    <h4 class="black">
                        { if !index.name.is_empty() {
                            format!("{} index properties", index.name)
                        } else {
                            format!("Index {} properties", index_index + 1)
                        }}
                    </h4>
                    <div class="form-headers">
                        { for index.properties.iter().enumerate().map(|(i, _)| {
                            self.view_index_property(ctx, doc_index, index_index, i)
                        }) }
                    </div>
                </div>
                <p></p>
                <div class="forms-line">
                    <button
                        class="button property"
                        onclick={ctx.link().callback(move |_| AppMsg::AddIndexProperty(doc_index, index_index))}
                    >
                        <span class="plus">{ "+" }</span>
                        { "Add index property" }
                    </button>
                </div>
                <p></p>
            </div>
        }
    }

    fn view_index_property(
        &self,
        ctx: &Context<Self>,
        doc_index: usize,
        index_index: usize,
        prop_index: usize,
    ) -> Html {
        let index = &self.document_types[doc_index].indices[index_index];
        let index_prop = &index.properties[prop_index];
        
        // Check if the index has a name
        let has_index_name = !index.name.is_empty();
        
        // Get user-defined properties from the current document type
        let doc_type = &self.document_types[doc_index];
        let mut property_options: Vec<String> = doc_type.properties.iter()
            .filter(|p| !p.name.is_empty())
            .map(|p| p.name.clone())
            .collect();
        
        // Add system properties
        property_options.push("$ownerId".to_string());
        property_options.push("$createdAt".to_string());
        property_options.push("$updatedAt".to_string());
        
        let selected_value = index_prop.field().to_string();

        html! {
            <div class="forms-line number-block index">
                <div class="form-headers">
                    { if has_index_name {
                        html! {
                            <select
                                value={selected_value.clone()}
                                onchange={ctx.link().callback(move |e: Event| {
                                    let target = e.target().expect("Event should have target");
                                    let select = target.dyn_into::<HtmlSelectElement>().expect("Target should be select element");
                                    AppMsg::UpdateIndexPropertyField(doc_index, index_index, prop_index, select.value())
                                })}
                            >
                                <option value="" selected={selected_value.is_empty()}>
                                    { "Select property..." }
                                </option>
                                { for property_options.iter().map(|prop| {
                                    let is_selected = &selected_value == prop;
                                    html! {
                                        <option value={prop.clone()} selected={is_selected}>
                                            { prop }
                                        </option>
                                    }
                                }) }
                            </select>
                        }
                    } else {
                        html! {
                            <select disabled=true>
                                <option>{ "Enter index name first" }</option>
                            </select>
                        }
                    }}
                </div>
            </div>
        }
    }
}
