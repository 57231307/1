-- P3-4 BI 数据仓库：日期维表
-- 标准日期维度（年/季/月/周/日 + 是否周末/节假日 + 财年）

CREATE TABLE IF NOT EXISTS dim_dates (
    id BIGSERIAL PRIMARY KEY,
    date DATE NOT NULL UNIQUE,
    year SMALLINT NOT NULL,
    quarter SMALLINT NOT NULL,
    month SMALLINT NOT NULL,
    week SMALLINT NOT NULL,
    day_of_week SMALLINT NOT NULL,
    is_weekend BOOLEAN NOT NULL,
    is_holiday BOOLEAN NOT NULL DEFAULT false,
    fiscal_year SMALLINT,
    fiscal_quarter SMALLINT
);

CREATE INDEX IF NOT EXISTS idx_dim_dates_year_month ON dim_dates (year, month);
CREATE INDEX IF NOT EXISTS idx_dim_dates_year ON dim_dates (year);
CREATE INDEX IF NOT EXISTS idx_dim_dates_quarter ON dim_dates (year, quarter);

COMMENT ON TABLE dim_dates IS 'P3-4 BI 日期维表';
COMMENT ON COLUMN dim_dates.day_of_week IS '1=周一，7=周日';
