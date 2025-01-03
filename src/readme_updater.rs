use chrono::NaiveDate;
use serde_json::{Map, Value};
use std::fs;

pub fn update_readme(history: &Map<String, Value>) -> std::io::Result<()> {
    let mut table_rows = String::new();
    let mut total_usage = 0.0;
    let mut valid_days = 0;

    // 将历史数据转换为可排序的向量
    let mut history_vec: Vec<_> = history.iter().collect();
    history_vec.sort_by(|a, b| {
        NaiveDate::parse_from_str(a.0, "%Y-%m-%d")
            .unwrap()
            .cmp(&NaiveDate::parse_from_str(b.0, "%Y-%m-%d").unwrap())
    });

    // 生成表格行
    for (date, data) in history_vec {
        if let (Some(remaining), Some(used)) = (
            data["remaining_amount"].as_str(),
            data["used_amount"].as_str(),
        ) {
            if let (Ok(remaining), Ok(used)) = (
                remaining.parse::<f64>(),
                used.parse::<f64>(),
            ) {
                let timestamp = data["timestamp"].as_str().unwrap_or(date);
                table_rows.push_str(&format!(
                    "|{}|{}|{}|{}|\n",
                    date, remaining, used, timestamp
                ));
                
                // 累计使用量和有效天数
                if used > 0.0 {
                    total_usage += used;
                    valid_days += 1;
                }
            }
        }
    }

    // 计算平均每日使用量
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
|日期|剩余额度|使用额度|记录时间|
|---|---|---|---|
{}
"#,
        total_usage, valid_days, avg_usage, table_rows
    );

    // 写入 README.md
    fs::write("README.md", readme_content)?;
    Ok(())
}
