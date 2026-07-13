-- B-P1-8 修复（批次 365 v13 复审）：回滚事件幂等去重表
DROP TABLE IF EXISTS processed_events;
