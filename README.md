<p align="center">
    <img alt="babel" src="https://upload.wikimedia.org/wikipedia/commons/b/bf/Dash_logo_2018_rgb_for_screens.png" width="400">
</p>

# Data Contract Creator

This is a web app, hosted at [dashpay.io](https://dashpay.io/), to help people intuitively create and edit Dash Platform data contracts. There are three options available for getting started. Users can:

1. Use ChatGPT to automatically generate a data contract based on the provided context.
2. Manually fill out a dynamic form.
3. Import an existing data contract and edit it.

The app also validates the data contracts against Dash Platform Protocol, so users can save money by validating before actually submitting data contracts to Dash Platform.

This app is built in Rust using the Yew framework and WebAssembly.

## Features

- Use ChatGPT to generate and modify Platform-compliant (usually) data contracts
- Dynamically generate and modify data contracts using a web interface
- Import existing data contract schemas for editing
- Validate data contract schemas against Dash Platform Protocol rules

## Usage

### ChatGPT

1. Paste your OpenAI API key into the field at the top left.
2. Describe your app in a few words or sentences and press return.
3. Have a look at the generated contract and note any changes you'd like to make.
4. It's recommended to make changes manually using the dynamic form, but you may also describe changes to the AI.

### Dynamic form

1. Use the dynamic form on the left to add, edit, or remove document types, properties, and indexes manually.
2. Once finished, click the "Submit" button.
3. View the generated contract and potential validation errors with the right-side interface.

### Import a Data Contract

1. If the right-side text area is already populated, click the "Clear" button.
2. Paste a data contract into the right-side text area.
3. Click the "Import" button. The dynamic form should automatically populate.

## Setup

This app is available to use at [dashpay.io](https://dashpay.io/), however, you can also run the code locally, following these steps:

Yew environment:

1. Install WebAssembly target: `rustup target add wasm32-unknown-unknown`
2. Install Trunk: `cargo install --locked trunk`

App:

1. Clone the repository: `git clone https://github.com/dashpay/data-contract-creator.git`
2. Change into the project directory: `cd data-contract-creator`
3. **Mac users** may need to run the following commands if they have issues compiling certain libraries such as secp256k1-sys:
```
export AR_PATH=$(command -v llvm-ar)
export CLANG_PATH=$(command -v clang)
export AR=${AR_PATH} CC=${CLANG_PATH} ${BUILD_COMMAND}
export AR=${AR_PATH} CC=${CLANG_PATH} ${BINDGEN_COMMAND}
```
4. Start the app `trunk serve --open`

## Future work

Once a wallet capable of authentication is available for Dash Platform, this app should integrate a "connect wallet" button so the generated data contract can be directly registered on Dash Platform from [dashpay.io](https://dashpay.io/).

## Contributing

Contributions are welcome! Please submit a pull request or open an issue if you encounter any problems or have suggestions for improvement.

## License

This project is licensed under the [MIT License](https://opensource.org/licenses/MIT).
