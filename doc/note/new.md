# Rust WebAssembly 项目设置指南

## 1. 环境准备

更新 Rust stable 版本
`rustup update stable`

创建新的 Rust 库项目
`cargo new rust_wasm_lib --lib`

添加 wasm-bindgen 依赖，用于 Rust 和 JavaScript 交互
`cargo add wasm-bindgen`

添加 WebAssembly 目标平台
`rustup target add wasm32-unknown-unknown`

## 2. 编写 Rust 代码

```rs
//src/lib.rs
use wasm_bindgen::prelude::*;

// 使用 #[wasm_bindgen] 宏来导出函数到 JavaScript
#[wasm_bindgen]
pub fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

## 3. 配置 Cargo.toml

添加以下配置使项目编译为动态链接库

```toml
[lib]
crate-type = ["cdylib"]
```

## 4. 构建 WebAssembly

编译项目为 WebAssembly 格式
`cargo build --target wasm32-unknown-unknown --release`

## 5. 生成 JavaScript 绑定

安装 wasm-bindgen CLI 工具

`cargo install wasm-bindgen-cli`

生成 JavaScript 绑定文件

```bash
# 为浏览器环境生成
wasm-bindgen --out-dir ./out --target web target/wasm32-unknown-unknown/release/rust_wasm_lib.wasm

# 为 Node.js 环境生成
wasm-bindgen --out-dir ./out --target nodejs target/wasm32-unknown-unknown/release/rust_wasm_lib.wasm
```

--out-dir: 输出目录
--target: 目标平台 (web/nodejs)
最后一个参数是编译生成的 wasm 文件路径

## 6. 使用示例

### 浏览器环境

```html
<!DOCTYPE html>
<html>
  <head>
    <title>Lib WASM Demo</title>
  </head>
  <body>
    <script type="module">
      // 导入 WASM 初始化函数和导出的 greet 函数
      import init, { greet } from "./out/rust_wasm_lib.js";
      // 初始化 WASM 模块并调用 greet 函数
      init().then(() => {
        console.log(greet("World"));
      });
    </script>
  </body>
</html>
```

### Node.js 环境

创建 `test.js` 文件：
```js
const { greet } = require('./out/rust_wasm_lib.js');

async function main() {
    await greet("World");  // 输出: Hello, World!
}

main().catch(console.error);
```

执行命令：
```bash
node test.js
```

注意：
1. Node.js 版本需要支持 WebAssembly
2. 使用 --target nodejs 生成的绑定文件才能在 Node.js 中运行
3. Node.js 环境使用 require 语法而不是 import
4. 需要使用异步方式调用 WASM 函数


## 7. 运行测试

`cargo test`

`cargo test -- --nocapture`