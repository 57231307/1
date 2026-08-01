//! 数据本地化配置（V15 类八缺陷项2：数据跨境传输合规评估）
//!
//! 配置项：
//! - DATA_LOCALITY_MODE：permissive（默认，允许跨境）| cn-only（禁止出境）
//! - DATA_LOCALITY_OVERSEAS_IP_BLOCKLIST：境外 IP 拦截列表（逗号分隔的 CIDR）

use serde::Deserialize;
use std::net::Ipv4Addr;
use std::str::FromStr;

/// 数据本地化模式
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub enum DataLocalityMode {
    /// 宽松模式（默认，允许跨境数据传输）
    Permissive,
    /// 仅限中国境内（禁止向境外 IP 发起 outbound 请求）
    CnOnly,
}

impl Default for DataLocalityMode {
    fn default() -> Self {
        Self::Permissive
    }
}

impl FromStr for DataLocalityMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "permissive" => Ok(Self::Permissive),
            "cn-only" | "cn_only" => Ok(Self::CnOnly),
            _ => Err(format!("无效的 DATA_LOCALITY_MODE：{}（可选：permissive / cn-only）", s)),
        }
    }
}

/// CIDR 网段（IPv4）
#[derive(Debug, Clone)]
pub struct IpCidr {
    pub network: Ipv4Addr,
    pub prefix_len: u8,
}

impl IpCidr {
    pub fn contains_ipv4(&self, ip: &Ipv4Addr) -> bool {
        if self.prefix_len == 0 {
            return true;
        }
        let network_u32 = u32::from(self.network);
        let ip_u32 = u32::from(*ip);
        let mask = if self.prefix_len >= 32 {
            0xFFFFFFFFu32
        } else {
            !0u32 << (32 - self.prefix_len)
        };
        (network_u32 & mask) == (ip_u32 & mask)
    }
}

impl FromStr for IpCidr {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(format!("无效的 CIDR 格式：{}（应为 network/prefix_len）", s));
        }
        let network = Ipv4Addr::from_str(parts[0])
            .map_err(|e| format!("无效的 IP 地址 {}：{}", parts[0], e))?;
        let prefix_len = parts[1].parse::<u8>()
            .map_err(|e| format!("无效的前缀长度 {}：{}", parts[1], e))?;
        if prefix_len > 32 {
            return Err(format!("前缀长度不能超过 32：{}", prefix_len));
        }
        Ok(Self { network, prefix_len })
    }
}

/// 数据本地化配置
#[derive(Debug, Clone)]
pub struct DataLocalityConfig {
    pub mode: DataLocalityMode,
    pub overseas_ip_blocklist: Vec<IpCidr>,
}

impl DataLocalityConfig {
    pub fn from_env() -> Result<Self, String> {
        let mode_str = std::env::var("DATA_LOCALITY_MODE").unwrap_or_default();
        let mode = if mode_str.is_empty() {
            DataLocalityMode::default()
        } else {
            mode_str.parse()?
        };

        let blocklist_str = std::env::var("DATA_LOCALITY_OVERSEAS_IP_BLOCKLIST").unwrap_or_default();
        let overseas_ip_blocklist = if blocklist_str.is_empty() {
            Vec::new()
        } else {
            blocklist_str.split(',')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim().parse())
                .collect::<Result<Vec<_>, _>>()?
        };

        Ok(Self { mode, overseas_ip_blocklist })
    }
}

/// 检查 IP 是否在境外拦截列表中
pub fn is_overseas_blocked(ip: &Ipv4Addr, blocklist: &[IpCidr]) -> bool {
    blocklist.iter().any(|cidr| cidr.contains_ipv4(ip))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_cidr_contains() {
        let cidr: IpCidr = "10.0.0.0/8".parse().unwrap();
        assert!(cidr.contains_ipv4(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(cidr.contains_ipv4(&Ipv4Addr::new(10, 255, 255, 255)));
        assert!(!cidr.contains_ipv4(&Ipv4Addr::new(11, 0, 0, 1)));
    }

    #[test]
    fn test_data_locality_mode_parse() {
        assert_eq!("permissive".parse::<DataLocalityMode>().unwrap(), DataLocalityMode::Permissive);
        assert_eq!("cn-only".parse::<DataLocalityMode>().unwrap(), DataLocalityMode::CnOnly);
        assert!("invalid".parse::<DataLocalityMode>().is_err());
    }

    #[test]
    fn test_is_overseas_blocked() {
        let blocklist = vec![
            "8.8.8.0/24".parse::<IpCidr>().unwrap(),
        ];
        assert!(is_overseas_blocked(&Ipv4Addr::new(8, 8, 8, 8), &blocklist));
        assert!(!is_overseas_blocked(&Ipv4Addr::new(1, 1, 1, 1), &blocklist));
    }
}