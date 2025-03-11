cargo test

cargo build --target wasm32-unknown-unknown --release

wasm-bindgen --out-dir ./out --target web target/wasm32-unknown-unknown/release/rust_wasm_lib.wasm