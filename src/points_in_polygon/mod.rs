// 这个模块实现了判断点是否在多边形内部的算法
// 该算法支持带洞的多边形，并可通过WebAssembly从JavaScript调用

// 输入(js端):
//     1. 点云 类型Float32Array 例子[x1, y1, x2, y2, ...]
//     2. 多边形路径点 类型Float32Array 例子[x1, y1, x2, y2, ...]
//     3. 多边形路径点的拆分 类型Uint32Array 例子[20, 30, 40] 表示0-20的点索引为外部多边形,20-30为内部的第一个洞,30-40为内部的第二个洞,40-结束为内部的第三个洞
//     4. 边界上点是否考虑为内部 boolean 默认为true
// 输出(js端):
//     1. 点云是否在多边形内部 类型Uint32Array 例子[1, 0, 1, 0, ...] 1表示在多边形内部,0表示在多边形外部

use wasm_bindgen::prelude::*; // 引入WebAssembly绑定，用于与JavaScript交互
use std::f64; // 引入浮点数相关功能，如EPSILON常量
use std::collections::HashMap;

pub mod test;  // 引入测试模块

// 优化常量
const EPSILON: f64 = 1e-10;        // 精度控制
const GRID_SIZE: usize = 64;      // 空间网格大小
const CACHE_SIZE: usize = 1024;   // 交点缓存大小

// 优化的数据结构
#[derive(Clone, Copy)]
struct Edge {
    x1: f64, y1: f64,
    x2: f64, y2: f64,
}

struct Ring {
    start_idx: usize,
    edge_count: usize,
    is_hole: bool,
    bounds: Bounds,
}

#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64, min_y: f64,
    max_x: f64, max_y: f64,
}

struct Polygon {
    edges: Vec<Edge>,
    rings: Vec<Ring>,
    bounds: Bounds,
}

#[derive(Clone)]
struct GridCell {
    edge_indices: Vec<usize>,
}

// 主函数：判断点是否在多边形内部
// 使用wasm_bindgen标注，使其可以从JavaScript调用
#[wasm_bindgen]
pub fn point_in_polygon(
    points: &[f32],           // 输入点集，格式为[x1, y1, x2, y2, ...]
    polygon: &[f32],          // 多边形顶点，格式为[x1, y1, x2, y2, ...]
    rings: &[u32],            // 多边形环的分割点，表示每个环的结束位置
    boundary_is_inside: bool, // 边界上的点是否视为在多边形内部
) -> Vec<u32> {               // 返回结果，1表示在内部，0表示在外部
    let point_count = points.len() / 2;
    if point_count == 0 || polygon.is_empty() || rings.is_empty() {
        return vec![0; point_count];
    }
    
    // 构建多边形数据结构和空间索引
    let poly = build_polygon(polygon, rings);
    let grid = build_grid(&poly);
    
    // 预分配结果
    let mut results = vec![0; point_count];
    
    // 创建射线交点缓存
    let mut ray_cache: HashMap<i64, HashMap<usize, Vec<f64>>> = HashMap::new();
    
    // 处理每个点
    for i in 0..point_count {
        let x = points[i * 2] as f64;
        let y = points[i * 2 + 1] as f64;
        
        // 1. 边界框快速检查
        if !point_in_bounds(x, y, &poly.bounds) {
            continue; // 点在多边形外部
        }
        
        // 2. 更简单直接的边界检查
        if is_point_exactly_on_edge(&poly, x, y) {
            results[i] = boundary_is_inside as u32;
            continue;
        }
        
        // 3. 使用优化的射线法判断点是否在多边形内部
        let y_key = quantize_y(y);
        let inside = optimized_ray_cast(&poly, x, y, &mut ray_cache, y_key);
        results[i] = inside as u32;
    }
    
    results
}

// 构建多边形数据结构
fn build_polygon(polygon: &[f32], rings: &[u32]) -> Polygon {
    let mut edges = Vec::new();
    let mut poly_rings = Vec::new();
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    
    let mut prev_idx = 0;
    
    // 处理每个环
    for (i, &split) in rings.iter().enumerate() {
        let mut ring_min_x = f64::MAX;
        let mut ring_min_y = f64::MAX;
        let mut ring_max_x = f64::MIN;
        let mut ring_max_y = f64::MIN;
        
        let start_edge_idx = edges.len();
        let start = prev_idx as usize * 2;
        let end = split as usize * 2;
        
        // 提取当前环的所有边
        let mut ring_edges = 0;
        for j in (start..end).step_by(2) {
            if j + 3 < end {
                let x1 = polygon[j] as f64;
                let y1 = polygon[j + 1] as f64;
                let x2 = polygon[j + 2] as f64;
                let y2 = polygon[j + 3] as f64;
                
                // 忽略退化边
                if (x1 - x2).abs() < EPSILON && (y1 - y2).abs() < EPSILON {
                    continue;
                }
                
                edges.push(Edge { x1, y1, x2, y2 });
                ring_edges += 1;
                
                // 更新环的边界框
                ring_min_x = ring_min_x.min(x1).min(x2);
                ring_min_y = ring_min_y.min(y1).min(y2);
                ring_max_x = ring_max_x.max(x1).max(x2);
                ring_max_y = ring_max_y.max(y1).max(y2);
            }
        }
        
        // 连接环的最后一点和第一点，封闭环
        if end > start + 2 {
            let x1 = polygon[end - 2] as f64;
            let y1 = polygon[end - 1] as f64;
            let x2 = polygon[start] as f64;
            let y2 = polygon[start + 1] as f64;
            
            if (x1 - x2).abs() >= EPSILON || (y1 - y2).abs() >= EPSILON {
                edges.push(Edge { x1, y1, x2, y2 });
                ring_edges += 1;
            }
        }
        
        // 创建环的边界框
        let ring_bounds = Bounds {
            min_x: ring_min_x, min_y: ring_min_y,
            max_x: ring_max_x, max_y: ring_max_y,
        };
        
        // 添加环到环列表
        poly_rings.push(Ring {
            start_idx: start_edge_idx,
            edge_count: ring_edges,
            is_hole: i > 0,  // 第一个环(i=0)是外环，其余(i>0)是内环(洞)
            bounds: ring_bounds,
        });
        
        // 更新整个多边形的边界框
        min_x = min_x.min(ring_min_x);
        min_y = min_y.min(ring_min_y);
        max_x = max_x.max(ring_max_x);
        max_y = max_y.max(ring_max_y);
        
        prev_idx = split;
    }
    
    // 创建多边形
    Polygon {
        edges,
        rings: poly_rings,
        bounds: Bounds { min_x, min_y, max_x, max_y },
    }
}

// 构建空间网格索引
fn build_grid(poly: &Polygon) -> Vec<Vec<GridCell>> {
    // 初始化网格
    let mut grid = vec![vec![GridCell { edge_indices: Vec::new() }; GRID_SIZE]; GRID_SIZE];
    
    let width = poly.bounds.max_x - poly.bounds.min_x;
    let height = poly.bounds.max_y - poly.bounds.min_y;
    
    // 如果多边形是一个点或非常小，返回空网格
    if width < EPSILON || height < EPSILON {
        return grid;
    }
    
    // 把每条边放入相应的网格单元
    for (edge_idx, edge) in poly.edges.iter().enumerate() {
        // 找出边覆盖的网格单元
        let cells = line_to_grid_cells(
            edge.x1, edge.y1, edge.x2, edge.y2,
            poly.bounds.min_x, poly.bounds.min_y, width, height
        );
        
        // 将边的索引添加到每个覆盖的网格单元中
        for (gx, gy) in cells {
            if gx < GRID_SIZE && gy < GRID_SIZE {
                grid[gx][gy].edge_indices.push(edge_idx);
            }
        }
    }
    
    grid
}

// 使用Bresenham算法将线段映射到网格单元
fn line_to_grid_cells(
    x1: f64, y1: f64, x2: f64, y2: f64,
    min_x: f64, min_y: f64, width: f64, height: f64
) -> Vec<(usize, usize)> {
    let mut cells = Vec::new();
    
    // 计算网格坐标
    let grid_x1 = ((x1 - min_x) / width * (GRID_SIZE as f64)).floor() as isize;
    let grid_y1 = ((y1 - min_y) / height * (GRID_SIZE as f64)).floor() as isize;
    let grid_x2 = ((x2 - min_x) / width * (GRID_SIZE as f64)).floor() as isize;
    let grid_y2 = ((y2 - min_y) / height * (GRID_SIZE as f64)).floor() as isize;
    
    // 使用Bresenham算法遍历线段覆盖的网格单元
    let dx = (grid_x2 - grid_x1).abs();
    let dy = -(grid_y2 - grid_y1).abs();
    let sx = if grid_x1 < grid_x2 { 1 } else { -1 };
    let sy = if grid_y1 < grid_y2 { 1 } else { -1 };
    
    let mut err = dx + dy;
    let mut x = grid_x1;
    let mut y = grid_y1;
    
    loop {
        if x >= 0 && y >= 0 && x < GRID_SIZE as isize && y < GRID_SIZE as isize {
            cells.push((x as usize, y as usize));
        }
        
        if x == grid_x2 && y == grid_y2 {
            break;
        }
        
        let e2 = 2 * err;
        if e2 >= dy {
            if x == grid_x2 {
                break;
            }
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            if y == grid_y2 {
                break;
            }
            err += dx;
            y += sy;
        }
    }
    
    cells
}

// 检查点是否在边界框内
#[inline]
fn point_in_bounds(x: f64, y: f64, bounds: &Bounds) -> bool {
    x >= bounds.min_x && x <= bounds.max_x && y >= bounds.min_y && y <= bounds.max_y
}

// 检查点是否在边上
fn is_point_on_edge(poly: &Polygon, grid: &Vec<Vec<GridCell>>, x: f64, y: f64) -> bool {
    // 确定点所在网格单元
    let width = poly.bounds.max_x - poly.bounds.min_x;
    let height = poly.bounds.max_y - poly.bounds.min_y;
    
    // 边界特殊处理：点在多边形外边界上
    // 正方形多边形的测试案例中，点(3.0, 1.5)在右边界上，需要特殊处理
    for (ring_idx, ring) in poly.rings.iter().enumerate() {
        if !ring.is_hole { // 只检查外环
            // 检查点是否在边界框边上
            if (x - ring.bounds.min_x).abs() < EPSILON || 
               (x - ring.bounds.max_x).abs() < EPSILON || 
               (y - ring.bounds.min_y).abs() < EPSILON || 
               (y - ring.bounds.max_y).abs() < EPSILON {
                
                // 只有当点在边界上时，才进行详细检查
                let start_idx = ring.start_idx;
                let end_idx = start_idx + ring.edge_count;
                
                for edge_idx in start_idx..end_idx {
                    let edge = &poly.edges[edge_idx];
                    
                    // 处理垂直线段 - 这是测试失败的关键区域
                    if (edge.x1 - edge.x2).abs() < EPSILON {
                        // 垂直线段，检查x坐标匹配且y在范围内
                        if (x - edge.x1).abs() < EPSILON && 
                           y >= (edge.y1.min(edge.y2) - EPSILON) && 
                           y <= (edge.y1.max(edge.y2) + EPSILON) {
                            return true;
                        }
                    }
                    // 处理水平线段
                    else if (edge.y1 - edge.y2).abs() < EPSILON {
                        // 水平线段，检查y坐标匹配且x在范围内
                        if (y - edge.y1).abs() < EPSILON && 
                           x >= (edge.x1.min(edge.x2) - EPSILON) && 
                           x <= (edge.x1.max(edge.x2) + EPSILON) {
                            return true;
                        }
                    }
                }
            }
        }
    }
    
    // 网格检查 - 原有代码保持不变
    let grid_x = ((x - poly.bounds.min_x) / width * (GRID_SIZE as f64)) as usize;
    let grid_y = ((y - poly.bounds.min_y) / height * (GRID_SIZE as f64)) as usize;
    
    if grid_x >= GRID_SIZE || grid_y >= GRID_SIZE {
        return false;
    }
    
    // 检查该网格单元中的所有边
    for &edge_idx in &grid[grid_x][grid_y].edge_indices {
        let edge = &poly.edges[edge_idx];
        
        // 边界框检查
        let min_x = edge.x1.min(edge.x2) - EPSILON;
        let max_x = edge.x1.max(edge.x2) + EPSILON;
        let min_y = edge.y1.min(edge.y2) - EPSILON;
        let max_y = edge.y1.max(edge.y2) + EPSILON;
        
        if x < min_x || x > max_x || y < min_y || y > max_y {
            continue;
        }
        
        // 计算点到线段的距离
        let dx = edge.x2 - edge.x1;
        let dy = edge.y2 - edge.y1;
        let len_sq = dx * dx + dy * dy;
        
        const EDGE_EPSILON: f64 = EPSILON * 0.1;  // 边缘检测使用更小的阈值
        
        if len_sq < EDGE_EPSILON * EDGE_EPSILON {
            if (x - edge.x1).abs() < EDGE_EPSILON && (y - edge.y1).abs() < EDGE_EPSILON {
                return true;
            }
            continue;
        }
        
        // 计算投影参数
        let t = ((x - edge.x1) * dx + (y - edge.y1) * dy) / len_sq;
        
        if t < 0.0 || t > 1.0 {
            continue; // 投影在线段外
        }
        
        // 计算投影点和距离
        let px = edge.x1 + t * dx;
        let py = edge.y1 + t * dy;
        let dist_sq = (x - px) * (x - px) + (y - py) * (y - py);
        
        if dist_sq <= EDGE_EPSILON * EDGE_EPSILON {
            return true;
        }
    }
    
    false
}

// 量化y坐标用于缓存
#[inline]
fn quantize_y(y: f64) -> i64 {
    (y * 1_000_000.0).round() as i64
}

// 优化的射线法实现
fn optimized_ray_cast(
    poly: &Polygon,
    x: f64,
    y: f64,
    cache: &mut HashMap<i64, HashMap<usize, Vec<f64>>>,
    y_key: i64
) -> bool {
    // 边界检查：如果点在任意边界上，应该在is_point_on_edge中已处理
    // 所以这里只处理内部点
    
    // 确保缓存不会无限增长
    if cache.len() > CACHE_SIZE {
        let keys: Vec<_> = cache.keys().cloned().collect();
        for key in keys.iter().take(cache.len() / 2) {
            cache.remove(key);
        }
    }
    
    // 使用标准的射线法判断
    let mut inside = false;
    
    for (ring_idx, ring) in poly.rings.iter().enumerate() {
        // 跳过不可能相交的环
        if y < ring.bounds.min_y - EPSILON || y > ring.bounds.max_y + EPSILON {
            continue;
        }
        
        // 查找或计算射线交点
        let intersections = if let Some(ring_cache) = cache.get(&y_key).and_then(|c| c.get(&ring_idx)) {
            ring_cache
        } else {
            let mut x_intersections = compute_ray_intersections(poly, ring_idx, y);
            x_intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
            
            cache.entry(y_key)
                 .or_insert_with(HashMap::new)
                 .insert(ring_idx, x_intersections.clone());
            
            &cache.get(&y_key).unwrap().get(&ring_idx).unwrap()
        };
        
        // 计算穿过点右侧边界的次数
        let mut crossings = 0;
        for &xi in intersections {
            // 使用大于等于处理交点，这样能正确处理点在边上的情况
            if xi >= x - EPSILON {
                crossings += 1;
            }
        }
        
        // 应用奇偶规则
        if crossings % 2 == 1 {
            if !ring.is_hole {
                inside = !inside;
            } else if inside {
                inside = false;
                break;
            }
        }
    }
    
    inside
}

// 修复交点计算函数，确保精确处理所有情况
fn compute_ray_intersections(poly: &Polygon, ring_idx: usize, y: f64) -> Vec<f64> {
    let ring = &poly.rings[ring_idx];
    let mut intersections = Vec::new();
    
    let start_idx = ring.start_idx;
    let end_idx = start_idx + ring.edge_count;
    
    for edge_idx in start_idx..end_idx {
        let edge = &poly.edges[edge_idx];
        
        // 更精确的边界检查
        let min_y = edge.y1.min(edge.y2) - EPSILON;
        let max_y = edge.y1.max(edge.y2) + EPSILON;
        
        // 跳过不与射线水平线相交的边
        if y < min_y || y > max_y {
            continue;
        }
        
        // 跳过水平边（特殊情况单独处理）
        if (edge.y1 - edge.y2).abs() < EPSILON {
            continue;
        }
        
        // 计算交点
        if (edge.y1 - y).abs() < EPSILON {
            // 起点在射线上
            if edge.y2 < y {  // 从上到下穿过射线
                intersections.push(edge.x1);
            }
            // 注意：从下到上穿过不算交点，避免重复计算
        } else if (edge.y2 - y).abs() < EPSILON {
            // 终点在射线上
            if edge.y1 < y {  // 从上到下穿过射线
                intersections.push(edge.x2);
            }
        } else if (edge.y1 < y && edge.y2 > y) || (edge.y1 > y && edge.y2 < y) {
            // 边与射线相交
            let t = (y - edge.y1) / (edge.y2 - edge.y1);
            let x = edge.x1 + t * (edge.x2 - edge.x1);
            intersections.push(x);
        }
    }
    
    intersections
}

// 添加检查点是否严格在边界上的函数
fn is_point_exactly_on_edge(poly: &Polygon, x: f64, y: f64) -> bool {
    // 检查每个边
    for edge in &poly.edges {
        // 检查垂直边界
        if (edge.x1 - edge.x2).abs() < EPSILON {
            // 点在垂直线上
            if (x - edge.x1).abs() < EPSILON && 
               y >= edge.y1.min(edge.y2) - EPSILON && 
               y <= edge.y1.max(edge.y2) + EPSILON {
                return true;
            }
        } 
        // 检查水平边界
        else if (edge.y1 - edge.y2).abs() < EPSILON {
            // 点在水平线上
            if (y - edge.y1).abs() < EPSILON && 
               x >= edge.x1.min(edge.x2) - EPSILON && 
               x <= edge.x1.max(edge.x2) + EPSILON {
                return true;
            }
        }
        // 一般斜线
        else {
            // 计算点到线段的精确距离
            let dx = edge.x2 - edge.x1;
            let dy = edge.y2 - edge.y1;
            let len_sq = dx * dx + dy * dy;
            
            // 计算投影参数
            let t = ((x - edge.x1) * dx + (y - edge.y1) * dy) / len_sq;
            
            if t >= 0.0 && t <= 1.0 {
                // 计算投影点和距离
                let px = edge.x1 + t * dx;
                let py = edge.y1 + t * dy;
                let dist_sq = (x - px) * (x - px) + (y - py) * (y - py);
                
                if dist_sq < EPSILON * EPSILON {
                    return true;
                }
            }
        }
    }
    
    false
}