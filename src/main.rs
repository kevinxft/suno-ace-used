use chrono::Local;
use dotenv::dotenv;
use reqwest;
use serde_json::{json, Map, Value};
use std::env;
use std::fs;
use std::path::Path;

mod readme_updater;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 加载 .env 文件
    dotenv().ok();

    // 获取环境变量
    let authorization = env::var("AUTHORIZATION").expect("AUTHORIZATION not set");
    let app_id = env::var("APP_ID").expect("APP_ID not set");

    // 构建 URL
    let url = format!(
        "https://platform.acedata.cloud/api/v1/applications/{}",
        app_id
    );

    // 创建 HTTP 客户端
    let client = reqwest::Client::new();

    // 发送 GET 请求
    let response = client
        .get(&url)
        .header("Authorization", &authorization)
        .header("Accept", "application/json")
        .send()
        .await?;

    // 检查响应状态并解析 JSON
    if response.status().is_success() {
        let json: Value = response.json().await?;
        let remaining = json["remaining_amount"].as_f64().unwrap_or(0.0);
        let used = json["used_amount"].as_f64().unwrap_or(0.0);

        // 获取当前日期作为键
        let today = Local::now().format("%Y-%m-%d").to_string();

        // 读取现有的 JSON 文件或创建新的
        let file_path = "balance_history.json";
        let mut history: Map<String, Value> = if Path::new(file_path).exists() {
            let content = fs::read_to_string(file_path)?;
            serde_json::from_str(&content)?
        } else {
            Map::new()
        };

        // 获取前一天的数据（如果存在）
        let yesterday = Local::now()
            .checked_sub_days(chrono::Days::new(1))
            .unwrap()
            .format("%Y-%m-%d")
            .to_string();
        
        let should_clear = if let Some(yesterday_data) = history.get(&yesterday) {
            if let (Some(yesterday_remaining), Some(today_remaining)) = (
                yesterday_data["remaining_amount"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok()),
                remaining,
            ) {
                today_remaining > yesterday_remaining
            } else {
                false
            }
        } else {
            false
        };

        // 如果需要清空历史，创建新的空 Map
        if should_clear {
            history = Map::new();
            println!("检测到余额增加，已清空历史数据");
        }

        // 计算今天的使用量
        let today_usage = if let Some(yesterday_data) = history.get(&yesterday) {
            if let (Some(yesterday_remaining), Some(today_remaining)) = (
                yesterday_data["remaining_amount"]
                    .as_str()
                    .and_then(|s| s.parse::<f64>().ok()),
                remaining,
            ) {
                if today_remaining <= yesterday_remaining {
                    yesterday_remaining - today_remaining
                } else {
                    used // 如果今天充值了，就使用API返回的used值
                }
            } else {
                used
            }
        } else {
            used
        };

        // 准备要保存的数据
        let data = json!({
            "remaining_amount": format!("{:.2}", remaining),
            "used_amount": format!("{:.2}", today_usage),
            "timestamp": Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
        });

        // 添加新的数据
        history.insert(today, data);

        // 保存回文件
        fs::write(file_path, serde_json::to_string_pretty(&history)?)?;

        // 更新 README.md
        readme_updater::update_readme(&history)?;

        println!("剩余额度: {:.2}", remaining);
        println!("今日使用额度: {:.2}", today_usage);
        println!("数据已保存到 {} 和 README.md", file_path);
    } else {
        println!("Error: {}", response.status());
    }

    Ok(())
}
