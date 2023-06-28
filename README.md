# OpenAI Data Contract Creator

This is a web application built in Rust using the Yew framework and WebAssembly. It generates Dash Platform data contract JSON schemas based on user-provided prompts. The application interacts with the OpenAI API to generate the schema, and then validates it using Dash Platform Protocol (DPP).

## Prerequisites

App:

1. **Rust:** Follow the instructions on the [Rust website](https://www.rust-lang.org/tools/install) to install the Rust toolchain on your machine.
2. **[OpenAI API key](https://beta.openai.com/signup/):** Required to interact with the OpenAI API. Be sure to keep it safe.

Yew environment:

1. Install WebAssembly target: `rustup target add wasm32-unknown-unknown`
2. Install Trunk: `cargo install --locked trunk`

## Installation

1. Clone the repository:

    ```
    git clone https://github.com/dashpay/data-contract-creator.git
    cd data-contract-creator/openai
    ```

2. Add your OpenAI API key:

    In src/main.rs, set the OPENAI_API_KEY constant to your api key.

3. **Mac users** may need to run the following commands if they have issues compiling certain libraries such as secp256k1-sys:
    ```
    export AR_PATH=$(command -v llvm-ar)
    export CLANG_PATH=$(command -v clang)
    export AR=${AR_PATH} CC=${CLANG_PATH} ${BUILD_COMMAND}
    export AR=${AR_PATH} CC=${CLANG_PATH} ${BINDGEN_COMMAND}
    ```

4. Start the app `trunk serve --open`

## Usage

1. Provide a brief description of a data contract and hit "return" or press the "Generate" button. The app will generate and display a corresponding Dash Platform data contract JSON schema.

2. If you want to make any adjustments to the schema, simply provide the changes in the input field and press "return" or the "Generate" button again. 

## Features

* Utilizes the OpenAI API to generate Dash Platform data contract JSON schemas.
* Validates the generated schemas using Dash Platform Protocol (DPP).
* Displays validation errors, if any.
* Stores and displays a history of user prompts.

## License

This project is licensed under the terms of the MIT license.

## Contributions

Contributions are welcome! Please feel free to submit a Pull Request.
