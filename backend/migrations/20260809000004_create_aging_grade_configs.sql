-- batch-15 P3: 账龄档位配置表
CREATE TABLE IF NOT EXISTS aging_grade_configs (
    id BIGSERIAL PRIMARY KEY,
    grade_name VARCHAR(50) NOT NULL,
    min_days INTEGER NOT NULL,
    max_days INTEGER NOT NULL,
    sort_order INTEGER NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT TRUE,
    remark TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- 插入默认账龄档位
INSERT INTO aging_grade_configs (grade_name, min_days, max_days, sort_order) VALUES
    ('当前', 0, 0, 1),
    ('1-30天', 1, 30, 2),
    ('31-60天', 31, 60, 3),
    ('61-90天', 61, 90, 4),
    ('91-180天', 91, 180, 5),
    ('181-365天', 181, 365, 6),
    ('1年以上', 366, -1, 7)
ON CONFLICT DO NOTHING;

COMMENT ON TABLE aging_grade_configs IS '账龄档位配置表';
