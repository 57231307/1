//! 通用工具函数单元测试

use bingxi_backend::utils::response::ApiResponse;
use serde_json::json;

#[test]
fn test_api_response_success() {
    // 测试成功响应
    let response = ApiResponse::success("test data");
    assert!(response.success);
    assert_eq!(response.message, "Success");
    assert_eq!(response.data.unwrap(), "test data");
}

#[test]
fn test_api_response_error() {
    // 测试错误响应
    let response: ApiResponse<()> = ApiResponse::error("Something went wrong");
    assert!(!response.success);
    assert_eq!(response.message, "Something went wrong");
}

#[test]
fn test_api_response_with_message() {
    // 测试自定义消息的成功响应
    let response = ApiResponse::success_with_message("custom message", 42);
    assert!(response.success);
    assert_eq!(response.message, "custom message");
    assert_eq!(response.data.unwrap(), 42);
}

#[test]
fn test_api_response_json_serialization() {
    // 测试API响应的JSON序列化
    let response = ApiResponse::success(vec![1, 2, 3]);
    let json = serde_json::to_value(&response).unwrap();
    
    assert_eq!(json["success"], true);
    assert_eq!(json["message"], "Success");
    assert_eq!(json["data"], json!([1, 2, 3]));
}

#[test]
fn test_api_response_empty_data() {
    // 测试空数据的响应
    let response: ApiResponse<()> = ApiResponse::success(());
    assert!(response.success);
    assert!(response.data.is_none());
}

#[test]
fn test_string_manipulation() {
    // 测试字符串操作
    let test_str = "  Hello World  ";
    let trimmed = test_str.trim();
    
    assert_eq!(trimmed, "Hello World");
    assert_ne!(trimmed, test_str);
}

#[test]
fn test_collection_operations() {
    // 测试集合操作
    let mut items = vec!["a", "b", "c"];
    
    // 添加元素
    items.push("d");
    assert_eq!(items.len(), 4);
    
    // 删除元素
    items.retain(|&x| x != "b");
    assert_eq!(items.len(), 3);
    assert!(!items.contains(&"b"));
    
    // 排序
    items.sort();
    assert_eq!(items, vec!["a", "c", "d"]);
}

#[test]
fn test_option_handling() {
    // 测试Option处理
    let some_value = Some(42);
    let none_value: Option<i32> = None;
    
    assert!(some_value.is_some());
    assert_eq!(some_value.unwrap(), 42);
    assert!(none_value.is_none());
    
    // 测试map操作
    let mapped = some_value.map(|x| x * 2);
    assert_eq!(mapped, Some(84));
}

#[test]
fn test_result_handling() {
    // 测试Result处理
    let ok_result: Result<i32, &str> = Ok(42);
    let err_result: Result<i32, &str> = Err("error");
    
    assert!(ok_result.is_ok());
    assert_eq!(ok_result.unwrap(), 42);
    assert!(err_result.is_err());
    assert_eq!(err_result.unwrap_err(), "error");
    
    // 测试map_err
    let converted = err_result.map_err(|e| e.to_uppercase());
    assert_eq!(converted.unwrap_err(), "ERROR");
}

#[test]
fn test_date_formatting() {
    // 测试日期和时间处理
    use chrono::{NaiveDate, Utc};
    
    let now = Utc::now();
    let date = now.date_naive();
    
    // 测试日期比较
    let yesterday = date - chrono::Duration::days(1);
    assert!(yesterday < date);
    
    // 测试日期格式化
    let formatted = date.format("%Y-%m-%d").to_string();
    assert!(formatted.contains('-'));
    assert_eq!(formatted.len(), 10);
}

#[test]
fn test_number_formatting() {
    // 测试数字格式化
    let num = 1234.5678;
    
    // 四舍五入到2位小数
    let rounded = (num * 100.0).round() / 100.0;
    assert_eq!(rounded, 1234.57);
    
    // 整数操作
    let integer = 123;
    assert_eq!(integer % 2, 1);
    assert_eq!(integer / 2, 61);
}

#[test]
fn test_hash_map_operations() {
    // 测试HashMap操作
    use std::collections::HashMap;
    
    let mut map = HashMap::new();
    map.insert("key1", "value1");
    map.insert("key2", "value2");
    
    assert_eq!(map.len(), 2);
    assert_eq!(map.get("key1"), Some(&"value1"));
    assert_eq!(map.get("key3"), None);
    
    map.remove("key1");
    assert_eq!(map.len(), 1);
}
