#[cfg(test)]
mod tests {
use bingxi_backend::utils::docx_export::*;


    #[test]
    fn test_docx_gj_jbbg() {
        let table = DocxTable {
            title: "测试文档".to_string(),
            subtitle: None,
            headers: vec!["编号".to_string(), "名称".to_string()],
            rows: vec![
                vec!["001".to_string(), "测试项目1".to_string()],
                vec!["002".to_string(), "测试项目2".to_string()],
            ],
        };
        let result = build_docx(&table);
        assert!(result.is_ok(), "docx 构建应成功");
        let bytes = result.unwrap();
        // docx 文件以 PK 开头（zip 格式）
        assert_eq!(&bytes[0..2], b"PK", "docx 文件应以 PK 开头（zip 格式）");
        assert!(bytes.len() > 1000, "docx 文件大小异常: {}", bytes.len());
    }

    #[test]
    fn test_docx_gj_ksj() {
        let table = DocxTable {
            title: "空表".to_string(),
            subtitle: None,
            headers: vec![],
            rows: vec![],
        };
        let result = build_docx(&table);
        assert!(result.is_ok(), "docx 空表构建应成功");
    }

    #[test]
    fn test_docx_xy_zqd_content_type() {
        let response = docx_response(vec![1, 2, 3], "test");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            content_type,
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
        );
    }

    #[test]
    fn test_docx_xy_zqd_content_disposition() {
        let response = docx_response(vec![1, 2, 3], "sales_contract_001");
        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(disposition.contains("sales_contract_001.docx"));
        assert!(disposition.contains("attachment"));
    }

    #[test]
    fn test_docx_djzdgj() {
        let kv = DocxKeyValue {
            keys: vec!["合同编号".to_string(), "客户名称".to_string()],
            values: vec!["HT-2026-001".to_string(), "客户A".to_string()],
        };
        let result = build_docx_with_kv(
            "销售合同",
            &kv,
            &["序号".to_string(), "产品".to_string()],
            &[
                vec!["1".to_string(), "面料A".to_string()],
                vec!["2".to_string(), "面料B".to_string()],
            ],
        );
        assert!(result.is_ok(), "带键值对 docx 构建应成功");
        let bytes = result.unwrap();
        assert_eq!(&bytes[0..2], b"PK");
    }
}