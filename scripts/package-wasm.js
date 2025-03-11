const { runScripts } = require("./run-scripts");
runScripts(/*bash*/ `
  # 清空out文件夹
  rm -rf out

  # 新建out文件夹
  mkdir out

  # 添加wasm目标
  rustup target add wasm32-unknown-unknown

  # 构建wasm
  cargo build --target wasm32-unknown-unknown --release

  # 绑定wasm
  wasm-bindgen --out-dir ./out --target web target/wasm32-unknown-unknown/release/grasm_lib.wasm
`);
