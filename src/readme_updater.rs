use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;

fn generate_trend_graph(daily_stats: &BTreeMap<String, (f64, f64, f64)>) -> String {
    const GRAPH_WIDTH: usize = 60;
    const GRAPH_HEIGHT: usize = 10;

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
        return String::from("暂无足够数据生成走势图");
    }

    // 找出最大和最小值
    let max_value = stats.iter().map(|(_, v)| *v).fold(0.0, f64::max);
    let min_value = stats.iter().map(|(_, v)| *v).fold(max_value, f64::min);
    let value_range = if (max_value - min_value).abs() < f64::EPSILON {
        1.0 // 避免除以零
    } else {
        max_value - min_value
    };

    // 生成图表
    let mut graph = String::new();
    graph.push_str("\n```\n消耗量走势图");
    
    // 如果只有一天数据，显示特殊提示
    if stats.len() == 1 {
        graph.push_str("（当前仅有一天数据）");
    } else {
        graph.push_str("（最近14天）");
    }
    graph.push_str(":\n\n");

    // Y轴刻度
    for i in (0..GRAPH_HEIGHT).rev() {
        let value = if stats.len() == 1 {
            // 单天数据时，以当前值为中心，上下浮动20%
            let current = stats[0].1;
            let range = current * 0.2;
            current - range + (range * 2.0 * i as f64 / (GRAPH_HEIGHT as f64 - 1.0))
        } else {
            min_value + (value_range * (i as f64) / (GRAPH_HEIGHT as f64 - 1.0))
        };
        graph.push_str(&format!("{:5.1} │", value));

        // 添加数据点和连线
        let mut last_pos = None;
        for (idx, (_, daily)) in stats.iter().enumerate() {
            let pos =
                ((GRAPH_WIDTH - 1) as f64 * idx as f64 / (stats.len() - 1).max(1) as f64) as usize;
            let normalized_height = if stats.len() == 1 {
                (GRAPH_HEIGHT / 2) as f64
            } else {
                (GRAPH_HEIGHT - 1) as f64 * (*daily - min_value) / value_range
            };
            let current_height = normalized_height.round() as usize;

            if i == current_height {
                let spaces = if let Some(last) = last_pos {
                    if pos > last {
                        pos - last - 1
                    } else {
                        0
                    }
                } else {
                    pos
                };
                graph.push_str(&" ".repeat(spaces));
                graph.push('●');
                last_pos = Some(pos);
            }
        }
        graph.push('\n');
    }

    // X轴
    graph.push_str("      └");
    graph.push_str(&"─".repeat(GRAPH_WIDTH));
    graph.push('\n');

    // X轴日期标签
    graph.push_str("        ");
    if stats.len() == 1 {
        // 单天数据时只显示一个日期
        let date = &stats[0].0;
        let date_str = &date[5..]; // 只显示月-日
        graph.push_str(date_str);
    } else {
        // 多天数据时显示首尾日期
        let label_positions = [0, stats.len() - 1];
        for pos in label_positions {
            if pos < stats.len() {
                let date = &stats[pos].0;
                let date_str = &date[5..]; // 只显示月-日
                graph.push_str(&format!(
                    "{:<width$}",
                    date_str,
                    width = if pos == 0 { 30 } else { 0 }
                ));
            }
        }
    }

    graph.push_str("\n```\n");
    graph
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
