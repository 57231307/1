-- v14 批次 423B：化验室打样流程贯通
-- 依据：面料行业真实业务调研文档 §11.1 化验室打样 5 步闭环 + §11.1.1 染色技术卡
-- 真实业务流程：打样通知单 → 打样（ABCD 多版样）→ 色样确认（OK 样）→ 复样 → 建数据库
-- 来源：印染厂技术部研发操作规程 / 染厂化验室操作手册 / 福步外贸论坛跟单流程（2026-07-15 真实调研）

-- ============================================================================
-- 1. 打样通知单表（lab_dip_request）
-- 业务来源：业务跟单接到客户打样需求，录入规范的技术要求
-- 真实必填字段：对色光源/色牢度/打样版数/坯布规格/完成时间
-- ============================================================================
CREATE TABLE IF NOT EXISTS "lab_dip_request" (
    "id" SERIAL PRIMARY KEY,
    -- 单号：LD-YYYYMMDDHHMMSS-NNN（Lab Dip 缩写）
    "request_no" VARCHAR(50) NOT NULL,
    -- 客户信息
    "customer_id" INTEGER,
    "customer_color_no" VARCHAR(100),          -- 客户色号
    "customer_color_name" VARCHAR(200),        -- 客户色名
    -- 来样信息
    "sample_type" VARCHAR(50),                 -- 来样类型：fabric(布样)/yarn(纱样)/paper(纸板)/pantone(色卡)
    "fabric_spec" VARCHAR(500),                -- 坯布规格（纱支/成分/组织）
    "fabric_component" VARCHAR(200),           -- 纤维成分（棉/麻/黏胶/毛/丝/涤纶/锦纶等，决定染料类别）
    "sample_size" VARCHAR(50),                 -- 打样坯布大小（如 30x15cm）
    -- 对色光源（真实业务多光源）
    "light_source" VARCHAR(100) NOT NULL,      -- 对色光源：D65/TL84/U3000/CWF/A 等，多光源用逗号分隔
    "secondary_light_source" VARCHAR(100),     -- 副光源（检查跳灯现象）
    -- 色牢度要求（真实业务多项）
    "color_fastness_req" VARCHAR(500),         -- 色牢度要求 JSON：{soaping, rubbing, daylight, chlorine, dry_cleaning}
    -- 环保要求
    "eco_requirement" VARCHAR(200),            -- 环保标准：Oeko-Tex/GOTS/无
    -- 打样版数（真实业务 ABCD 四版）
    "sample_versions" INTEGER NOT NULL DEFAULT 4, -- 打样版数：默认 4（A/B/C/D）
    -- 染料类别（来样分析后确定）
    "dye_category" VARCHAR(50),                -- 染料类别：reactive(活性)/disperse(分散)/acid(酸性)/vat(还原)/sulfur(硫化)/direct(直接)
    -- 交期管理
    "required_date" DATE NOT NULL,             -- 客户要求交期
    "expected_days" INTEGER DEFAULT 3,         -- 预期打样天数（行业惯例：染色烧杯样3天/印花样10天/色织样10天）
    -- 状态机
    "status" VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- pending(待打样) → sampling(打样中) → submitted(已送客户) → approved(OK样确认) / rejected(重打) → completed(已建库)
    -- 客户确认信息
    "customer_approved_at" TIMESTAMPTZ,        -- 客户确认时间
    "customer_approval_comment" TEXT,          -- 客户确认意见
    "approved_sample_id" INTEGER,              -- 客户选中的 OK 样 ID（关联 lab_dip_sample）
    -- 关联生产
    "production_recipe_id" INTEGER,            -- 复样通过后升级的 dye_recipe ID（大货处方模板）
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT false,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：打样通知单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "idx_lab_dip_request_no" ON "lab_dip_request" ("request_no");
-- 状态+交期复合索引（跟单查询待打样/已送客户）
CREATE INDEX IF NOT EXISTS "idx_lab_dip_request_status_date" ON "lab_dip_request" ("status", "required_date");
-- 客户查询索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_request_customer" ON "lab_dip_request" ("customer_id");
-- 软删除过滤索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_request_not_deleted" ON "lab_dip_request" ("is_deleted") WHERE "is_deleted" = false;

-- ============================================================================
-- 2. 打样小样表（lab_dip_sample）
-- 业务来源：技术科打样员根据通知单打 ABCD 多版小样
-- 真实业务：每版样含处方/工艺参数/对色结果，客户从中选 1 版作为 OK 样
-- ============================================================================
CREATE TABLE IF NOT EXISTS "lab_dip_sample" (
    "id" SERIAL PRIMARY KEY,
    "request_id" INTEGER NOT NULL,             -- 关联打样通知单
    -- 版本标识（真实业务 ABCD 四版）
    "version_label" VARCHAR(10) NOT NULL,      -- 版本标识：A/B/C/D/E...
    "version_seq" INTEGER NOT NULL,            -- 版本序号：1/2/3/4...
    -- 处方信息（真实业务：染料+助剂+工艺参数）
    "recipe_no" VARCHAR(50),                   -- 配方编号（关联 dye_recipe 或独立小样配方）
    "dye_recipe_id" INTEGER,                   -- 关联 dye_recipe 表（如已建档）
    "formula" TEXT,                            -- 处方详情（染料组合+用量，文本描述）
    "formula_detail" JSONB,                    -- 处方明细 JSON：[{dye_name, amount, unit, percentage}]
    -- 工艺参数（真实业务关键参数）
    "temperature" DECIMAL(5,2),                -- 染色温度（℃）
    "time_minutes" INTEGER,                    -- 保温时间（分钟）
    "liquor_ratio" VARCHAR(20),                -- 浴比（如 1:5、1:8，小样标准 1:5）
    "ph_value" DECIMAL(5,2),                   -- pH 值
    "dyeing_method" VARCHAR(50),               -- 染色方法：dip(浸染)/pad(轧染)
    -- 成本核算
    "dye_cost" DECIMAL(10,4),                  -- 染料成本（元）
    "auxiliary_cost" DECIMAL(10,4),            -- 助剂成本（元）
    "total_cost" DECIMAL(10,4),                -- 总成本（元，为报价依据）
    -- 对色结果
    "color_difference_grade" INTEGER,          -- 色差等级（4-5 级为 OK，<4 级为重打）
    "color_difference_value" DECIMAL(5,2),     -- Delta E 色差值
    "matching_result" VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- pending(待对色) → matched(对色OK) → not_matched(不匹配) → selected(客户选中OK样)
    -- 审核信息
    "approved_by" INTEGER,                     -- 审核人（研发组长）
    "approved_at" TIMESTAMPTZ,                 -- 审核时间
    "approval_comment" TEXT,                   -- 审核意见
    -- 复样关联（OK 样升级为复样）
    "resample_status" VARCHAR(20) DEFAULT 'none', -- none(未复样) → resampling(复样中) → resampled(复样通过) → failed(复样失败)
    "resample_recipe_id" INTEGER,              -- 复样升级后的大货处方 ID（关联 dye_recipe）
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT false,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 通知单+版本联合唯一（同一通知单下版本号唯一）
CREATE UNIQUE INDEX IF NOT EXISTS "idx_lab_dip_sample_req_ver" ON "lab_dip_sample" ("request_id", "version_label");
-- 通知单外键索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_sample_request" ON "lab_dip_sample" ("request_id");
-- 配方编号索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_sample_recipe_no" ON "lab_dip_sample" ("recipe_no");
-- 对色结果索引（查询 OK 样）
CREATE INDEX IF NOT EXISTS "idx_lab_dip_sample_matching" ON "lab_dip_sample" ("matching_result");
-- 软删除过滤索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_sample_not_deleted" ON "lab_dip_sample" ("is_deleted") WHERE "is_deleted" = false;

-- ============================================================================
-- 3. 复样记录表（lab_dip_resample）
-- 业务来源：OK 样确认后，大货生产前必须复样
-- 真实业务：用车间半制品布+生产染化料模拟大生产，色差达 4-5 级方可投产
-- ============================================================================
CREATE TABLE IF NOT EXISTS "lab_dip_resample" (
    "id" SERIAL PRIMARY KEY,
    "request_id" INTEGER NOT NULL,             -- 关联打样通知单
    "source_sample_id" INTEGER NOT NULL,       -- 关联 OK 样（lab_dip_sample.id）
    "resample_no" VARCHAR(50) NOT NULL,        -- 复样单号：RS-YYYYMMDDHHMMSS-NNN
    -- 复样条件（真实业务：模拟大生产）
    "workshop_fabric_batch" VARCHAR(100),      -- 车间半制品布批号（不可用化验室存布）
    "dye_batch_no" VARCHAR(100),               -- 染料批号（与生产一致）
    "auxiliary_batch_no" VARCHAR(100),         -- 助剂批号（与生产一致）
    "production_plan_id" INTEGER,              -- 关联生产计划（按生产计划安排的纱种/浴比/染化料）
    -- 复样处方（基于 OK 样处方+加成系数调整）
    "adjusted_formula" TEXT,                   -- 调整后处方（加成/冲减）
    "adjustment_factor" DECIMAL(5,2),          -- 加成系数（小样→大货得色差异修正）
    "adjusted_temperature" DECIMAL(5,2),       -- 调整后温度
    "adjusted_time_minutes" INTEGER,           -- 调整后时间
    "adjusted_liquor_ratio" VARCHAR(20),       -- 调整后浴比（车间大货 1:8 以上）
    -- 复样结果
    "color_difference_grade" INTEGER,          -- 色差等级（4-5 级方可投产）
    "color_difference_value" DECIMAL(5,2),     -- Delta E 色差值
    "result" VARCHAR(20) NOT NULL DEFAULT 'pending',
    -- pending(待复样) → passed(复样通过) → failed(复样失败) → adjusted(需调整重试)
    -- 审核信息
    "reviewed_by" INTEGER,                     -- 审核人（研发组长，落实审核制度）
    "reviewed_at" TIMESTAMPTZ,
    "review_comment" TEXT,
    -- 升级大货处方
    "production_recipe_id" INTEGER,            -- 复样通过后升级的 dye_recipe ID（大货处方模板）
    -- 染色技术卡（研发输出物）
    "tech_card_no" VARCHAR(50),                -- 染色技术卡编号
    "tech_card_issued_by" INTEGER,             -- 开卡人（研发组长）
    "tech_card_issued_at" TIMESTAMPTZ,         -- 开卡时间
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT false,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 复样单号唯一
CREATE UNIQUE INDEX IF NOT EXISTS "idx_lab_dip_resample_no" ON "lab_dip_resample" ("resample_no");
-- 通知单外键索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_resample_request" ON "lab_dip_resample" ("request_id");
-- OK 样外键索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_resample_source" ON "lab_dip_resample" ("source_sample_id");
-- 复样结果索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_resample_result" ON "lab_dip_resample" ("result");
-- 技术卡编号索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_resample_tech_card" ON "lab_dip_resample" ("tech_card_no");
-- 软删除过滤索引
CREATE INDEX IF NOT EXISTS "idx_lab_dip_resample_not_deleted" ON "lab_dip_resample" ("is_deleted") WHERE "is_deleted" = false;

-- ============================================================================
-- 4. 外键约束（DO 块幂等）
-- ============================================================================
DO $$
BEGIN
    -- lab_dip_request.customer_id → customers.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_req_customer') THEN
        ALTER TABLE "lab_dip_request" ADD CONSTRAINT "fk_lab_dip_req_customer"
            FOREIGN KEY ("customer_id") REFERENCES "customers" ("id") ON DELETE SET NULL;
    END IF;
    -- lab_dip_request.approved_sample_id → lab_dip_sample.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_req_ok_sample') THEN
        ALTER TABLE "lab_dip_request" ADD CONSTRAINT "fk_lab_dip_req_ok_sample"
            FOREIGN KEY ("approved_sample_id") REFERENCES "lab_dip_sample" ("id") ON DELETE SET NULL;
    END IF;
    -- lab_dip_request.production_recipe_id → dye_recipe.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_req_recipe') THEN
        ALTER TABLE "lab_dip_request" ADD CONSTRAINT "fk_lab_dip_req_recipe"
            FOREIGN KEY ("production_recipe_id") REFERENCES "dye_recipe" ("id") ON DELETE SET NULL;
    END IF;
    -- lab_dip_sample.request_id → lab_dip_request.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_sample_req') THEN
        ALTER TABLE "lab_dip_sample" ADD CONSTRAINT "fk_lab_dip_sample_req"
            FOREIGN KEY ("request_id") REFERENCES "lab_dip_request" ("id") ON DELETE CASCADE;
    END IF;
    -- lab_dip_sample.dye_recipe_id → dye_recipe.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_sample_recipe') THEN
        ALTER TABLE "lab_dip_sample" ADD CONSTRAINT "fk_lab_dip_sample_recipe"
            FOREIGN KEY ("dye_recipe_id") REFERENCES "dye_recipe" ("id") ON DELETE SET NULL;
    END IF;
    -- lab_dip_resample.request_id → lab_dip_request.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_resample_req') THEN
        ALTER TABLE "lab_dip_resample" ADD CONSTRAINT "fk_lab_dip_resample_req"
            FOREIGN KEY ("request_id") REFERENCES "lab_dip_request" ("id") ON DELETE CASCADE;
    END IF;
    -- lab_dip_resample.source_sample_id → lab_dip_sample.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_resample_source') THEN
        ALTER TABLE "lab_dip_resample" ADD CONSTRAINT "fk_lab_dip_resample_source"
            FOREIGN KEY ("source_sample_id") REFERENCES "lab_dip_sample" ("id") ON DELETE RESTRICT;
    END IF;
    -- lab_dip_resample.production_recipe_id → dye_recipe.id
    IF NOT EXISTS (SELECT 1 FROM information_schema.table_constraints WHERE constraint_name = 'fk_lab_dip_resample_recipe') THEN
        ALTER TABLE "lab_dip_resample" ADD CONSTRAINT "fk_lab_dip_resample_recipe"
            FOREIGN KEY ("production_recipe_id") REFERENCES "dye_recipe" ("id") ON DELETE SET NULL;
    END IF;
END $$;
