//! 通用工具函数单元测试

use bingxi_backend::utils::response::ApiResponse;

#[test]
fn test_api_response_success() {
    let response = ApiResponse::success("test data");
    assert_eq!(response.code, Some(200));
    assert!(response.message.is_none());
    assert_eq!(response.data.unwrap(), "test data");
}

#[test]
fn test_api_response_error() {
    let response: ApiResponse<()> = ApiResponse::error("Something went wrong");
    assert_eq!(response.code, Some(500));
    assert_eq!(response.message.as_ref().unwrap(), "Something went wrong");
    assert!(response.data.is_none());
}

#[test]
fn test_api_response_with_message() {
    let response = ApiResponse::success_with_message(42, "custom message");
    assert_eq!(response.code, Some(200));
    assert_eq!(response.message.unwrap(), "custom message");
    assert_eq!(response.data.unwrap(), 42);
}

#[test]
fn test_api_response_json_serialization() {
    let response = ApiResponse::success(vec![1, 2, 3]);
    let json = serde_json::to_value(&response).unwrap();

    assert_eq!(json["code"], 200);
    assert_eq!(json["data"], serde_json::json!([1, 2, 3]));
}

#[test]
fn test_api_response_empty_data() {
    let response: ApiResponse<()> = ApiResponse::success(());
    assert_eq!(response.code, Some(200));
    assert!(response.data.is_some());
}

#[test]
fn test_string_manipulation() {
    let test_str = "  Hello World  ";
    let trimmed = test_str.trim();

    assert_eq!(trimmed, "Hello World");
    assert_ne!(trimmed, test_str);
}

#[test]
fn test_collection_operations() {
    let mut items = vec!["a", "b", "c"];

    items.push("d");
    assert_eq!(items.len(), 4);

    items.retain(|&x| x != "b");
    assert_eq!(items.len(), 3);
    assert!(!items.contains(&"b"));

    items.sort();
    assert_eq!(items, vec!["a", "c", "d"]);
}

#[test]
fn test_option_handling() {
    let some_value: Option<i32> = Some(42);
    let none_value: Option<i32> = None;

    assert!(some_value.is_some());
    assert_eq!(some_value, Some(42));
    assert!(none_value.is_none());

    let mapped = some_value.map(|x| x * 2);
    assert_eq!(mapped, Some(84));
}

#[test]
fn test_result_handling() {
    let ok_result: Result<i32, &str> = Ok(42);
    let err_result: Result<i32, &str> = Err("error");

    assert!(ok_result.is_ok());
    assert_eq!(ok_result, Ok(42));
    assert!(err_result.is_err());
    assert_eq!(err_result, Err("error"));

    let converted = err_result.map_err(|e| e.to_uppercase());
    assert_eq!(converted, Err("ERROR".to_string()));
}

#[test]
fn test_date_formatting() {
    use chrono::Utc;

    let now = Utc::now();
    let date = now.date_naive();

    let yesterday = date - chrono::Duration::days(1);
    assert!(yesterday < date);

    let formatted = date.format("%Y-%m-%d").to_string();
    assert!(formatted.contains('-'));
    assert_eq!(formatted.len(), 10);
}

#[test]
fn test_number_formatting() {
    let num: f64 = 1234.5678;

    let rounded = (num * 100.0).round() / 100.0;
    assert!((rounded - 1234.57).abs() < 0.001);

    let integer = 123;
    assert_eq!(integer % 2, 1);
    assert_eq!(integer / 2, 61);
}

#[test]
fn test_hash_map_operations() {
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
