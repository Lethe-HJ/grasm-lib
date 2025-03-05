// 帮我写一个rust代码 内容是判断点是否在多边形内部, 并且我希望通过js调用rust生成的wasm
// 输入(js端):
//     1. 点云 类型Float32Array 例子[x1, y1, x2, y2, ...]
//     2. 多边形路径点 类型Float32Array 例子[x1, y1, x2, y2, ...]
//     3. 多边形路径点的拆分 类型Uint32Array 例子[20, 30, 40] 表示0-20的点索引为外部多边形,20-30为内部的第一个洞,30-40为内部的第二个洞,40-结束为内部的第三个洞
//     4. 边界上点是否考虑为内部 boolean 默认为true
// 输出(js端):
//     1. 点云是否在多边形内部 类型Uint32Array 例子[1, 0, 1, 0, ...] 1表示在多边形内部,0表示在多边形外部


use wasm_bindgen::prelude::*;
use std::f64;

pub mod test;  // 添加这行来引入测试模块

#[wasm_bindgen]
pub fn point_in_polygon(
    points: &[f32],
    polygon: &[f32],
    rings: &[u32],
    boundary_is_inside: bool,
) -> Vec<u32> {
    let rings = parse_rings(polygon, rings);
    points
        .chunks_exact(2)
        .map(|p| is_point_inside(p[0] as f64, p[1] as f64, &rings, boundary_is_inside) as u32)
        .collect()
}

struct Ring {
    points: Vec<(f64, f64)>,
    is_hole: bool,
}

fn parse_rings(polygon: &[f32], splits: &[u32]) -> Vec<Ring> {
    let mut rings = Vec::new();
    let mut prev = 0;
    for &split in splits {
        let slice = &polygon[prev as usize * 2..split as usize * 2];
        rings.push(create_ring(slice, false));
        prev = split;
    }
    let last = &polygon[prev as usize * 2..];
    if !last.is_empty() {
        rings.push(create_ring(last, true));
    }
    rings
}

fn create_ring(data: &[f32], is_hole: bool) -> Ring {
    let mut points: Vec<_> = data
        .chunks_exact(2)
        .map(|c| (c[0] as f64, c[1] as f64))
        .collect();
    if points.first() != points.last() {
        points.push(points[0]);
    }
    Ring { points, is_hole }
}

fn is_point_inside(x: f64, y: f64, rings: &[Ring], boundary_is_inside: bool) -> bool {
    let mut inside = false;
    for ring in rings {
        let is_in_ring = ray_cast(x, y, &ring.points, boundary_is_inside);
        if ring.is_hole {
            inside &= !is_in_ring;
        } else {
            inside |= is_in_ring;
        }
    }
    inside
}

fn ray_cast(x: f64, y: f64, polygon: &[(f64, f64)], boundary_is_inside: bool) -> bool {
    let mut inside = false;
    for i in 0..polygon.len() - 1 {
        let (x1, y1) = polygon[i];
        let (x2, y2) = polygon[i + 1];
        
        if on_segment(x, y, x1, y1, x2, y2) {
            return boundary_is_inside;
        }
        
        if ((y1 > y) != (y2 > y)) && (x < (x2 - x1) * (y - y1) / (y2 - y1) + x1) {
            inside = !inside;
        }
    }
    inside
}

fn on_segment(x: f64, y: f64, x1: f64, y1: f64, x2: f64, y2: f64) -> bool {
    if (x - x1).abs() < f64::EPSILON && (y - y1).abs() < f64::EPSILON ||
       (x - x2).abs() < f64::EPSILON && (y - y2).abs() < f64::EPSILON {
        return true;
    }
    
    let cross = (x2 - x1) * (y - y1) - (y2 - y1) * (x - x1);
    if cross.abs() > f64::EPSILON {
        return false;
    }
    
    let dot = (x - x1) * (x2 - x1) + (y - y1) * (y2 - y1);
    if dot < 0.0 {
        return false;
    }
    
    let squared_length = (x2 - x1).powi(2) + (y2 - y1).powi(2);
    dot <= squared_length
}
