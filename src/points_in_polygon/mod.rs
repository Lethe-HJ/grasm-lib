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

pub mod test;  // 引入测试模块

// 预计算的多边形数据结构，用于存储优化后的多边形信息
struct PrecomputedPolygon {
    rings: Vec<PrecomputedRing>, // 存储多边形的所有环（外环和内环/洞）
}

// 预计算的环数据结构，表示多边形的一个环（可能是外环或内环/洞）
struct PrecomputedRing {
    edges: Vec<Edge>,       // 环的所有边
    bounds: Bounds,         // 环的边界框，用于快速判断点是否可能在环内
    is_hole: bool,          // 是否是洞（内环）
}

// 边数据结构，存储边的信息和预计算值
struct Edge {
    x1: f64, y1: f64,       // 边的起点坐标
    x2: f64, y2: f64,       // 边的终点坐标
    dx: f64, dy: f64,       // 边的方向向量 (x2-x1, y2-y1)
    squared_length: f64,    // 边长度的平方，预计算以提高性能
}

// 边界框数据结构，用于快速剔除明显不在多边形内的点
struct Bounds {
    min_x: f64, min_y: f64, // 边界框的左下角坐标
    max_x: f64, max_y: f64, // 边界框的右上角坐标
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
    // 预计算多边形数据，提高后续判断的效率
    let precomputed = precompute_polygon(polygon, rings);
    
    // 对每个点进行判断，并收集结果
    // chunks_exact(2)将点集按照(x,y)对进行分组
    points
        .chunks_exact(2)
        .map(|p| {
            let x = p[0] as f64; // 将f32转换为f64以提高精度
            let y = p[1] as f64;
            // 判断点是否在多边形内部，并将bool转换为u32
            is_point_inside_optimized(x, y, &precomputed, boundary_is_inside) as u32
        })
        .collect() // 收集所有结果到Vec<u32>
}

// 预计算多边形数据，将原始数据转换为优化的数据结构
fn precompute_polygon(polygon: &[f32], splits: &[u32]) -> PrecomputedPolygon {
    let mut rings = Vec::new();
    let mut prev = 0;
    
    // 处理外环和内环
    // 遍历所有分割点，每个分割点表示一个环的结束
    for &split in splits.iter() {
        // 提取当前环的顶点数据
        let slice = &polygon[prev as usize * 2..split as usize * 2];
        // 创建预计算环，is_hole=false表示这是外环
        rings.push(create_precomputed_ring(slice, false));
        prev = split; // 更新起始位置为当前分割点
    }
    
    // 处理最后一个环（如果有）
    // 这通常是最后一个洞，从最后一个分割点到数组结束
    let last = &polygon[prev as usize * 2..];
    if !last.is_empty() {
        // 创建预计算环，is_hole=true表示这是内环（洞）
        rings.push(create_precomputed_ring(last, true));
    }
    
    // 返回预计算的多边形数据
    PrecomputedPolygon { rings }
}

// 创建预计算的环数据结构
fn create_precomputed_ring(data: &[f32], is_hole: bool) -> PrecomputedRing {
    // 将f32坐标对转换为f64坐标对
    let mut points: Vec<(f64, f64)> = data
        .chunks_exact(2)
        .map(|c| (c[0] as f64, c[1] as f64))
        .collect();
    
    // 确保多边形闭合（首尾相连）
    // 如果首尾点不同且点集不为空，则添加首点作为尾点
    if points.first() != points.last() && !points.is_empty() {
        points.push(points[0]);
    }
    
    // 计算环的边界框
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    
    // 遍历所有点，找出最小和最大的x、y坐标
    for &(x, y) in &points {
        min_x = min_x.min(x); // 取当前min_x和x中的较小值
        min_y = min_y.min(y);
        max_x = max_x.max(x); // 取当前max_x和x中的较大值
        max_y = max_y.max(y);
    }
    
    // 预计算环的所有边
    let mut edges = Vec::with_capacity(points.len() - 1);
    for i in 0..points.len() - 1 {
        let (x1, y1) = points[i];     // 边的起点
        let (x2, y2) = points[i + 1]; // 边的终点
        
        let dx = x2 - x1; // 边的x方向分量
        let dy = y2 - y1; // 边的y方向分量
        let squared_length = dx * dx + dy * dy; // 边长度的平方
        
        // 创建边对象并存储预计算的值
        edges.push(Edge {
            x1, y1, x2, y2,
            dx, dy,
            squared_length,
        });
    }
    
    // 返回预计算的环数据
    PrecomputedRing {
        edges,
        bounds: Bounds { min_x, min_y, max_x, max_y },
        is_hole,
    }
}

// 判断点是否在多边形内部的优化版本
// #[inline]提示编译器考虑内联此函数以提高性能
#[inline]
fn is_point_inside_optimized(x: f64, y: f64, polygon: &PrecomputedPolygon, boundary_is_inside: bool) -> bool {
    let mut inside = false; // 初始假设点在多边形外部
    
    // 遍历多边形的所有环
    for ring in &polygon.rings {
        // 快速边界框检查 - 如果点在环的边界框外，可以快速判断
        if x < ring.bounds.min_x || x > ring.bounds.max_x || 
           y < ring.bounds.min_y || y > ring.bounds.max_y {
            // 点在边界框外
            if !ring.is_hole {
                continue; // 如果是外环，跳过此环（点肯定不在此环内）
            }
            // 如果是内环（洞），点不在洞内，不改变inside状态
        } else {
            // 点在边界框内，需要精确检查
            // 使用射线法判断点是否在环内
            let is_in_ring = ray_cast_optimized(x, y, ring, boundary_is_inside);
            
            if ring.is_hole {
                // 如果是内环（洞）
                if is_in_ring {
                    // 如果点在洞内，则不在多边形内
                    inside = false;
                    break; // 提前返回，不需要检查其他环
                }
            } else {
                // 如果是外环
                // 使用逻辑或操作，如果点在任一外环内，则在多边形内
                inside |= is_in_ring;
            }
        }
    }
    
    inside // 返回最终结果
}

// 射线法判断点是否在环内的优化版本
#[inline]
fn ray_cast_optimized(x: f64, y: f64, ring: &PrecomputedRing, boundary_is_inside: bool) -> bool {
    let mut inside = false; // 初始假设点在环外
    
    // 遍历环的所有边
    for edge in &ring.edges {
        // 检查点是否在边上
        if on_segment_optimized(x, y, edge) {
            return boundary_is_inside; // 如果点在边上，根据参数决定返回值
        }
        
        // 射线法检查 - 从点向右发射一条水平射线，计算与多边形边的交点数
        // 如果边的一个端点在射线上方，另一个在下方（或正好在射线上）
        if (edge.y1 > y) != (edge.y2 > y) {
            // 避免除以零（水平边的情况）
            if edge.dy != 0.0 {
                // 计算射线与边的交点的x坐标
                let intersect_x = edge.x1 + (edge.dx * (y - edge.y1) / edge.dy);
                // 如果交点在点的右侧，则射线与边相交
                if x < intersect_x {
                    // 每次相交，切换inside状态
                    inside = !inside;
                }
            }
        }
    }
    
    inside // 返回最终结果
}

// 判断点是否在线段上的优化版本
#[inline]
fn on_segment_optimized(x: f64, y: f64, edge: &Edge) -> bool {
    // 快速端点检查 - 如果点是线段的端点，直接返回true
    if (x - edge.x1).abs() < f64::EPSILON && (y - edge.y1).abs() < f64::EPSILON ||
       (x - edge.x2).abs() < f64::EPSILON && (y - edge.y2).abs() < f64::EPSILON {
        return true;
    }
    
    // 叉积检查 - 判断点是否在线段所在的直线上
    // 如果点在线段所在直线上，则向量(x-x1,y-y1)和(x2-x1,y2-y1)共线，叉积为0
    let cross = edge.dx * (y - edge.y1) - edge.dy * (x - edge.x1);
    if cross.abs() > f64::EPSILON {
        return false; // 如果叉积不为0，点不在线段所在直线上
    }
    
    // 点积检查 - 判断点是否在线段范围内
    // 计算向量(x-x1,y-y1)和(x2-x1,y2-y1)的点积
    let dot = (x - edge.x1) * edge.dx + (y - edge.y1) * edge.dy;
    if dot < 0.0 {
        return false; // 如果点积小于0，点在线段起点的反方向
    }
    
    // 如果点积小于等于线段长度的平方，点在线段上或线段的延长线上
    dot <= edge.squared_length
}