# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Development
```bash
# Start development server with hot reload (opens browser automatically)
trunk serve --open

# Build for production
trunk build --release

# Run tests
cargo test

# Run specific test
cargo test test_name

# Check code without building
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

### Mac-specific setup for compilation issues
```bash
export AR_PATH=$(command -v llvm-ar)
export CLANG_PATH=$(command -v clang)
export AR=${AR_PATH} CC=${CLANG_PATH}
```

## Architecture

This is a Yew-based WebAssembly application for creating Dash Platform data contracts. The architecture follows a clean separation of concerns:

### Core Components
- **Yew Framework**: Component-based UI with virtual DOM, using Yew 0.21
- **State Management**: Component-level state with message passing
- **JSON Processing**: Bidirectional conversion between form data and JSON contracts
- **Validation**: Uses Dash Platform Protocol (DPP) for contract validation
- **AI Integration**: OpenAI API for generating contracts from natural language

### Key Modules
- `components/app.rs`: Main application component managing state and UI coordination
- `services/json_generator.rs`: Converts document types to valid JSON contracts
- `services/json_parser.rs`: Imports JSON contracts back to editable forms
- `services/validation.rs`: Validates contracts against DPP rules
- `services/openai.rs`: Handles AI-powered contract generation
- `types/`: Data structures (DocumentType, Property, Index) with proper defaults

### Important Patterns
- All async operations use `wasm-bindgen-futures` for browser compatibility
- Error handling uses `anyhow` for consistent error types
- JSON operations preserve field order using `serde_json` features
- Component updates trigger re-renders through Yew's message system
- Validation errors are displayed inline with helpful messages