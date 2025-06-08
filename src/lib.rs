//! Dash Platform Data Contract Creator
//!
//! A web application for generating and editing Dash Platform data contracts.
//! Supports both manual form-based creation and AI-assisted generation using GPT.
//!
//! ## Features
//! - Visual form-based data contract creation
//! - AI-powered contract generation with OpenAI
//! - Real-time validation against Dash Platform Protocol
//! - Import/export functionality for existing contracts
//! - Support for complex nested properties and indices
//!
//! ## Architecture
//! - **Components**: UI components built with Yew framework
//! - **Services**: Business logic for API calls and validation
//! - **Types**: Data structures and type definitions
//! - **Utils**: Helper functions and constants

pub mod components;
pub mod services;
pub mod types;
pub mod utils;

use wasm_bindgen::prelude::*;

/// Initialize the application
#[wasm_bindgen(start)]
pub fn run_app() {
    wasm_logger::init(wasm_logger::Config::default());
    log::info!("Starting Dash Platform Data Contract Creator");
    yew::Renderer::<components::App>::new().render();
}
