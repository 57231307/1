#[cfg(test)]
mod tests {
use bingxi_backend::handlers::slow_query_handler::*;


    /// 列表查询参数默认值
    #[test]
    fn test_list_params_default() {
        let p = SlowQueryListParams::default();
        assert!(p.start_time.is_none());
        assert!(p.end_time.is_none());
        assert!(p.min_duration.is_none());
        assert!(p.keyword.is_none());
        assert!(p.page.is_none());
        assert!(p.page_size.is_none());
    }
}