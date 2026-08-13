use bingxi_backend::services::scheduling_query::*;
// BE-D 修复（2026-06-26 第三优先级）：
// 原 test_gantt_duration 测试 GanttItem 结构体（已被删除，业务改用 GanttItemDto）。
// 原 test_module_loaded 是恒真断言（常量与自身字面量比较），已删除。
// scheduling_query 的业务逻辑由 scheduling_e2e.rs 集成测试覆盖。