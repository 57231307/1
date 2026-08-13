use bingxi_backend::middleware::deprecation::*;


#[test]
fn test_deprecation_middleware_exists() {
    // 验证中间件函数存在且可调用
    let fn_ptr = deprecation_headers_middleware as *const ();
    assert!(!fn_ptr.is_null(), "deprecation_headers_middleware 函数指针不应为空");
}