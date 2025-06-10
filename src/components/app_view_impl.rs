// View implementation methods for the App component
use wasm_bindgen::JsCast;
use web_sys::{HtmlInputElement, HtmlSelectElement};
use yew::events::{Event, MouseEvent};
use yew::prelude::*;

use super::app::{App, AppMsg};
use crate::types::{DataType, Property};

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
                            <h3 class="header-with-info">
                                <span>{ "Properties" }</span>
                                <div class="info-icon-container">
                                    <span
                                        class="info-icon"
                                        onclick={ctx.link().callback(move |e: MouseEvent| {
                                            e.stop_propagation();
                                            AppMsg::ToggleInfoTooltip(index + 1000)
                                        })}
                                    >{ "ℹ" }</span>
                                    { if self.shown_info_tooltip == Some(index + 1000) {
                                        html! {
                                            <div class="info-tooltip visible" style="background-color: white; color: #333333; font-weight: 300;">
                                                <p style="color: #333333; font-weight: 300;">{ "Properties define the data fields that documents of this type can (or must) contain." }</p>
                                                <p style="color: #333333; font-weight: 300;">{ "Each property has a name, type, and can be marked as required or optional." }</p>
                                                <p style="color: #333333; font-weight: 300;">{ "Properties are stored on the blockchain and incur storage costs based on their size." }</p>
                                            </div>
                                        }
                                    } else {
                                        html! {}
                                    }}
                                </div>
                            </h3>
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
                        <h3 class="header-with-info">
                            <span>{ "Indices" }</span>
                            <div class="info-icon-container">
                                <span
                                    class="info-icon"
                                    onclick={ctx.link().callback(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        AppMsg::ToggleInfoTooltip(index + 2000)
                                    })}
                                >{ "ℹ" }</span>
                                { if self.shown_info_tooltip == Some(index + 2000) {
                                    html! {
                                        <div class="info-tooltip visible" style="background-color: white; color: #333333; font-weight: 300;">
                                            <p style="color: #333333; font-weight: 300;">{ "Indices enable efficient querying of documents by specific properties." }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Mark an index as 'unique' to ensure no two documents can have the same value for that property." }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Each index incurs additional storage costs but significantly improves query performance." }</p>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                        </h3>
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
                        <h3 class="header-with-info">
                            <span>{ "Keywords" }</span>
                            <div class="info-icon-container">
                                <span
                                    class="info-icon"
                                    onclick={ctx.link().callback(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        AppMsg::ToggleInfoTooltip(index)
                                    })}
                                >{ "ℹ" }</span>
                                { if self.shown_info_tooltip == Some(index) {
                                    html! {
                                        <div class="info-tooltip visible" style="background-color: white; color: #333333; font-weight: 300;">
                                            <p style="color: #333333; font-weight: 300;">{ "Keywords enable your data contract to be discoverable through searches on the Dash Platform." }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Cost: 0.1 Dash credits per keyword" }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Use relevant terms that describe your contract's purpose and functionality." }</p>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                        </h3>
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
        let is_expanded = self
            .expanded_property_options
            .contains(&(doc_index, prop_index));

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
                        <label class="label-with-info">
                            <span>{ "RE2 pattern " }</span>
                            <div class="info-icon-container inline">
                                <span 
                                    class="info-icon small"
                                    onclick={ctx.link().callback(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        AppMsg::ToggleInfoTooltip(doc_index * 10000 + prop_index + 3000)
                                    })}
                                >{ "ℹ" }</span>
                                { if self.shown_info_tooltip == Some(doc_index * 10000 + prop_index + 3000) {
                                    html! {
                                        <div class="info-tooltip visible" style="background-color: white; color: #333333; font-weight: 300;">
                                            <p style="color: #333333; font-weight: 300;">{ "RE2 is a regular expression syntax used for pattern matching." }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Examples: ^[A-Z]{3}$ for 3 uppercase letters, ^\\d{4}$ for 4 digits." }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Learn more at: github.com/google/re2/wiki/Syntax" }</p>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                        </label>
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
                        <label class="label-with-info">
                            <span>{ "Format " }</span>
                            <div class="info-icon-container inline">
                                <span 
                                    class="info-icon small"
                                    onclick={ctx.link().callback(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        AppMsg::ToggleInfoTooltip(doc_index * 10000 + prop_index + 4000)
                                    })}
                                >{ "ℹ" }</span>
                                { if self.shown_info_tooltip == Some(doc_index * 10000 + prop_index + 4000) {
                                    html! {
                                        <div class="info-tooltip visible" style="background-color: white; color: #333333; font-weight: 300;">
                                            <p style="color: #333333; font-weight: 300;">{ "Format specifies a semantic validation for string values." }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Common formats: email, uri, date, date-time, ipv4, ipv6, uuid" }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "These are validated according to JSON Schema specifications." }</p>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                        </label>
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
                        <label class="label-with-info">
                            <span>{ "Content media type " }</span>
                            <div class="info-icon-container inline">
                                <span 
                                    class="info-icon small"
                                    onclick={ctx.link().callback(move |e: MouseEvent| {
                                        e.stop_propagation();
                                        AppMsg::ToggleInfoTooltip(doc_index * 10000 + prop_index + 5000)
                                    })}
                                >{ "ℹ" }</span>
                                { if self.shown_info_tooltip == Some(doc_index * 10000 + prop_index + 5000) {
                                    html! {
                                        <div class="info-tooltip visible" style="background-color: white; color: #333333; font-weight: 300;">
                                            <p style="color: #333333; font-weight: 300;">{ "Specifies the MIME type of binary data stored in the array." }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "Common types: application/json, image/png, image/jpeg, application/pdf" }</p>
                                            <p style="color: #333333; font-weight: 300;">{ "This helps applications interpret the binary data correctly." }</p>
                                        </div>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
                        </label>
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
                    <div class="nested-properties">
                        { if let Some(properties) = &property.properties {
                            html! {
                                <>
                                    { for properties.iter().enumerate().map(|(nested_index, _)| {
                                        self.view_nested_property(ctx, doc_index, prop_index, vec![nested_index], 0)
                                    }) }
                                </>
                            }
                        } else {
                            html! {}
                        }}
                    </div>
                    <div class="forms-line">
                        <button
                            class="button"
                            onclick={ctx.link().callback(move |_| AppMsg::AddNestedProperty(doc_index, prop_index, vec![]))}
                        >
                            { "Add inner property" }
                        </button>
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
                                onclick={ctx.link().callback(move |e: MouseEvent| {
                                    e.stop_propagation();
                                    AppMsg::ToggleIndexUnique(doc_index, index_index)
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
        let mut property_options: Vec<String> = doc_type
            .properties
            .iter()
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

    fn view_nested_property(
        &self,
        ctx: &Context<Self>,
        doc_index: usize,
        prop_index: usize,
        nested_indices: Vec<usize>,
        depth: usize,
    ) -> Html {
        // Limit nesting to 2 additional layers (depth 0 is the first nested level)
        if depth >= 2 {
            return html! {};
        }

        // Get the nested property
        let property = if let Some(doc_type) = self.document_types.get(doc_index) {
            if let Some(parent_prop) = doc_type.properties.get(prop_index) {
                let mut current_prop = parent_prop;
                for &idx in &nested_indices {
                    if let Some(props) = &current_prop.properties {
                        if let Some(prop) = props.get(idx) {
                            current_prop = prop;
                        } else {
                            return html! {};
                        }
                    } else {
                        return html! {};
                    }
                }
                current_prop
            } else {
                return html! {};
            }
        } else {
            return html! {};
        };

        let nested_indices_for_remove = nested_indices.clone();
        let nested_indices_for_name = nested_indices.clone();
        let nested_indices_for_type = nested_indices.clone();
        let nested_indices_for_required = nested_indices.clone();
        let nested_indices_for_add = nested_indices.clone();
        let nested_indices_for_render = nested_indices.clone();

        let data_type_options = vec!["String", "Integer", "Array", "Object", "Number", "Boolean"];
        let selected_data_type = match property.data_type {
            DataType::String => "String",
            DataType::Integer => "Integer",
            DataType::Array => "Array",
            DataType::Object => "Object",
            DataType::Number => "Number",
            DataType::Boolean => "Boolean",
        };

        // Add indentation based on depth
        let indent_class = format!("nested-property-level-{}", depth + 1);

        html! {
            <div class={format!("property-section nested-property {}", indent_class)}>
                <div class="properties-block">
                    <input
                        class="name-input-header"
                        type="text"
                        placeholder="Enter property name"
                        value={property.name.clone()}
                        oninput={ctx.link().callback(move |e: InputEvent| {
                            let target = e.target().expect("Event should have target");
                            let input = target.dyn_into::<HtmlInputElement>().expect("Target should be input element");
                            AppMsg::UpdateNestedPropertyName(doc_index, prop_index, nested_indices_for_name.clone(), input.value())
                        })}
                    />
                    <button
                        class="button remove"
                        onclick={ctx.link().callback(move |_| {
                            let mut indices_with_self = nested_indices_for_remove.clone();
                            if let Some(last_index) = indices_with_self.last() {
                                let self_index = *last_index;
                                indices_with_self.pop();
                                indices_with_self.push(self_index);
                            }
                            AppMsg::RemoveNestedProperty(doc_index, prop_index, indices_with_self)
                        })}
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
                                AppMsg::UpdateNestedPropertyType(doc_index, prop_index, nested_indices_for_type.clone(), data_type)
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
                                    AppMsg::UpdateNestedPropertyRequired(doc_index, prop_index, nested_indices_for_required.clone(), input.checked())
                                })}
                            />
                            <span class="checkmark"></span>
                        </label>
                    </div>
                </div>

                { if property.data_type != DataType::Object && property.data_type != DataType::Boolean {
                    // Show optional fields for non-object, non-boolean types
                    html! {
                        <div class="optional-fields-section">
                            <div class="optional-fields-content">
                                { self.render_nested_additional_properties(ctx, doc_index, prop_index, nested_indices_for_render.clone(), property) }
                            </div>
                        </div>
                    }
                } else {
                    html! {}
                }}

                { if property.data_type == DataType::Object {
                    if depth < 1 {
                        // Allow one more level of nesting
                        html! {
                            <>
                                <h5 class="black">
                                    { if !property.name.is_empty() {
                                        format!("{} inner properties", property.name)
                                    } else {
                                        "Inner properties".to_string()
                                    }}
                                </h5>
                                <div class="nested-properties">
                                    { if let Some(properties) = &property.properties {
                                        html! {
                                            <>
                                                { for properties.iter().enumerate().map(|(inner_index, _)| {
                                                    let mut new_indices = nested_indices.clone();
                                                    new_indices.push(inner_index);
                                                    self.view_nested_property(ctx, doc_index, prop_index, new_indices, depth + 1)
                                                }) }
                                            </>
                                        }
                                    } else {
                                        html! {}
                                    }}
                                </div>
                                <div class="forms-line">
                                    <button
                                        class="button"
                                        onclick={ctx.link().callback(move |_| AppMsg::AddNestedProperty(doc_index, prop_index, nested_indices_for_add.clone()))}
                                    >
                                        { "Add inner property" }
                                    </button>
                                </div>
                            </>
                        }
                    } else {
                        // Maximum nesting depth reached
                        html! {
                            <div class="nesting-limit-message">
                                <p>{ "Maximum nesting depth reached. The Data Contract Creator only supports up to 3 levels of nested objects." }</p>
                            </div>
                        }
                    }
                } else {
                    html! {}
                }}
            </div>
        }
    }

    fn render_nested_additional_properties(
        &self,
        ctx: &Context<Self>,
        doc_index: usize,
        prop_index: usize,
        nested_indices: Vec<usize>,
        property: &Property,
    ) -> Html {
        // Clone nested_indices for each closure
        let nested_indices_min_len = nested_indices.clone();
        let nested_indices_max_len = nested_indices.clone();
        let nested_indices_pattern = nested_indices.clone();
        let nested_indices_format = nested_indices.clone();
        let nested_indices_minimum = nested_indices.clone();
        let nested_indices_maximum = nested_indices.clone();

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
                                    AppMsg::UpdateNestedPropertyMinLength(doc_index, prop_index, nested_indices_min_len.clone(), input.value())
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
                                    AppMsg::UpdateNestedPropertyMaxLength(doc_index, prop_index, nested_indices_max_len.clone(), input.value())
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
                                AppMsg::UpdateNestedPropertyPattern(doc_index, prop_index, nested_indices_pattern.clone(), input.value())
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
                                AppMsg::UpdateNestedPropertyFormat(doc_index, prop_index, nested_indices_format.clone(), input.value())
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
                                    AppMsg::UpdateNestedPropertyMinimum(doc_index, prop_index, nested_indices_minimum.clone(), input.value())
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
                                    AppMsg::UpdateNestedPropertyMaximum(doc_index, prop_index, nested_indices_maximum.clone(), input.value())
                                })}
                            />
                        </div>
                    </div>
                </>
            },
            DataType::Array => html! {
                <>
                    <p class="info-text">{ "Note: Arrays in Dash Platform are byte arrays" }</p>
                </>
            },
            _ => html! {},
        }
    }
}
