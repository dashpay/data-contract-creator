# Data Contract Creator

This is a simple web app written in Rust using the [Yew](https://yew.rs/) framework for generating, editing, and validating data contract schemas for Dash Platform.

## Features

- Dynamically generate and modify data contract schemas using a web interface
- Import existing data contract schemas for editing
- Validate data contract schemas against Dash Platform Protocol rules

## Setup

Yew environment:

1. Install WebAssembly target: `rustup target add wasm32-unknown-unknown`
2. Install Trunk: `cargo install --locked trunk`

App:

1. Clone the repository: `git clone https://github.com/pauldelucia/data-contract-creator.git`
2. **Mac users** may need to run the following commands if they have issues compiling certain libraries such as secp256k1-sys:
```
export AR_PATH=$(command -v llvm-ar)
export CLANG_PATH=$(command -v clang)
export AR=${AR_PATH} CC=${CLANG_PATH} ${BUILD_COMMAND}
export AR=${AR_PATH} CC=${CLANG_PATH} ${BINDGEN_COMMAND}
```
3. Change into the project directory: `cd data-contract-creator`
4. Start the app `trunk serve --open`

## Usage

### Create and Edit a Data Contract

1. Use the left-side interface to add document types, properties, and indexes
2. Click the "Submit" button
3. View the generated schema and potential validation errors with the right-side interface

### Import a Data Contract

1. If the right-side text area is already populated, click the "Clear" button
2. Paste a data contract into the right-side text area
3. Click the "Import" button

## Contributing

Contributions are welcome! Please submit a pull request or open an issue if you encounter any problems or have suggestions for improvement.

## License

This project is licensed under the [MIT License](https://opensource.org/licenses/MIT).
