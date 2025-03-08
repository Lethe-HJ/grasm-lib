// 导入 points_in_polygon 模块
pub mod points_in_polygon;

// 重新导出 points_in_polygon 模块中的函数，使其可以从 JavaScript 调用
// pub use points_in_polygon::rayster::point_in_polygon_rayster;
pub use points_in_polygon::scanline::point_in_polygon_scanline;