use plotters::prelude::*;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;

fn generate_trend_graph(daily_stats: &BTreeMap<String, (f64, f64, f64)>) -> String {
    const OUT_FILE_NAME: &str = "assets/trend.svg";
    const SVG_WIDTH: u32 = 800;
    const SVG_HEIGHT: u32 = 400;

    // 确保 assets 目录存在
    fs::create_dir_all("assets").unwrap();

    // 获取最近14天的数据
    let stats: Vec<(String, f64)> = daily_stats
        .iter()
        .rev()
        .take(14)
        .map(|(date, (_, _, daily))| (date.clone(), *daily))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    if stats.is_empty() {
        return String::from("<p>暂无足够数据生成走势图</p>");
    }

    // 创建SVG后端
    let root = SVGBackend::new(OUT_FILE_NAME, (SVG_WIDTH, SVG_HEIGHT)).into_drawing_area();
    root.fill(&WHITE).unwrap();

    // 找出数据范围
    let max_value = stats.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let min_value = stats.iter().map(|(_, v)| *v).fold(max_value, f64::min);
    let y_range = if (max_value - min_value).abs() < f64::EPSILON {
        min_value - 1.0..min_value + 1.0
    } else {
        let padding = (max_value - min_value) * 0.1;
        (min_value - padding).max(0.0)..(max_value + padding)
    };

    let x_range = 0.0..stats.len() as f64;

    // 创建图表
    let mut chart = ChartBuilder::on(&root)
        .margin(50)
        .caption(
            if stats.len() == 1 {
                "使用量走势图（当前仅有一天数据）"
            } else {
                "使用量走势图（最近14天）"
            },
            ("sans-serif", 20).into_font().color(&BLACK),
        )
        .x_label_area_size(35)
        .y_label_area_size(45)
        .build_cartesian_2d(x_range, y_range)
        .unwrap();

    // 设置网格样式
    chart
        .configure_mesh()
        .x_labels(stats.len())
        .x_label_formatter(&|x| {
            if let Some((date, _)) = stats.get(*x as usize) {
                date[5..].to_string() // 只显示月-日
            } else {
                String::new()
            }
        })
        .y_label_formatter(&|y| format!("{:.1}", y))
        .axis_style(BLACK.mix(0.8))
        .light_line_style(BLACK.mix(0.2))
        .draw()
        .unwrap();

    // 绘制数据线和点
    if stats.len() > 1 {
        // 绘制折线
        chart
            .draw_series(LineSeries::new(
                stats
                    .iter()
                    .enumerate()
                    .map(|(i, (_, value))| (i as f64, *value)),
                RGBColor(33, 150, 243).filled(),
            ))
            .unwrap();

        // 绘制数据点
        chart
            .draw_series(PointSeries::of_element(
                stats
                    .iter()
                    .enumerate()
                    .map(|(i, (_, value))| (i as f64, *value)),
                5,
                RGBColor(33, 150, 243).filled(),
                &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
            ))
            .unwrap();
    } else {
        // 单个数据点
        chart
            .draw_series(PointSeries::of_element(
                vec![(0.0, stats[0].1)],
                5,
                RGBColor(33, 150, 243).filled(),
                &|coord, size, style| EmptyElement::at(coord) + Circle::new((0, 0), size, style),
            ))
            .unwrap();
    }

    // 保存图表
    root.present().unwrap();

    // 返回引用链接
    format!("![使用量走势图]({})", OUT_FILE_NAME)
}

pub fn update_readme(history: &Map<String, Value>) -> Result<(), Box<dyn std::error::Error>> {
    // 将数据转换为可排序的结构
    let mut daily_stats: BTreeMap<String, (f64, f64, f64)> = BTreeMap::new();

    // 收集所有数据
    for (date, data) in history {
        let used = data["used_amount"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        let remaining = data["remaining_amount"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        daily_stats.insert(date.clone(), (used, remaining, 0.0)); // 第三个值用于存储当日消耗
    }

    // 计算每日消耗
    let dates: Vec<String> = daily_stats.keys().cloned().collect();
    for i in 1..dates.len() {
        let current_date = &dates[i];
        let prev_date = &dates[i - 1];
        if let (Some(&(current_used, _, _)), Some(&(prev_used, _, _))) =
            (daily_stats.get(current_date), daily_stats.get(prev_date))
        {
            let daily_usage = current_used - prev_used;
            if let Some(stat) = daily_stats.get_mut(current_date) {
                stat.2 = daily_usage;
            }
        }
    }

    // 为第一天设置初始消耗量
    if let Some(first_date) = dates.first() {
        if let Some(stat) = daily_stats.get_mut(first_date) {
            stat.2 = stat.0; // 第一天的消耗就是其总使用量
        }
    }

    // 生成 README 内容
    let mut readme_content = String::from(
        r#"# Suno API 使用量统计

这个仓库用于追踪 Suno API 的使用情况，每两小时自动更新一次数据。

## 使用量走势
"#,
    );

    // 添加走势图
    readme_content.push_str(&generate_trend_graph(&daily_stats));

    // 添加表格标题和表头
    readme_content.push_str(
        r#"
## 每日消耗统计

| 日期 | 当日消耗量 | 累计使用量 | 剩余额度 |
|------|------------|------------|-----------|
"#,
    );

    // 添加表格内容（倒序）
    for (date, (used, remaining, daily)) in daily_stats.iter().rev() {
        readme_content.push_str(&format!(
            "| {} | {:.2} | {:.2} | {:.2} |\n",
            date, daily, used, remaining
        ));
    }

    // 添加说明
    readme_content.push_str(
        r#"
## 说明

- 当日消耗量：当天的使用量相比前一天的增加值
- 累计使用量：从开始统计到当天的总使用量
- 剩余额度：当天的剩余可用额度

数据每两小时自动更新一次，通过 GitHub Actions 自动运行。"#,
    );

    // 写入文件
    fs::write("README.md", readme_content)?;
    Ok(())
}
