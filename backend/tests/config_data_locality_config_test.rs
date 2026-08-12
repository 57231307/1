#[cfg(test)]
mod tests {
use bingxi_backend::config::data_locality_config::*;


    #[test]
    fn test_ip_cidr_contains() {
        let cidr: IpCidr = "10.0.0.0/8".parse().unwrap();
        assert!(cidr.contains_ipv4(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(cidr.contains_ipv4(&Ipv4Addr::new(10, 255, 255, 255)));
        assert!(!cidr.contains_ipv4(&Ipv4Addr::new(11, 0, 0, 1)));
    }

    #[test]
    fn test_data_locality_mode_parse() {
        assert_eq!(
            "permissive".parse::<DataLocalityMode>().unwrap(),
            DataLocalityMode::Permissive
        );
        assert_eq!(
            "cn-only".parse::<DataLocalityMode>().unwrap(),
            DataLocalityMode::CnOnly
        );
        assert!("invalid".parse::<DataLocalityMode>().is_err());
    }

    #[test]
    fn test_is_overseas_blocked() {
        let blocklist = vec!["8.8.8.0/24".parse::<IpCidr>().unwrap()];
        assert!(is_overseas_blocked(&Ipv4Addr::new(8, 8, 8, 8), &blocklist));
        assert!(!is_overseas_blocked(&Ipv4Addr::new(1, 1, 1, 1), &blocklist));
    }
}