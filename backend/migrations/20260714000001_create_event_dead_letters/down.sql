-- B-P1-7 修复（批次 384 v13 复审）：回滚事件死信队列表
DROP TABLE IF EXISTS event_dead_letters;
