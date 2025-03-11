const { execSync } = require("child_process");

const scripts = [
  "rm -rf out",
  "mkdir out",
  "rustup target add wasm32-unknown-unknown",
  "cargo build --target wasm32-unknown-unknown --release",
  "wasm-bindgen --out-dir ./out --target web target/wasm32-unknown-unknown/release/grasm_lib.wasm",
];

for (const script of scripts) {
  console.log(`$ ${script}`);
  try {
    execSync(script, { stdio: "inherit" });
  } catch (err) {
    console.error(err);
    process.exit(1);
  }
}
