#[cfg(test)]
mod tests {
use bingxi_backend::utils::xlsx_export::*;


    #[test]
    fn test_xlsx_gj_jbbg() {
        let table = XlsxTable {
            sheet_name: "测试".to_string(),
            headers: vec!["编号".to_string(), "名称".to_string()],
            rows: vec![
                vec!["001".to_string(), "测试项目1".to_string()],
                vec!["002".to_string(), "测试项目2".to_string()],
            ],
        };
        let result = build_xlsx(&table);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // xlsx 文件最小大小约 4KB（zip 压缩格式）
        assert!(bytes.len() > 4000, "xlsx 文件大小异常: {}", bytes.len());
        // xlsx 文件以 PK 开头（zip 格式）
        assert_eq!(&bytes[0..2], b"PK", "xlsx 文件应以 PK 开头（zip 格式）");
    }

    #[test]
    fn test_xlsx_gj_ksj() {
        let table = XlsxTable {
            sheet_name: "空表".to_string(),
            headers: vec!["列1".to_string()],
            rows: vec![],
        };
        let result = build_xlsx(&table);
        assert!(result.is_ok());
    }

    #[test]
    fn test_xlsx_xy_zqd_content_type() {
        let response = xlsx_response(vec![1, 2, 3], "test");
        let content_type = response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert_eq!(
            content_type,
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
    }

    #[test]
    fn test_xlsx_xy_zqd_content_disposition() {
        let response = xlsx_response(vec![1, 2, 3], "crm_leads_export");
        let disposition = response
            .headers()
            .get(header::CONTENT_DISPOSITION)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(disposition.contains("crm_leads_export.xlsx"));
        assert!(disposition.contains("attachment"));
    }

    /// V15 P0-S15：WatermarkConfig::render 全字段填充应输出 4 段
    #[test]
    fn test_watermark_render_qzd() {
        let wm = WatermarkConfig {
            operator: Some("admin".to_string()),
            ip_address: Some("127.0.0.1".to_string()),
            exported_at: Some("2026-07-17T10:00:00Z".to_string()),
            extra: Some("客户列表导出".to_string()),
        };
        let rendered = wm.render().expect("应输出水印文本");
        assert!(rendered.contains("操作员:admin"));
        assert!(rendered.contains("导出IP:127.0.0.1"));
        assert!(rendered.contains("导出时间:2026-07-17T10:00:00Z"));
        assert!(rendered.contains("客户列表导出"));
    }

    /// V15 P0-S15：WatermarkConfig::render 全字段为 None 应返回 None
    #[test]
    fn test_watermark_render_qk() {
        let wm = WatermarkConfig::default();
        assert!(wm.render().is_none());
    }

    /// V15 P0-S15：带水印 xlsx 构建应成功且文件大小合理
    #[test]
    fn test_xlsx_dsy_gj() {
        let table = XlsxTable {
            sheet_name: "客户列表".to_string(),
            headers: vec!["编码".to_string(), "名称".to_string()],
            rows: vec![
                vec!["C001".to_string(), "客户A".to_string()],
                vec!["C002".to_string(), "客户B".to_string()],
            ],
        };
        let wm = WatermarkConfig {
            operator: Some("admin".to_string()),
            ip_address: Some("10.0.0.1".to_string()),
            exported_at: Some("2026-07-17T10:00:00Z".to_string()),
            extra: Some("合规导出".to_string()),
        };
        let result = build_xlsx_with_watermark(&table, &wm);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // xlsx 文件最小约 4KB（带水印应略大）
        assert!(
            bytes.len() > 4000,
            "xlsx 带水印文件大小异常: {}",
            bytes.len()
        );
        assert_eq!(&bytes[0..2], b"PK");
    }

    /// V15 P0-S15：水印为空时退化为 build_xlsx 行为（向后兼容）
    #[test]
    fn test_xlsx_dsy_ksyyth() {
        let table = XlsxTable {
            sheet_name: "测试".to_string(),
            headers: vec!["列1".to_string()],
            rows: vec![vec!["v1".to_string()]],
        };
        let wm = WatermarkConfig::default();
        let with_wm = build_xlsx_with_watermark(&table, &wm).expect("应成功");
        let without_wm = build_xlsx(&table).expect("应成功");
        // 退化路径直接调用 build_xlsx，字节流应完全一致
        assert_eq!(with_wm, without_wm);
    }
}