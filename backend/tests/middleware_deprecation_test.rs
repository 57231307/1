#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deprecation_middleware_exists() {
        // 确保中间件函数可以被引用
        let _ = deprecation_headers_middleware;
    }
}