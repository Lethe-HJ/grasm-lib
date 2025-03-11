<template>
  <div class="polygon-demo">
    <h1>点在多边形内算法 - 扫描线可视化</h1>
    
    <div class="legend">
      <div class="legend-item">
        <div class="color-box" style="background-color: #000;"></div>
        <span>多边形边界</span>
      </div>
      <div class="legend-item">
        <div class="color-box" style="background-color: #f00;"></div>
        <span>内部点</span>
      </div>
      <div class="legend-item">
        <div class="color-box" style="background-color: #00f;"></div>
        <span>外部点</span>
      </div>
    </div>
    
    <div class="controls">
      <el-button @click="zoomIn">放大</el-button>
      <el-button @click="zoomOut">缩小</el-button>
      <el-button @click="toggleSamplePoints">
        {{ showSamplePoints ? '隐藏采样点' : '显示采样点' }}
      </el-button>
      <el-button @click="increaseResolution">增加点密度</el-button>
      <el-button @click="decreaseResolution">减少点密度</el-button>
    </div>
    
    <div id="performanceInfo" class="performance-info">{{ performanceInfo }}</div>
    
    <canvas ref="canvas" width="600" height="600"></canvas>
    
    <el-card class="details">
      <p>测试多边形: 大圆(半径为5，中心在(0,0))，带有两个小圆洞(半径为1，中心在(-2,0)和(2,0))</p>
      <p>测试点: 在[-10,10]x[-10,10]区域内均匀分布的点网格</p>
      <p>判定规则: 点在大圆内且不在任何小圆内视为多边形内部(红色)，否则视为外部(蓝色)</p>
      <p>性能优化提示: 使用"减少点密度"按钮可以提升渲染速度，或通过缩放查看更多细节</p>
    </el-card>
  </div>
</template>

<script setup>
import { ref, onMounted, onBeforeUnmount } from 'vue'

// 响应式状态
const canvas = ref(null);
const performanceInfo = ref('');
const showSamplePoints = ref(true);

// 状态变量
let gl = null;
let scale = 25; // 初始缩放
let translateX = 0;
let translateY = 0;
let currentStep = 0.5; // 初始采样步长
let range = 10.0;
let flatPoints = null;
let result = null;
let isDragging = false;
let lastX, lastY;

// 着色器程序变量
let program = null;
let positionLocation = null;
let resolutionLocation = null;
let colorLocation = null;
let pointSizeLocation = null;
let translateLocation = null;
let scaleLocation = null;
let positionBuffer = null;
let insidePointsBuffer = null;
let outsidePointsBuffer = null;

// WASM模块
let wasmModule = null;

// 方法
const zoomIn = () => {
  scale *= 1.5;
  resetPerformanceInfo();
  draw();
};

const zoomOut = () => {
  scale /= 1.5;
  resetPerformanceInfo();
  draw();
};

const toggleSamplePoints = () => {
  showSamplePoints.value = !showSamplePoints.value;
  resetPerformanceInfo();
  draw();
};

const increaseResolution = () => {
  if (currentStep > 0.06) {
    currentStep /= 2;
    generatePointsAndCompute();
    draw();
  } else {
    alert('点密度已达到最大值，再增加可能导致性能问题！');
  }
};

const decreaseResolution = () => {
  if (currentStep < 5.0) {
    currentStep *= 2;
    generatePointsAndCompute();
    draw();
  }
};

const resetPerformanceInfo = () => {
  const parts = performanceInfo.value.split('|');
  if (parts.length > 3) {
    performanceInfo.value = parts.slice(0, 3).join('|');
  }
};

// 导入WebAssembly模块并初始化
const initWasm = async () => {
  try {
    // 导入WASM模块
    wasmModule = await import('grasm-lib');
    // wasmModule = await import('../../');
    await wasmModule.default();
    return true;
  } catch (error) {
    console.error("加载WASM模块失败:", error);
    return false;
  }
};

// 生成多边形数据
const generatePolygon = () => {
  const segments = 64*2; // 每个圆的线段数
  let polygon = [];
  
  // 构造外部大圆 (0,0) r=5
  for (let i = 0; i < segments; i++) {
    const angle = 2.0 * Math.PI * i / segments;
    polygon.push(5.0 * Math.cos(angle)); // x
    polygon.push(5.0 * Math.sin(angle)); // y
  }
  
  // 构造第一个小圆洞 (-2,0) r=1
  for (let i = 0; i < segments; i++) {
    const angle = 2.0 * Math.PI * i / segments;
    polygon.push(-2.0 + Math.cos(angle)); // x
    polygon.push(Math.sin(angle)); // y
  }
  
  // 构造第二个小圆洞 (2,0) r=1
  for (let i = 0; i < segments; i++) {
    const angle = 2.0 * Math.PI * i / segments;
    polygon.push(2.0 + Math.cos(angle)); // x
    polygon.push(Math.sin(angle)); // y
  }
  
  return {
    polygon: new Float32Array(polygon),
    rings: new Uint32Array([segments, segments * 2])
  };
};

// 生成测试点并计算结果
const generatePointsAndCompute = () => {
  const testPoints = [];
  
  // 创建测试点网格(采样)
  for (let x = -range; x <= range; x += currentStep) {
    for (let y = -range; y <= range; y += currentStep) {
      testPoints.push(x);
      testPoints.push(y);
    }
  }
  
  flatPoints = new Float32Array(testPoints);
  
  // 执行点在多边形内测试
  console.log(`开始计算点在多边形内判定 (步长: ${currentStep})...`);
  const start = performance.now();
  
  const polygonData = generatePolygon();
  result = wasmModule.point_in_polygon_scanline(
    flatPoints, 
    polygonData.polygon, 
    polygonData.rings, 
    true
  );
  
  const end = performance.now();
  const duration = end - start;
  console.log(`计算完成，用时: ${duration} 毫秒`);
  
  // 更新性能信息
  performanceInfo.value = `点数量: ${flatPoints.length/2} | 计算用时: ${duration.toFixed(2)}ms | 步长: ${currentStep}`;
  
  return duration;
};

// 初始化WebGL
const initWebGL = () => {
  const canvasEl = canvas.value;
  gl = canvasEl.getContext('webgl');
  
  if (!gl) {
    alert('您的浏览器不支持WebGL');
    return false;
  }
  
  // 创建着色器
  const vertexShaderSource = /* glsl */`
    attribute vec2 a_position;
    uniform vec2 u_resolution;
    uniform float u_pointSize;
    uniform vec2 u_translate;
    uniform float u_scale;
    
    void main() {
      // 应用缩放和平移
      vec2 scaledPos = a_position * u_scale + u_translate;
      
      // 从像素坐标转换到裁剪空间
      vec2 clipSpace = (scaledPos / u_resolution) * 2.0 - 1.0;
      gl_Position = vec4(clipSpace * vec2(1, -1), 0, 1);
      gl_PointSize = u_pointSize;
    }
  `;
  
  const fragmentShaderSource = /* glsl */`
    precision mediump float;
    uniform vec4 u_color;
    
    void main() {
      gl_FragColor = u_color;
    }
  `;
  
  // 创建和编译着色器
  const createShader = (gl, type, source) => {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      console.error('着色器编译错误:', gl.getShaderInfoLog(shader));
      gl.deleteShader(shader);
      return null;
    }
    
    return shader;
  };
  
  const createProgram = (gl, vertexShader, fragmentShader) => {
    const program = gl.createProgram();
    gl.attachShader(program, vertexShader);
    gl.attachShader(program, fragmentShader);
    gl.linkProgram(program);
    
    if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
      console.error('程序链接错误:', gl.getProgramInfoLog(program));
      return null;
    }
    
    return program;
  };
  
  const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource);
  const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource);
  program = createProgram(gl, vertexShader, fragmentShader);
  gl.useProgram(program);
  
  // 获取属性和全局变量位置
  positionLocation = gl.getAttribLocation(program, 'a_position');
  resolutionLocation = gl.getUniformLocation(program, 'u_resolution');
  colorLocation = gl.getUniformLocation(program, 'u_color');
  pointSizeLocation = gl.getUniformLocation(program, 'u_pointSize');
  translateLocation = gl.getUniformLocation(program, 'u_translate');
  scaleLocation = gl.getUniformLocation(program, 'u_scale');
  
  // 设置分辨率
  gl.uniform2f(resolutionLocation, canvasEl.width, canvasEl.height);
  
  // 创建缓冲区
  positionBuffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.enableVertexAttribArray(positionLocation);
  gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
  
  // 创建点缓冲区
  insidePointsBuffer = gl.createBuffer();
  outsidePointsBuffer = gl.createBuffer();
  
  // 设置初始平移
  translateX = canvasEl.width / 2;
  translateY = canvasEl.height / 2;
  
  return true;
};

// 获取视口范围内的点
const getVisiblePointsRange = () => {
  // 将屏幕坐标转换为世界坐标
  const left = (0 - translateX) / scale;
  const right = (canvas.value.width - translateX) / scale;
  const top = (0 - translateY) / scale;
  const bottom = (canvas.value.height - translateY) / scale;
  
  // 扩大一些范围，确保边缘点也能被渲染
  const margin = 2 * currentStep;
  
  return {
    minX: left - margin,
    maxX: right + margin, 
    minY: top - margin,
    maxY: bottom + margin
  };
};

// 准备批量绘制的点数据
const preparePointBatches = () => {
  const viewport = getVisiblePointsRange();
  
  const insidePoints = [];
  const outsidePoints = [];
  
  // 当前视口中的点计数
  let visiblePointCount = 0;
  
  // 根据视口范围过滤点
  for (let i = 0; i < flatPoints.length / 2; i++) {
    const x = flatPoints[i * 2];
    const y = flatPoints[i * 2 + 1];
    
    // 仅处理视口中的点
    if (x >= viewport.minX && x <= viewport.maxX && 
        y >= viewport.minY && y <= viewport.maxY) {
      visiblePointCount++;
      
      if (result[i] === 1) {
        insidePoints.push(x, y);
      } else {
        outsidePoints.push(x, y);
      }
    }
  }
  
  return {
    inside: new Float32Array(insidePoints),
    outside: new Float32Array(outsidePoints),
    count: visiblePointCount
  };
};

// 绘制函数
const draw = () => {
  if (!gl) return;
  
  const drawStart = performance.now();
  
  // 清空画布
  gl.clearColor(0.9, 0.9, 0.9, 1.0);
  gl.clear(gl.COLOR_BUFFER_BIT);
  
  // 更新变换
  gl.uniform2f(translateLocation, translateX, translateY);
  gl.uniform1f(scaleLocation, scale);
  
  const polygonData = generatePolygon();
  const segments = polygonData.rings[0];
  
  // 绘制多边形
  gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
  gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
  
  // 绘制大圆轮廓
  const bigCirclePoints = new Float32Array(
    Array.from(polygonData.polygon).slice(0, segments * 2)
  );
  gl.bufferData(gl.ARRAY_BUFFER, bigCirclePoints, gl.STATIC_DRAW);
  gl.uniform4f(colorLocation, 0, 0, 0, 1); // 黑色
  gl.drawArrays(gl.LINE_LOOP, 0, segments);
  
  // 绘制第一个小圆洞
  const hole1Points = new Float32Array(
    Array.from(polygonData.polygon).slice(segments * 2, segments * 4)
  );
  gl.bufferData(gl.ARRAY_BUFFER, hole1Points, gl.STATIC_DRAW);
  gl.drawArrays(gl.LINE_LOOP, 0, segments);
  
  // 绘制第二个小圆洞
  const hole2Points = new Float32Array(
    Array.from(polygonData.polygon).slice(segments * 4, segments * 6)
  );
  gl.bufferData(gl.ARRAY_BUFFER, hole2Points, gl.STATIC_DRAW);
  gl.drawArrays(gl.LINE_LOOP, 0, segments);
  
  // 仅当showSamplePoints为true时绘制采样点
  if (showSamplePoints.value) {
    // 优化：只绘制视口中的点，并且批量绘制
    const pointBatches = preparePointBatches();
    
    // 设置点大小
    gl.uniform1f(pointSizeLocation, 4.0);
    
    // 批量绘制内部点(红色)
    if (pointBatches.inside.length > 0) {
      gl.bindBuffer(gl.ARRAY_BUFFER, insidePointsBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, pointBatches.inside, gl.STATIC_DRAW);
      gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
      gl.uniform4f(colorLocation, 1, 0, 0, 0.7); // 红色
      gl.drawArrays(gl.POINTS, 0, pointBatches.inside.length / 2);
    }
    
    // 批量绘制外部点(蓝色)
    if (pointBatches.outside.length > 0) {
      gl.bindBuffer(gl.ARRAY_BUFFER, outsidePointsBuffer);
      gl.bufferData(gl.ARRAY_BUFFER, pointBatches.outside, gl.STATIC_DRAW);
      gl.vertexAttribPointer(positionLocation, 2, gl.FLOAT, false, 0, 0);
      gl.uniform4f(colorLocation, 0, 0, 1, 0.7); // 蓝色
      gl.drawArrays(gl.POINTS, 0, pointBatches.outside.length / 2);
    }
    
    // 更新性能信息
    const drawEnd = performance.now();
    const renderTime = drawEnd - drawStart;
    
    // 获取当前性能信息文本，并添加渲染信息
    const currentInfo = performanceInfo.value;
    performanceInfo.value = `${currentInfo} | 渲染用时: ${renderTime.toFixed(2)}ms | 可见点: ${pointBatches.count}`;
  }
};

// 设置鼠标事件处理
const setupMouseEvents = () => {
  const canvasEl = canvas.value;
  
  const handleMouseDown = (e) => {
    isDragging = true;
    lastX = e.clientX;
    lastY = e.clientY;
  };
  
  const handleMouseMove = (e) => {
    if (isDragging) {
      const deltaX = e.clientX - lastX;
      const deltaY = e.clientY - lastY;
      translateX += deltaX;
      translateY += deltaY;
      lastX = e.clientX;
      lastY = e.clientY;
      
      resetPerformanceInfo();
      draw();
    }
  };
  
  const handleMouseUp = () => {
    isDragging = false;
  };
  
  const handleMouseLeave = () => {
    isDragging = false;
  };
  
  const handleWheel = (e) => {
    e.preventDefault();
    
    // 获取鼠标位置
    const rect = canvasEl.getBoundingClientRect();
    const mouseX = e.clientX - rect.left;
    const mouseY = e.clientY - rect.top;
    
    // 计算鼠标在缩放前的世界坐标
    const worldX = (mouseX - translateX) / scale;
    const worldY = (mouseY - translateY) / scale;
    
    // 应用缩放
    const zoomFactor = e.deltaY > 0 ? 0.9 : 1.1;
    scale *= zoomFactor;
    
    // 计算新的平移量，使鼠标位置保持在同一世界坐标
    translateX = mouseX - worldX * scale;
    translateY = mouseY - worldY * scale;
    
    resetPerformanceInfo();
    draw();
  };
  
  canvasEl.addEventListener('mousedown', handleMouseDown);
  canvasEl.addEventListener('mousemove', handleMouseMove);
  canvasEl.addEventListener('mouseup', handleMouseUp);
  canvasEl.addEventListener('mouseleave', handleMouseLeave);
  canvasEl.addEventListener('wheel', handleWheel);
  
  // 返回清理函数
  return () => {
    canvasEl.removeEventListener('mousedown', handleMouseDown);
    canvasEl.removeEventListener('mousemove', handleMouseMove);
    canvasEl.removeEventListener('mouseup', handleMouseUp);
    canvasEl.removeEventListener('mouseleave', handleMouseLeave);
    canvasEl.removeEventListener('wheel', handleWheel);
  };
};

// 初始化演示
const initDemo = async () => {
  const wasmLoaded = await initWasm();
  if (!wasmLoaded) {
    alert('无法加载WebAssembly模块，请检查浏览器控制台');
    return;
  }
  
  const glInitialized = initWebGL();
  if (!glInitialized) return;
  
  generatePointsAndCompute();
  draw();
};

// 生命周期钩子
onMounted(() => {
  initDemo();
  const cleanup = setupMouseEvents();
  
  // 确保在组件卸载时清理事件监听器
  onBeforeUnmount(cleanup);
});
</script>

<style scoped>
.polygon-demo {
  width: 100%;
}
canvas {
  border: 1px solid #ccc;
  margin-bottom: 20px;
  display: block;
}
.legend {
  display: flex;
  gap: 20px;
  margin-bottom: 20px;
}
.legend-item {
  display: flex;
  align-items: center;
  gap: 5px;
}
.color-box {
  width: 15px;
  height: 15px;
}
.controls {
  margin: 20px 0;
}
.controls button {
  margin-right: 10px;
}
.performance-info {
  margin: 10px 0;
  font-weight: bold;
}
.details {
  margin-top: 20px;
  line-height: 1.5;
}
</style> 