//! SSRF 防护守卫（服务端请求伪造）
//!
//! 用于校验 Webhook URL 不会指向内网/loopback/云元数据服务。
//!
//! ## 拦截范围
//!
//! - **IPv4 私有网络**（RFC1918）：10.0.0.0/8、172.16.0.0/12、192.168.0.0/16
//! - **IPv4 loopback**：127.0.0.0/8
//! - **IPv4 link-local**：169.254.0.0/16（含云元数据服务 169.254.169.254）
//! - **IPv4 广播地址**：255.255.255.255
//! - **IPv4 未指定地址**：0.0.0.0
//! - **IPv4 保留地址**：240.0.0.0/4
//! - **IPv6 loopback**：`::1`
//! - **IPv6 未指定地址**：`::`
//! - **IPv6 link-local**：`fe80::/10`
//! - **IPv6 唯一本地地址**（ULA）：`fc00::/7`（含 `fd00::/8`）
//! - **特殊主机名**：`localhost`、`.local`、`.internal`、`.intranet`、`.corp`
//!
//! ## DNS Rebinding 防御
//!
//! 仅在 `create_webhook` 时校验一次不足以防御 DNS Rebinding
//! （域名创建时解析为公网 IP，触发时再解析为内网 IP）。
//! 因此 `trigger_webhook` 实际发送 HTTP 请求前**必须**再次调用 [`validate_url`]，
//! 对实时 DNS 解析结果做校验。
//!
//! ## 协议白名单
//!
//! 仅允许 `http` 与 `https`，其他协议（`file://`、`gopher://`、`ftp://` 等）直接拒绝。

use crate::utils::error::AppError;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

/// 校验 URL 是否安全（不指向内网/loopback/云元数据）
///
/// # 参数
/// - `url_str`: 待校验的 URL 字符串
///
/// # 返回
/// - `Ok(())`: URL 安全可用
/// - `Err(AppError)`: URL 不安全或解析失败，错误信息直接返回给客户端
///
/// # 实现细节
///
/// 1. 解析 URL 字符串
/// 2. 检查协议（仅允许 http/https）
/// 3. 提取主机名：
///    - 如果是 IP 字面量 → 检查是否在禁止范围
///    - 如果是域名 → 拒绝明显的主机名黑名单（localhost 等）+ 解析为 IP 后检查
pub fn validate_url(url_str: &str) -> Result<(), AppError> {
    // 1. 解析 URL
    let parsed = url::Url::parse(url_str)
        .map_err(|e| AppError::validation(format!("URL 格式无效: {}", e)))?;

    // 2. 协议白名单
    match parsed.scheme() {
        "http" | "https" => {}
        scheme => {
            return Err(AppError::validation(format!(
                "URL 协议不允许：{}（仅允许 http/https）",
                scheme
            )));
        }
    }

    // 3. 提取主机名
    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::validation("URL 缺少主机名".to_string()))?;

    // 4. 主机名黑名单（不依赖 DNS 解析）
    if is_blocked_hostname(host) {
        return Err(AppError::validation(format!(
            "Webhook URL 主机名被禁止：{}（可能指向本地或私有网络）",
            host
        )));
    }

    // 5. IP 字面量检查
    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_blocked_ip(&ip) {
            return Err(AppError::validation(format!(
                "Webhook URL 指向被禁止的 IP 地址：{}",
                ip
            )));
        }
        return Ok(());
    }

    // 6. 域名解析为 IP 后检查（防御 DNS 解析结果指向内网）
    // 使用 ToSocketAddrs 同步解析（DNS 解析延迟可接受；create_webhook 调用频率低）
    // 超时保护：解析失败时拒绝（fail-secure）
    let port = parsed.port_or_known_default().unwrap_or(80);
    let socket_addr_str = format!("{}:{}", host, port);
    match socket_addr_str.to_socket_addrs() {
        Ok(addrs) => {
            for addr in addrs {
                if is_blocked_ip(&addr.ip()) {
                    return Err(AppError::validation(format!(
                        "Webhook URL 域名 {} 解析到被禁止的 IP {}",
                        host,
                        addr.ip()
                    )));
                }
            }
            Ok(())
        }
        Err(e) => {
            // DNS 解析失败：fail-secure，拒绝 URL
            Err(AppError::validation(format!(
                "Webhook URL 主机名 {} DNS 解析失败: {}",
                host, e
            )))
        }
    }
}

/// 检查主机名是否在黑名单（不需要 DNS 解析的明显内网主机名）
fn is_blocked_hostname(host: &str) -> bool {
    let host_lower = host.to_lowercase();
    // 精确匹配
    if matches!(
        host_lower.as_str(),
        "localhost" | "localhost.localdomain" | "ip6-localhost" | "ip6-loopback"
    ) {
        return true;
    }
    // 后缀匹配（mDNS / 私有 DNS TLD）
    let blocked_suffixes = [".local", ".internal", ".intranet", ".corp", ".lan", ".home"];
    for suffix in blocked_suffixes {
        if host_lower.ends_with(suffix) {
            return true;
        }
    }
    false
}

/// 检查 IP 是否在被禁止范围
fn is_blocked_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => is_blocked_ipv4(v4),
        IpAddr::V6(v6) => is_blocked_ipv6(v6),
    }
}

fn is_blocked_ipv4(ip: &Ipv4Addr) -> bool {
    // 未指定地址 0.0.0.0
    if ip.is_unspecified() {
        return true;
    }
    // 广播地址 255.255.255.255
    if ip.is_broadcast() {
        return true;
    }
    // loopback 127.0.0.0/8
    if ip.is_loopback() {
        return true;
    }
    // link-local 169.254.0.0/16（含云元数据 169.254.169.254）
    if ip.is_link_local() {
        return true;
    }
    // 私有网络 10.0.0.0/8
    if ip.octets()[0] == 10 {
        return true;
    }
    // 私有网络 172.16.0.0/12
    if ip.octets()[0] == 172 && (ip.octets()[1] >= 16 && ip.octets()[1] <= 31) {
        return true;
    }
    // 私有网络 192.168.0.0/16
    if ip.octets()[0] == 192 && ip.octets()[1] == 168 {
        return true;
    }
    // 保留地址 240.0.0.0/4（含 255.255.255.255）
    if ip.octets()[0] >= 240 {
        return true;
    }
    // 多播地址 224.0.0.0/4
    if ip.octets()[0] >= 224 && ip.octets()[0] < 240 {
        return true;
    }
    // 100.64.0.0/10 (CGNAT)
    if ip.octets()[0] == 100 && (ip.octets()[1] >= 64 && ip.octets()[1] <= 127) {
        return true;
    }
    false
}

fn is_blocked_ipv6(ip: &Ipv6Addr) -> bool {
    // 未指定地址 ::
    if ip.is_unspecified() {
        return true;
    }
    // loopback ::1
    if ip.is_loopback() {
        return true;
    }
    // link-local fe80::/10
    if ip.segments()[0] >= 0xfe80 && ip.segments()[0] <= 0xfebf {
        return true;
    }
    // 唯一本地地址 fc00::/7
    if (ip.segments()[0] & 0xfe00) == 0xfc00 {
        return true;
    }
    // IPv4-mapped IPv6（如 ::ffff:127.0.0.1）需要拆开校验 IPv4 部分
    if let Some(v4) = ip.to_ipv4_mapped() {
        return is_blocked_ipv4(&v4);
    }
    // IPv4-compatible IPv6（已弃用但仍需检查）
    if let Some(v4) = ip.to_ipv4_compatible() {
        return is_blocked_ipv4(&v4);
    }
    // 多播 ff00::/8
    if ip.segments()[0] >= 0xff00 && ip.segments()[0] <= 0xffff {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, Ipv6Addr};

    // ============ IPv4 黑名单测试 ============

    #[test]
    fn test_ipv4_loopback_blocked() {
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))));
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(127, 255, 255, 255))));
    }

    #[test]
    fn test_ipv4_rfc1918_blocked() {
        // 10.0.0.0/8
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(10, 255, 255, 255))));
        // 172.16.0.0/12
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(172, 31, 255, 255))));
        // 192.168.0.0/16
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1))));
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))));
    }

    #[test]
    fn test_ipv4_link_local_blocked() {
        // 169.254.0.0/16（含云元数据 169.254.169.254）
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(169, 254, 169, 254))));
    }

    #[test]
    fn test_ipv4_unspecified_blocked() {
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0))));
    }

    #[test]
    fn test_ipv4_broadcast_blocked() {
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(255, 255, 255, 255))));
    }

    #[test]
    fn test_ipv4_public_allowed() {
        assert!(!is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
        assert!(!is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))));
        assert!(!is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(140, 82, 121, 4))));
    }

    #[test]
    fn test_ipv4_cgnat_blocked() {
        // 100.64.0.0/10
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(is_blocked_ip(&IpAddr::V4(Ipv4Addr::new(100, 127, 255, 255))));
    }

    // ============ IPv6 黑名单测试 ============

    #[test]
    fn test_ipv6_loopback_blocked() {
        assert!(is_blocked_ip(&IpAddr::V6(Ipv6Addr::LOCALHOST)));
    }

    #[test]
    fn test_ipv6_unspecified_blocked() {
        assert!(is_blocked_ip(&IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
    }

    #[test]
    fn test_ipv6_link_local_blocked() {
        let ip = Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1);
        assert!(is_blocked_ip(&IpAddr::V6(ip)));
    }

    #[test]
    fn test_ipv6_ula_blocked() {
        // fc00::/7（含 fd00::/8）
        assert!(is_blocked_ip(&IpAddr::V6(Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1))));
        assert!(is_blocked_ip(&IpAddr::V6(Ipv6Addr::new(0xfd00, 0, 0, 0, 0, 0, 0, 1))));
    }

    #[test]
    fn test_ipv4_mapped_ipv6_blocked() {
        // ::ffff:127.0.0.1（IPv4-mapped IPv6 loopback）
        let segments = [0, 0, 0, 0, 0, 0xffff, 0x7f00, 0x0001];
        let ip = Ipv6Addr::from(segments);
        assert!(is_blocked_ip(&IpAddr::V6(ip)));
    }

    // ============ 主机名黑名单测试 ============

    #[test]
    fn test_blocked_hostname_localhost() {
        assert!(is_blocked_hostname("localhost"));
        assert!(is_blocked_hostname("LOCALHOST"));
        assert!(is_blocked_hostname("LocalHost"));
    }

    #[test]
    fn test_blocked_hostname_suffixes() {
        assert!(is_blocked_hostname("foo.local"));
        assert!(is_blocked_hostname("BAR.LOCAL"));
        assert!(is_blocked_hostname("printer.internal"));
        assert!(is_blocked_hostname("gitlab.intranet"));
    }

    #[test]
    fn test_blocked_hostname_public_allowed() {
        assert!(!is_blocked_hostname("example.com"));
        assert!(!is_blocked_hostname("github.com"));
        assert!(!is_blocked_hostname("api.bingxi-erp.com"));
    }

    // ============ URL 校验集成测试 ============

    #[test]
    fn test_validate_url_invalid_format() {
        let result = validate_url("not-a-url");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_disallowed_scheme() {
        let result = validate_url("file:///etc/passwd");
        assert!(result.is_err());
        let result = validate_url("gopher://example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_localhost_blocked() {
        let result = validate_url("http://localhost/webhook");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_loopback_ip_blocked() {
        let result = validate_url("http://127.0.0.1/webhook");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_rfc1918_blocked() {
        assert!(validate_url("http://10.0.0.1/webhook").is_err());
        assert!(validate_url("http://172.16.0.1/webhook").is_err());
        assert!(validate_url("http://192.168.1.1/webhook").is_err());
    }

    #[test]
    fn test_validate_url_metadata_service_blocked() {
        let result = validate_url("http://169.254.169.254/latest/meta-data/");
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_url_public_ip_allowed() {
        // 公网 IP 应当通过（但此测试需要网络，可能 flaky）
        // 仅测试不需要 DNS 解析的 IP 字面量
        let result = validate_url("http://8.8.8.8/webhook");
        assert!(result.is_ok(), "公网 IP 应允许：{:?}", result);
    }
}
