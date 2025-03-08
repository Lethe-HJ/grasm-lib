// 扫描线算法模块：实现了使用扫描线算法判断点是否在多边形内部
// 该实现包含以下性能优化：
// 1. 空间网格索引加速边的查找
// 2. 扫描线交点计算缓存
// 3. 边界框快速过滤
// 4. 高精度边界点检测
// 该算法对于大量点和复杂多边形有更好的性能表现

use wasm_bindgen::prelude::*;
use std::f64;
use std::collections::HashMap;
// 移除未使用的导入
// use std::cmp::Ordering;

pub mod test;

// 精度和性能相关常量
const EPSILON: f64 = 1e-9;     // 浮点数比较的精度阈值，用于处理数值精度问题
const GRID_SIZE: usize = 64;   // 空间网格的大小，影响网格索引的精度和内存使用
const CACHE_SIZE: usize = 1024; // 扫描线交点缓存的最大数量

// 多边形数据结构：存储整个多边形的边和环信息
struct Polygon {
    edges: Vec<Edge>,    // 存储所有边的集合
    rings: Vec<Ring>,    // 存储所有环的集合（外环和内部的洞）
    bounds: Bounds,      // 整个多边形的边界框
}

// 环结构：表示多边形的一个环（外环或内部的洞）
struct Ring {
    start_idx: usize,    // 该环的第一条边在edges数组中的索引
    edge_count: usize,   // 该环包含的边数量
    is_hole: bool,       // 标识该环是否为洞（内环）
    bounds: Bounds,      // 该环的边界框
}

// 边结构：表示多边形的一条边（一个线段）
#[derive(Clone, Copy)]
struct Edge {
    x1: f64, y1: f64,    // 边的起点坐标
    x2: f64, y2: f64,    // 边的终点坐标
}

// 边界框：用于快速空间过滤
#[derive(Clone, Copy)]
struct Bounds {
    min_x: f64, min_y: f64,    // 边界框的最小坐标（左下角）
    max_x: f64, max_y: f64,    // 边界框的最大坐标（右上角）
}

// 空间网格单元：存储落在该网格内的边的索引
#[derive(Clone)]
struct GridCell {
    edge_indices: Vec<usize>,  // 该网格单元包含的边的索引列表
}

// WebAssembly导出函数：批量判断点是否在多边形内部
#[wasm_bindgen]
pub fn point_in_polygon_scanline(
    points: &[f32],           // 输入点集，平铺存储 [x1,y1,x2,y2...]
    polygon: &[f32],          // 多边形顶点，平铺存储 [x1,y1,x2,y2...]
    rings: &[u32],            // 多边形环的分割索引
    boundary_is_inside: bool, // 边界点是否视为内部
) -> Vec<u32> {
    // 处理空输入的边界情况
    let point_count = points.len() / 2;
    if point_count == 0 || polygon.is_empty() || rings.is_empty() {
        return vec![0; point_count];
    }
    
    // 构建多边形数据结构和空间索引
    let poly = build_polygon(polygon, rings);
    let grid = build_grid(&poly);
    
    // 预分配结果数组
    let mut results = vec![0; point_count];
    
    // 创建扫描线交点缓存，用于重用计算结果
    // 键是量化后的y坐标，值是该y坐标下与多边形的交点列表
    let mut scanline_cache: HashMap<i64, Vec<(f64, usize, usize)>> = HashMap::new();
    
    // 处理每个点
    for i in 0..point_count {
        let x = points[i * 2] as f64;     // 当前点的x坐标
        let y = points[i * 2 + 1] as f64; // 当前点的y坐标
        
        // 1. 边界框快速检查 - 如果点在整个多边形的边界框外，肯定在多边形外
        if !point_in_bounds(x, y, &poly.bounds) {
            continue; // 点在多边形外部
        }
        
        // 2. 检查点是否在边上 - 边界情况处理
        if is_point_on_edge(&poly, &grid, x, y) {
            results[i] = boundary_is_inside as u32;
            continue;
        }
        
        // 3. 使用扫描线算法判断点是否在多边形内部
        let y_key = quantize_y(y);  // 量化y坐标以便缓存查找
        let inside = is_point_in_polygon(&poly, &grid, x, y, &mut scanline_cache, y_key);
        results[i] = inside as u32;
    }
    
    results
}

// 构建多边形数据结构：从输入的平铺数组构建结构化的多边形表示
fn build_polygon(polygon: &[f32], rings: &[u32]) -> Polygon {
    let mut edges = Vec::new();        // 存储所有边
    let mut poly_rings = Vec::new();   // 存储所有环
    let mut min_x = f64::MAX;          // 整个多边形的最小x坐标
    let mut min_y = f64::MAX;          // 整个多边形的最小y坐标
    let mut max_x = f64::MIN;          // 整个多边形的最大x坐标
    let mut max_y = f64::MIN;          // 整个多边形的最大y坐标
    
    let _start_idx = 0;                // 未使用的变量
    let mut prev_idx = 0;              // 前一个环的结束索引
    
    // 处理每个环（外环和洞）
    for (i, &split) in rings.iter().enumerate() {
        let mut ring_min_x = f64::MAX;  // 当前环的最小x坐标
        let mut ring_min_y = f64::MAX;  // 当前环的最小y坐标
        let mut ring_max_x = f64::MIN;  // 当前环的最大x坐标
        let mut ring_max_y = f64::MIN;  // 当前环的最大y坐标
        
        let start_edge_idx = edges.len();  // 当前环的第一条边索引
        let start = prev_idx as usize * 2; // 当前环在polygon数组中的起始位置
        let end = split as usize * 2;      // 当前环在polygon数组中的结束位置
        
        // 提取当前环的所有边
        let mut ring_edges = 0;
        for j in (start..end).step_by(2) {
            if j + 3 < end {
                let x1 = polygon[j] as f64;         // 边的起点x坐标
                let y1 = polygon[j + 1] as f64;     // 边的起点y坐标
                let x2 = polygon[j + 2] as f64;     // 边的终点x坐标
                let y2 = polygon[j + 3] as f64;     // 边的终点y坐标
                
                // 忽略退化边（长度接近0的边）
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
            let x1 = polygon[end - 2] as f64;     // 最后一点的x坐标
            let y1 = polygon[end - 1] as f64;     // 最后一点的y坐标
            let x2 = polygon[start] as f64;       // 第一点的x坐标
            let y2 = polygon[start + 1] as f64;   // 第一点的y坐标
            
            // 检查是否是有效边（非退化边）
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
            is_hole: i > 0, // 第一个环是外环，其余是内环（洞）
            bounds: ring_bounds,
        });
        
        // 更新整个多边形的边界框
        min_x = min_x.min(ring_min_x);
        min_y = min_y.min(ring_min_y);
        max_x = max_x.max(ring_max_x);
        max_y = max_y.max(ring_max_y);
        
        prev_idx = split;
    }
    
    // 处理最后一个环（如果有）
    let start = prev_idx as usize * 2;
    let end = polygon.len();
    
    if end > start + 2 {
        let mut ring_min_x = f64::MAX;
        let mut ring_min_y = f64::MAX;
        let mut ring_max_x = f64::MIN;
        let mut ring_max_y = f64::MIN;
        
        let start_edge_idx = edges.len();
        let mut ring_edges = 0;
        
        // 提取最后一个环的所有边
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
                
                // 更新边界框
                ring_min_x = ring_min_x.min(x1).min(x2);
                ring_min_y = ring_min_y.min(y1).min(y2);
                ring_max_x = ring_max_x.max(x1).max(x2);
                ring_max_y = ring_max_y.max(y1).max(y2);
            }
        }
        
        // 连接最后一个环的最后一点和第一点
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
        
        // 创建最后一个环的边界框
        let ring_bounds = Bounds {
            min_x: ring_min_x, min_y: ring_min_y,
            max_x: ring_max_x, max_y: ring_max_y,
        };
        
        // 添加最后一个环
        poly_rings.push(Ring {
            start_idx: start_edge_idx,
            edge_count: ring_edges,
            is_hole: rings.len() > 0, // 如果之前有环，则这个是洞
            bounds: ring_bounds,
        });
        
        // 更新整个多边形的边界框
        min_x = min_x.min(ring_min_x);
        min_y = min_y.min(ring_min_y);
        max_x = max_x.max(ring_max_x);
        max_y = max_y.max(ring_max_y);
    }
    
    // 创建整个多边形的边界框
    let poly_bounds = Bounds {
        min_x, min_y, max_x, max_y,
    };
    
    // 返回构建好的多边形
    Polygon {
        edges,
        rings: poly_rings,
        bounds: poly_bounds,
    }
}

// 构建空间网格索引：将多边形的边分配到网格单元中，用于加速空间查询
fn build_grid(poly: &Polygon) -> Vec<Vec<GridCell>> {
    // 创建网格
    let mut grid = vec![vec![GridCell { edge_indices: Vec::new() }; GRID_SIZE]; GRID_SIZE];
    
    // 计算网格单元尺寸
    let width = poly.bounds.max_x - poly.bounds.min_x;
    let height = poly.bounds.max_y - poly.bounds.min_y;
    
    // 将所有边添加到相应的网格单元中
    for (edge_idx, edge) in poly.edges.iter().enumerate() {
        // 确定边横跨的网格单元
        let cells = get_grid_cells(
            poly.bounds.min_x, poly.bounds.min_y,
            width, height,
            edge.x1, edge.y1, edge.x2, edge.y2
        );
        
        // 将边的索引添加到相应的网格单元中
        for (gx, gy) in cells {
            grid[gx][gy].edge_indices.push(edge_idx);
        }
    }
    
    grid
}

// 计算线段横跨的网格单元：使用改进的Bresenham算法跟踪线段穿过的所有网格单元
fn get_grid_cells(
    min_x: f64, min_y: f64,
    width: f64, height: f64,
    x1: f64, y1: f64, x2: f64, y2: f64
) -> Vec<(usize, usize)> {
    // 结果列表：存储线段穿过的所有网格单元坐标
    let mut cells = Vec::new();
    
    // 将线段端点坐标转换为网格索引
    let x1_grid = ((x1 - min_x) / width * (GRID_SIZE as f64)) as usize;
    let y1_grid = ((y1 - min_y) / height * (GRID_SIZE as f64)) as usize;
    let x2_grid = ((x2 - min_x) / width * (GRID_SIZE as f64)) as usize;
    let y2_grid = ((y2 - min_y) / height * (GRID_SIZE as f64)) as usize;
    
    // 确保网格索引不超出范围
    let x1_grid = x1_grid.min(GRID_SIZE - 1);
    let y1_grid = y1_grid.min(GRID_SIZE - 1);
    let x2_grid = x2_grid.min(GRID_SIZE - 1);
    let y2_grid = y2_grid.min(GRID_SIZE - 1);
    
    // 如果线段在单个网格单元内，直接返回
    if x1_grid == x2_grid && y1_grid == y2_grid {
        cells.push((x1_grid, y1_grid));
        return cells;
    }
    
    // 简化的Bresenham算法：追踪线段穿过的所有网格单元
    let dx = (x2_grid as isize - x1_grid as isize).abs();
    let dy = (y2_grid as isize - y1_grid as isize).abs();
    let sx = if x1_grid < x2_grid { 1 } else { -1 };
    let sy = if y1_grid < y2_grid { 1 } else { -1 };
    let mut err = if dx > dy { dx } else { -dy } as isize / 2;
    
    let mut x = x1_grid as isize;
    let mut y = y1_grid as isize;
    
    // 追踪线段路径
    loop {
        // 如果网格单元在有效范围内，添加到结果列表
        if x >= 0 && y >= 0 && x < GRID_SIZE as isize && y < GRID_SIZE as isize {
            cells.push((x as usize, y as usize));
        }
        
        // 如果到达终点，结束循环
        if x == x2_grid as isize && y == y2_grid as isize {
            break;
        }
        
        // 计算下一个网格单元
        let e2 = err;
        if e2 > -dx {
            err -= dy as isize;
            x += sx;
        }
        if e2 < dy {
            err += dx as isize;
            y += sy;
        }
    }
    
    cells
}

// 检查点是否在边界框内：快速过滤点
#[inline]
fn point_in_bounds(x: f64, y: f64, bounds: &Bounds) -> bool {
    x >= bounds.min_x && x <= bounds.max_x && y >= bounds.min_y && y <= bounds.max_y
}

// 检查点是否在任何边上：用于处理边界点
fn is_point_on_edge(poly: &Polygon, grid: &Vec<Vec<GridCell>>, x: f64, y: f64) -> bool {
    // 确定点所在网格单元
    let width = poly.bounds.max_x - poly.bounds.min_x;
    let height = poly.bounds.max_y - poly.bounds.min_y;
    
    // 计算点所在的网格单元索引
    let grid_x = ((x - poly.bounds.min_x) / width * (GRID_SIZE as f64)) as usize;
    let grid_y = ((y - poly.bounds.min_y) / height * (GRID_SIZE as f64)) as usize;
    
    // 检查点是否在网格范围内
    if grid_x >= GRID_SIZE || grid_y >= GRID_SIZE {
        return false;
    }
    
    // 检查该网格单元中的所有边
    for &edge_idx in &grid[grid_x][grid_y].edge_indices {
        let edge = &poly.edges[edge_idx];
        
        // 快速边界框检查：如果点不在边的边界框内，跳过
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
        
        // 处理退化为点的边
        if len_sq < EPSILON * EPSILON {
            if (x - edge.x1).abs() < EPSILON && (y - edge.y1).abs() < EPSILON {
                return true;
            }
            continue;
        }
        
        // 计算点到线段的投影参数t
        // 当t在[0,1]范围内时，投影点在线段上
        let t = ((x - edge.x1) * dx + (y - edge.y1) * dy) / len_sq;
        
        if t < 0.0 || t > 1.0 {
            continue; // 投影点不在线段上
        }
        
        // 计算投影点坐标和到原点的距离
        let px = edge.x1 + t * dx;
        let py = edge.y1 + t * dy;
        let dist_sq = (x - px) * (x - px) + (y - py) * (y - py);
        
        // 如果距离小于阈值，认为点在边上
        if dist_sq <= EPSILON * EPSILON {
            return true;
        }
    }
    
    // 未找到点在上面的边
    false
}

// 量化y坐标以便缓存：将浮点y值转换为整数键
#[inline]
fn quantize_y(y: f64) -> i64 {
    // 将y坐标放大并四舍五入，用于HashMap键
    (y * 1_000_000.0).round() as i64
}

// 判断点是否在多边形内部：使用扫描线算法
fn is_point_in_polygon(
    poly: &Polygon,
    _grid: &Vec<Vec<GridCell>>,
    x: f64,
    y: f64,
    cache: &mut HashMap<i64, Vec<(f64, usize, usize)>>,
    y_key: i64
) -> bool {
    // 获取或计算扫描线交点
    let intersections = if let Some(cached) = cache.get(&y_key) {
        cached
    } else {
        // 缓存未命中，计算新的交点
        let mut inters = compute_intersections(poly, y);
        // 按x坐标排序交点
        inters.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // 维护缓存大小，防止内存泄漏
        if cache.len() >= CACHE_SIZE {
            // 缓存满时，清除一半缓存
            let keys: Vec<_> = cache.keys().cloned().collect();
            for key in keys.iter().take(cache.len() / 2) {
                cache.remove(key);
            }
        }
        
        // 将新计算的交点添加到缓存
        cache.insert(y_key, inters);
        cache.get(&y_key).unwrap()
    };
    
    // 分别处理外环和内环
    let mut in_holes = false;
    
    // 首先判断点是否在外环内 (奇数个交点表示在内部)
    let mut crossings_outer = 0;
    for &(xi, _edge_idx, ring_idx) in intersections.iter() {
        if xi >= x {
            continue; // 只考虑点左侧的交点
        }
        
        if !poly.rings[ring_idx].is_hole {
            crossings_outer += 1;
        }
    }
    let in_outer = crossings_outer % 2 == 1;
    
    // 如果不在外环内，肯定不在多边形内
    if !in_outer {
        return false;
    }
    
    // 然后判断点是否在任何洞内 (对每个洞单独判断)
    for ring_idx in 0..poly.rings.len() {
        if !poly.rings[ring_idx].is_hole {
            continue; // 跳过外环
        }
        
        // 跳过不包含该点的洞
        if !point_in_bounds(x, y, &poly.rings[ring_idx].bounds) {
            continue;
        }
        
        // 计算与该洞的交点数
        let mut hole_crossings = 0;
        for &(xi, _edge_idx, r_idx) in intersections.iter() {
            if xi >= x || r_idx != ring_idx {
                continue;
            }
            hole_crossings += 1;
        }
        
        // 如果在任何一个洞内，则不在多边形内
        if hole_crossings % 2 == 1 {
            in_holes = true;
            break;
        }
    }
    
    // 在外环内且不在任何洞内
    in_outer && !in_holes
}

// 计算扫描线与多边形的交点：找出y值与多边形边的所有交点
fn compute_intersections(poly: &Polygon, y: f64) -> Vec<(f64, usize, usize)> {
    // 结果列表：(x坐标, 边索引, 环索引)
    let mut intersections = Vec::new();
    
    // 遍历所有环
    for (ring_idx, ring) in poly.rings.iter().enumerate() {
        // 跳过不与扫描线相交的环
        if y < ring.bounds.min_y || y > ring.bounds.max_y {
            continue;
        }
        
        // 遍历环中的所有边
        let end_idx = ring.start_idx + ring.edge_count;
        for edge_idx in ring.start_idx..end_idx {
            let edge = &poly.edges[edge_idx];
            
            // 检查边是否与扫描线相交
            // 优化处理接近扫描线的情况
            if edge.y1 < y - EPSILON && edge.y2 < y - EPSILON {
                continue; // 边完全在扫描线下方
            }
            
            if edge.y1 > y + EPSILON && edge.y2 > y + EPSILON {
                continue; // 边完全在扫描线上方
            }
            
            // 改进处理扫描线经过顶点的情况
            if (edge.y1 - y).abs() < EPSILON {
                // 扫描线经过边的起点
                
                // 找到该顶点的前一条边
                let prev_edge_idx = if edge_idx > ring.start_idx {
                    edge_idx - 1
                } else {
                    ring.start_idx + ring.edge_count - 1
                };
                
                let prev_edge = &poly.edges[prev_edge_idx];
                
                // 如果两条相邻边的一个在上方一个在下方，则计算交点
                if (prev_edge.y1 > y && edge.y2 < y) || 
                   (prev_edge.y1 < y && edge.y2 > y) {
                    intersections.push((edge.x1, edge_idx, ring_idx));
                }
            } else if (edge.y2 - y).abs() < EPSILON {
                // 扫描线经过边的终点，不重复计算，因为它会被下一条边处理
                continue;
            } else if (edge.y1 - edge.y2).abs() < EPSILON {
                // 忽略水平边，它们不会产生有效交点
                continue;
            } else {
                // 标准情况：线段与扫描线相交于非顶点处
                let t = (y - edge.y1) / (edge.y2 - edge.y1);
                let x = edge.x1 + t * (edge.x2 - edge.x1);
                intersections.push((x, edge_idx, ring_idx));
            }
        }
    }
    
    intersections
} 