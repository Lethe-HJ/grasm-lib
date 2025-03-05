## 1. 点在多边形内部判断函数

### JavaScript 调用示例

```js
// 浏览器环境
import init, { point_in_polygon } from "./out/rust_wasm_lib.js";

init().then(() => {
  // 创建点云数据
  const points = new Float32Array([1.0, 1.5, 2.5, 0.5, 4.0, 1.5]);

  // 创建多边形数据
  const polygon = new Float32Array([
    0.0,
    0.0,
    3.0,
    0.0,
    3.0,
    3.0,
    0.0,
    3.0, // 外部多边形
    1.0,
    1.0,
    2.0,
    1.0,
    2.0,
    2.0,
    1.0,
    2.0, // 内部洞
  ]);

  // 定义多边形拆分
  const rings = new Uint32Array([4]);

  // 调用 Rust 函数
  const result = point_in_polygon(points, polygon, rings, true);
  console.log(result); // 输出: Uint32Array [0, 1, 0]
});

// Node.js 环境
const { point_in_polygon } = require("./out/rust_wasm_lib.js");

async function main() {
  // 创建点云数据
  const points = new Float32Array([1.0, 1.5, 2.5, 0.5, 4.0, 1.5]);

  // 创建多边形数据
  const polygon = new Float32Array([
    0.0,
    0.0,
    3.0,
    0.0,
    3.0,
    3.0,
    0.0,
    3.0, // 外部多边形
    1.0,
    1.0,
    2.0,
    1.0,
    2.0,
    2.0,
    1.0,
    2.0, // 内部洞
  ]);

  // 定义多边形拆分
  const rings = new Uint32Array([4]);

  // 调用 Rust 函数
  const result = await point_in_polygon(points, polygon, rings, true);
  console.log(result); // 输出: Uint32Array [0, 1, 0]
}

main().catch(console.error);
```

### 参数说明

1. `points`: Float32Array - 点云坐标，格式为 [x1, y1, x2, y2, ...]
2. `polygon`: Float32Array - 多边形路径点，格式为 [x1, y1, x2, y2, ...]
3. `rings`: Uint32Array - 多边形拆分索引，例如 [4] 表示前 4 个点为外部多边形，剩余点为内部洞
4. `boundary_is_inside`: boolean - 边界上的点是否视为内部，默认为 true

### 返回值

Uint32Array - 每个点是否在多边形内部的结果，1 表示在内部，0 表示在外部
