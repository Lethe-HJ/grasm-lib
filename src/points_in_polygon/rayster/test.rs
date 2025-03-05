#[cfg(test)]
mod tests {
    use crate::points_in_polygon::rayster::point_in_polygon_rayster;
    use std::time::Instant;

    #[test]
    fn test_square_with_hole() {
        let polygon = vec![
            0.0, 0.0, 3.0, 0.0, 3.0, 3.0, 0.0, 3.0, // Outer ring
            1.0, 1.0, 2.0, 1.0, 2.0, 2.0, 1.0, 2.0, // Hole
        ];
        let rings = vec![4];

        // Test outer area
        assert_eq!(
            point_in_polygon_rayster(&[4.0, 1.5], &polygon, &rings, true),
            vec![0]
        );
        assert_eq!(
            point_in_polygon_rayster(&[-1.0, 1.5], &polygon, &rings, true),
            vec![0]
        );

        // Test hole area
        assert_eq!(
            point_in_polygon_rayster(&[1.5, 1.5], &polygon, &rings, true),
            vec![0]
        );

        // Test valid area
        assert_eq!(
            point_in_polygon_rayster(&[0.5, 0.5], &polygon, &rings, true),
            vec![1]
        );
        assert_eq!(
            point_in_polygon_rayster(&[2.5, 0.5], &polygon, &rings, true),
            vec![1]
        );

        // Test boundary
        assert_eq!(
            point_in_polygon_rayster(&[3.0, 1.5], &polygon, &rings, true),
            vec![1]
        );
        assert_eq!(
            point_in_polygon_rayster(&[3.0, 1.5], &polygon, &rings, false),
            vec![0]
        );
    }

    #[test]
    fn test_square_with_hole_grid_points() {
        // 1.1. 输入
        // a. 点云：在-10到10范围内，间隔0.001的均匀点阵
        let step = 0.005;
        let range_start = -10.0;
        let range_end = 10.0;
        let points_per_axis = ((range_end - range_start) / step) as usize + 1;
        let num_points = points_per_axis * points_per_axis;
        let mut points = Vec::with_capacity(num_points * 2);

        // 生成均匀点阵
        for x in 0..points_per_axis {
            let x_coord = range_start + (x as f32) * step;
            for y in 0..points_per_axis {
                let y_coord = range_start + (y as f32) * step;
                points.push(x_coord);
                points.push(y_coord);
            }
        }

        // b. 多边形路径点[0, 0, 3, 0, 3, 3, 0, 3, 1, 1, 2, 1, 2, 2, 1, 2]
        let polygon = vec![
            0.0, 0.0, 3.0, 0.0, 3.0, 3.0, 0.0, 3.0, // Outer ring
            1.0, 1.0, 2.0, 1.0, 2.0, 2.0, 1.0, 2.0, // Hole
        ];

        // c. 多边形路径点的拆分[4]
        let rings = vec![4];

        // d. 边界上点是否考虑为内部true
        let boundary_is_inside = true;

        // 测量执行时间
        let start = Instant::now();
        let results = point_in_polygon_rayster(&points, &polygon, &rings, boundary_is_inside);
        let duration = start.elapsed();

        println!(
            "raycaster Point in square polygon test with {} points took: {:?}",
            num_points, duration
        );

        // 1.2. 验证输出正确判定
        let mut correct_count = 0;
        let mut total_count = 0;

        for i in 0..num_points {
            let x = points[i * 2] as f64;
            let y = points[i * 2 + 1] as f64;
            let result = results[i];

            let expected = if x > 3.0 || x < 0.0 || y > 3.0 || y < 0.0 {
                // a. 在大正方形(外部多边形)外部的点判定为0
                0
            } else if x > 1.0 && x < 2.0 && y > 1.0 && y < 2.0 {
                // b. 在小正方形(洞)内部的点判定为0
                0
            } else {
                // c. 其余点判定为1
                1
            };

            if result == expected {
                correct_count += 1;
            }
            total_count += 1;
        }

        let accuracy = (correct_count as f64 / total_count as f64) * 100.0;
        println!(
            "raycaster Point in square polygon test accuracy: {}/{} = {:.6}%",
            correct_count, total_count, accuracy
        );

        // 确保准确率至少为99.9%（允许一些浮点精度误差）
        assert!(correct_count as f64 / total_count as f64 > 0.999);
    }

    #[test]
    fn test_circle_with_holes_grid_points() {
        // 1.1. 输入
        // a. 点云：在-10到10范围内，间隔0.01的均匀点阵
        let step = 0.01;
        let range_start = -10.0;
        let range_end = 10.0;
        let points_per_axis = ((range_end - range_start) / step) as usize + 1;
        let num_points = points_per_axis * points_per_axis;
        let mut points = Vec::with_capacity(num_points * 2);

        // 生成均匀点阵
        for x in 0..points_per_axis {
            let x_coord = range_start + (x as f32) * step;
            for y in 0..points_per_axis {
                let y_coord = range_start + (y as f32) * step;
                points.push(x_coord);
                points.push(y_coord);
            }
        }

        // b. 构造圆形多边形（大圆半径5，两个小圆半径1，圆心分别在(-2,0)和(2,0)）
        let segments = 64 * 2; // 圆形的近似线段数
        let mut polygon = Vec::new();

        // 构造外部大圆 (0,0) r=5
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
            polygon.push(5.0 * angle.cos()); // x
            polygon.push(5.0 * angle.sin()); // y
        }

        // 构造第一个小圆洞 (-2,0) r=1
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
            polygon.push(-2.0 + angle.cos()); // x
            polygon.push(angle.sin()); // y
        }

        // 构造第二个小圆洞 (2,0) r=1
        for i in 0..segments {
            let angle = 2.0 * std::f32::consts::PI * (i as f32) / (segments as f32);
            polygon.push(2.0 + angle.cos()); // x
            polygon.push(angle.sin()); // y
        }

        // c. 多边形路径点的拆分 [外圆顶点数, 外圆+第一个洞顶点数]
        let rings = vec![segments as u32, segments * 2 as u32];

        // d. 边界上点是否考虑为内部
        let boundary_is_inside = true;


        // 测量执行时间
        let start = Instant::now();
        let results = point_in_polygon_rayster(&points, &polygon, &rings, boundary_is_inside);
        let duration = start.elapsed();

        println!(
            "raycaster Point in circle polygon test with {} points took: {:?}",
            num_points, duration
        );

        // 验证输出正确判定
        let mut correct_count = 0;
        let mut total_count = 0;

        for i in 0..num_points {
            let x = points[i * 2] as f64;
            let y = points[i * 2 + 1] as f64;
            let result = results[i];

            // 计算点到三个圆心的距离
            let dist_to_main = (x * x + y * y).sqrt(); // 到大圆圆心(0,0)的距离
            let dist_to_hole1 = ((x + 2.0) * (x + 2.0) + y * y).sqrt(); // 到第一个小圆圆心(-2,0)的距离
            let dist_to_hole2 = ((x - 2.0) * (x - 2.0) + y * y).sqrt(); // 到第二个小圆圆心(2,0)的距离

            let expected = if dist_to_main > 5.0 {
                // 在大圆外
                0
            } else if dist_to_hole1 < 1.0 || dist_to_hole2 < 1.0 {
                // 在任一小圆内
                0
            } else {
                1
            };

            if result == expected {
                correct_count += 1;
            }
            total_count += 1;
        }

        let accuracy = (correct_count as f64 / total_count as f64) * 100.0;
        println!(
            "raycaster Point in circle polygon test accuracy: {}/{} = {:.6}%",
            correct_count, total_count, accuracy
        );

        // 确保准确率至少为99%（由于圆形是用多边形近似，允许稍大的误差）
        assert!(correct_count as f64 / total_count as f64 > 0.99);
    }
}
