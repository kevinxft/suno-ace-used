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
        let remaining = data["remaining_amount"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);
        daily_stats.insert(date.clone(), (0.0, remaining, 0.0));
    }

    // 计算每日消耗，并处理充值情况
    let dates: Vec<String> = daily_stats.keys().cloned().collect();
    let mut last_valid_date = dates.first().cloned();
    
    for i in 1..dates.len() {
        let current_date = &dates[i];
        let prev_date = &dates[i - 1];
        
        if let (Some(&(_, current_remaining, _)), Some(&(_, prev_remaining, _))) =
            (daily_stats.get(current_date), daily_stats.get(prev_date))
        {
            // 检查是否发生充值（剩余额度增加）
            if current_remaining > prev_remaining {
                // 发生充值，清除之前的统计数据
                for j in 0..i {
                    if let Some(stat) = daily_stats.get_mut(&dates[j]) {
                        stat.2 = 0.0; // 清除之前的消耗统计
                    }
                }
                // 更新最后有效日期
                last_valid_date = Some(current_date.clone());
                // 当天消耗设为0
                if let Some(stat) = daily_stats.get_mut(current_date) {
                    stat.2 = 0.0;
                }
            } else {
                // 正常计算消耗
                let daily_usage = prev_remaining - current_remaining;
                if let Some(stat) = daily_stats.get_mut(current_date) {
                    stat.2 = daily_usage;
                }
            }
        }
    }

    // 生成 README 内容
    let mut readme_content = String::from(
        r#"# Suno API 使用量统计

这个仓库用于追踪 Suno API 的使用情况，每两小时自动更新一次数据。

## 使用统计

"#,
    );

    // 计算日平均消耗和预计剩余天数（只计算最后一次充值后的数据）
    let mut valid_days = 0.0;
    let mut valid_consumption = 0.0;
    
    if let Some(last_valid) = last_valid_date {
        for (date, (_, _, daily)) in daily_stats.iter() {
            if date >= &last_valid {
                valid_consumption += daily;
                valid_days += 1.0;
            }
        }
    }

    let daily_average = if valid_days > 0.0 { valid_consumption / valid_days } else { 0.0 };
    let latest_remaining = daily_stats.values().next_back().map(|(_, remaining, _)| remaining).unwrap_or(&0.0);
    let estimated_days = if daily_average > 0.0 { latest_remaining / daily_average } else { 0.0 };

    // 添加使用统计表格
    readme_content.push_str(&format!(
        r#"
| 指标 | 数值 |
|------|------|
| 日平均消耗 | {:.2} |
| 预计剩余天数 | {:.1} |

"#,
        daily_average,
        estimated_days
    ));

    // 添加走势图
    readme_content.push_str(
        r#"
## 使用量走势
"#,
    );
    readme_content.push_str(&generate_trend_graph(&daily_stats));

    // 添加表格标题和表头
    readme_content.push_str(
        r#"
## 每日消耗统计

| 日期 | 当日消耗量 | 剩余额度 |
|------|------------|-----------|
"#,
    );

    // 添加表格内容（倒序）
    for (date, (_, remaining, daily)) in daily_stats.iter().rev() {
        readme_content.push_str(&format!(
            "| {} | {:.2} | {:.2} |\n",
            date, daily, remaining
        ));
    }

    // 添加说明
    readme_content.push_str(
        r#"
## 说明

- 当日消耗量：当天剩余额度相比前一天剩余额度的减少值
- 剩余额度：当天的剩余可用额度

数据每两小时自动更新一次，通过 GitHub Actions 自动运行。"#,
    );

    // 写入文件
    fs::write("README.md", readme_content)?;
    Ok(())
}
