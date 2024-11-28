use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;

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
