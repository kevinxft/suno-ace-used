use dotenv::dotenv;
use reqwest;
use serde_json::Value;
use std::env;

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
        println!("剩余额度: {:.2}", remaining);
        println!("已使用额度: {:.2}", used);
    } else {
        println!("Error: {}", response.status());
    }

    Ok(())
}
