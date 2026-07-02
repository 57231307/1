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

// 以下 7 个测试原为测试 Rust 标准库（String/Vec/Option/Result/chrono/算术/HashMap），
// 与 utils::response 模块无关，已于批次 65 清理。
