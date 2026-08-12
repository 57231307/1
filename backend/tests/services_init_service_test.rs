    use bingxi_backend::services::init_service::*;
#[cfg(test)]
mod tests {

    #[test]
    fn to_connection_string_preserves_ip_host() {
        // 回退测试：确保 host 中合法的 IP 字符（数字、.）不会被错误编码
        // 批次 28 v7 P0-2 修复：原测试数据使用真实生产 IP，已改为 RFC 5737 文档示例段
        let cfg = DatabaseConfig {
            host: "192.0.2.100".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "bingxi".to_string(),
            password: "p@ss word".to_string(),
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer（原为 disable）
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        // 关键断言：host 段不应被编码
        assert!(
            s.contains("@192.0.2.100:"),
            "host 不应被 percent-encoding，连接串 = {}",
            s
        );
        // 同时确保 username/password 仍然被正确编码
        assert!(
            s.starts_with("postgres://bingxi:p%40ss%20word@"),
            "s = {}",
            s
        );
        // v5 审计批次 21：ssl_mode 缺省时默认 prefer
        assert!(s.ends_with("/bingxi?sslmode=prefer"));
    }

    #[test]
    fn to_connection_string_preserves_dns_host() {
        // DNS 主机名也必须原样保留
        let cfg = DatabaseConfig {
            host: "db.example.com".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        assert!(s.contains("@db.example.com:5432/"), "s = {}", s);
    }

    #[test]
    fn to_connection_string_preserves_ipv6_host() {
        // IPv6 主机名应保留方括号（注意：这里我们只做 verbatim 透传；
        // 真正使用 IPv6 时应额外处理）
        let cfg = DatabaseConfig {
            host: "[::1]".to_string(),
            port: "5432".to_string(),
            name: "bingxi".to_string(),
            username: "u".to_string(),
            password: "p".to_string(),
            // v5 审计批次 21：ssl_mode 缺省时回退到 prefer
            ssl_mode: None,
        };
        let s = cfg.to_connection_string();
        assert!(s.contains("@[::1]:5432/"), "s = {}", s);
    }
}