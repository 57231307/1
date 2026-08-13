use bingxi_backend::utils::error::*;
use bingxi_backend::utils::import_export::*;
use std::collections::HashMap;


/// 测试用 CSV 生成函数
fn generate_csv(
    headers: &[String],
    rows: &[HashMap<String, String>],
) -> Result<Vec<u8>, AppError> {
    let mut writer = csv::Writer::from_writer(Vec::new());

    // 写入表头
    writer
        .write_record(headers)
        .map_err(|e| AppError::internal(format!("CSV 头写入失败: {}", e)))?;

    // 写入数据
    for row in rows {
        let record: Vec<String> = headers
            .iter()
            .map(|h| row.get(h).cloned().unwrap_or_default())
            .collect();
        writer
            .write_record(&record)
            .map_err(|e| AppError::internal(format!("CSV 数据写入失败: {}", e)))?;
    }

    writer
        .into_inner()
        .map_err(|e| AppError::internal(format!("CSV 生成失败: {}", e)))
}

#[test]
fn test_csv_parse() {
    let csv_data = b"name,age,city\nAlice,30,Beijing\nBob,25,Shanghai";
    let records = CsvImporter::parse(csv_data).expect("P9-1: CSV 解析失败");

    assert_eq!(records.len(), 2);
    // P9-1: 用 if let Some(...) 替代 .get(...).unwrap()，明确处理键不存在场景
    assert_eq!(records[0].get("name").map(String::as_str), Some("Alice"));
    assert_eq!(records[0].get("age").map(String::as_str), Some("30"));
    assert_eq!(records[1].get("city").map(String::as_str), Some("Shanghai"));
}

#[test]
fn test_csv_generate() {
    let headers = vec!["name".to_string(), "age".to_string()];
    let mut row1 = HashMap::new();
    row1.insert("name".to_string(), "Alice".to_string());
    row1.insert("age".to_string(), "30".to_string());
    let rows = vec![row1];

    let data = generate_csv(&headers, &rows).expect("P9-1: CSV 生成失败");
    let content = String::from_utf8(data).expect("P9-1: UTF-8 解码失败");
    assert!(content.contains("name,age"));
    assert!(content.contains("Alice,30"));
}

#[test]
fn test_field_validator_required() {
    assert!(FieldValidator::required("test", "名称").is_ok());
    assert!(FieldValidator::required("", "名称").is_err());
    assert!(FieldValidator::required("   ", "名称").is_err());
}

#[test]
fn test_field_validator_integer() {
    assert_eq!(
        FieldValidator::integer("42", "数量").expect("P9-1: 整数校验"),
        42
    );
    assert!(FieldValidator::integer("abc", "数量").is_err());
}

#[test]
fn test_field_validator_decimal() {
    assert!(FieldValidator::decimal("99.99", "价格").is_ok());
    assert!(FieldValidator::decimal("abc", "价格").is_err());
}

#[test]
fn test_field_validator_date() {
    assert!(FieldValidator::date("2024-01-15", "日期").is_ok());
    assert!(FieldValidator::date("2024/01/15", "日期").is_err());
}

#[test]
fn test_field_validator_boolean() {
    // P9-1: 改用 expect 替代 unwrap，并明确中文失败原因
    assert!(FieldValidator::boolean("true", "启用").expect("P9-1: 布尔校验"));
    assert!(!FieldValidator::boolean("0", "启用").expect("P9-1: 布尔校验"));
    assert!(FieldValidator::boolean("是", "启用").expect("P9-1: 布尔校验"));
    assert!(FieldValidator::boolean("maybe", "启用").is_err());
}

#[test]
fn test_field_validator_enum() {
    let allowed = &["A", "B", "C"];
    assert_eq!(
        FieldValidator::enum_value("B", "类型", allowed).expect("P9-1: 枚举校验"),
        "B"
    );
    assert!(FieldValidator::enum_value("D", "类型", allowed).is_err());
}

// 死代码清理（2026-06-26）：test_import_format_from_extension 测试的 ImportFormat 已删除。