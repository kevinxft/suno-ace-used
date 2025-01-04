use chrono::NaiveDate;
use serde_json::{Map, Value};
use std::fs;

pub fn update_readme(history: &Map<String, Value>) -> std::io::Result<()> {
    let mut table_rows = String::new();
    let mut daily_usages = Vec::new();

    // 将历史数据转换为可排序的向量
    let mut history_vec: Vec<_> = history.iter().collect();
    history_vec.sort_by(|a, b| {
        NaiveDate::parse_from_str(a.0, "%Y-%m-%d")
            .unwrap()
            .cmp(&NaiveDate::parse_from_str(b.0, "%Y-%m-%d").unwrap())
    });

    // 计算每日使用量
    for i in 0..history_vec.len() {
        let (date, data) = history_vec[i];
        let remaining = data["remaining_amount"]
            .as_str()
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.0);

        let usage = if i > 0 {
            let prev_remaining = history_vec[i - 1].1["remaining_amount"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0);
            
            if remaining <= prev_remaining {
                prev_remaining - remaining
            } else {
                0.0 // 如果余额增加，说明充值了，使用量记为0
            }
        } else {
            0.0
        };

        if usage > 0.0 {
            daily_usages.push(usage);
        }

        let timestamp = data["timestamp"].as_str().unwrap_or(date);
        table_rows.push_str(&format!(
            "|{}|{}|{:.2}|{}|\n",
            date, remaining, usage, timestamp
        ));
    }

    // 计算统计信息
    let total_usage: f64 = daily_usages.iter().sum();
    let valid_days = daily_usages.len();
    let avg_usage = if valid_days > 0 {
        total_usage / valid_days as f64
    } else {
        0.0
    };

    // 生成 README 内容
    let readme_content = format!(
        r#"# AceData API 余额监控

## 统计信息
- 累计使用量：{:.2} 元
- 有效统计天数：{} 天
- 日均使用量：{:.2} 元

## 余额记录
|日期|剩余额度|当日使用|记录时间|
|---|---|---|---|
{}
"#,
        total_usage, valid_days, avg_usage, table_rows
    );

    // 写入 README.md
    fs::write("README.md", readme_content)?;
    Ok(())
}
