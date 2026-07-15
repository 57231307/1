-- ============================================================================
-- v14 批次 429：染化料主数据完善
-- 依据：面料行业真实业务调研文档 §4.3 染化料管理 + §11.4 染化料主数据管理 + §4.5 采购与坯布管理
-- 真实业务流程：
--   1. 染化料主数据：染料（分散/活性/还原/硫化/酸性/直接/阳离子）/ 助剂（前处理/染色/后整理/印花）/ 化工原料
--   2. GHS 危化品标注：依据 GHS 分类自动标注危化品属性（ghs_classification/un_number/hazard_class/hazard_pictogram/signal_word）
--   3. MSDS 安全数据表：支持扫描二维码直连 MSDS（msds_url/msds_version/msds_updated_at）
--   4. 化学品标识：CAS 号、分子式、分子量
--   5. 染料专属：dye_category（分散/活性/...）、color_index（C.I. 染料索引号）、fastness_light/fastness_washing
--   6. 助剂专属：auxiliary_category（前处理/染色/后整理/印花）、active_ingredient（有效成分）、concentration（浓度）
--   7. 保质期管理：shelf_life_days / storage_condition / storage_temperature
--   8. 安全库存：safety_stock / reorder_point / reorder_quantity
--   9. 包装规格：package_unit / package_capacity / packages_per_pallet
--  10. 供应商资质：supplier_ghs_qualification / supplier_cert_expires_at
--  11. 染化料批号管理：lot_no / supplier_lot_no / production_date / expiry_date / inspection_status
--  12. 领用退回流程：领用单 + 退回单 + 关联染色缸号 dye_batch_id
-- ============================================================================

-- ============================================================================
-- 1. 染化料分类表（chemical_category）
-- 真实业务：染料/助剂/化工原料的分类树，支持多级分类
-- ============================================================================
CREATE TABLE IF NOT EXISTS "chemical_category" (
    "id" SERIAL PRIMARY KEY,
    -- 分类编码
    "category_code" VARCHAR(64) NOT NULL,
    -- 分类名称
    "category_name" VARCHAR(128) NOT NULL,
    -- 父分类 ID（NULL 表示根分类）
    "parent_id" INTEGER,
    -- 分类类型：dye(染料) / auxiliary(助剂) / chemical(化工原料)
    "category_type" VARCHAR(32) NOT NULL,
    -- 描述
    "description" VARCHAR(512),
    -- 排序号
    "sort_order" INTEGER NOT NULL DEFAULT 0,
    -- 是否启用
    "is_active" BOOLEAN NOT NULL DEFAULT TRUE,
    -- 软删除
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：分类编码
CREATE UNIQUE INDEX IF NOT EXISTS "uk_chemical_category_code" ON "chemical_category" ("category_code") WHERE "is_deleted" = FALSE;
-- 索引：按父分类查询
CREATE INDEX IF NOT EXISTS "idx_chemical_category_parent" ON "chemical_category" ("parent_id") WHERE "is_deleted" = FALSE;
-- 索引：按分类类型查询
CREATE INDEX IF NOT EXISTS "idx_chemical_category_type" ON "chemical_category" ("category_type") WHERE "is_deleted" = FALSE;
-- 索引：按启用状态查询
CREATE INDEX IF NOT EXISTS "idx_chemical_category_active" ON "chemical_category" ("is_active") WHERE "is_deleted" = FALSE;
-- 索引：按排序查询
CREATE INDEX IF NOT EXISTS "idx_chemical_category_sort" ON "chemical_category" ("sort_order") WHERE "is_deleted" = FALSE;

-- 外键约束：父分类自引用
ALTER TABLE "chemical_category" ADD CONSTRAINT "fk_chemical_category_parent"
    FOREIGN KEY ("parent_id") REFERENCES "chemical_category" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "chemical_category" IS '染化料分类表（染料/助剂/化工原料的分类树）';
COMMENT ON COLUMN "chemical_category"."category_type" IS '分类类型 dye/auxiliary/chemical';
COMMENT ON COLUMN "chemical_category"."parent_id" IS '父分类 ID（NULL 表示根分类）';

-- ============================================================================
-- 2. 染化料主数据表（chemical_master）
-- 真实业务：染料/助剂/化工原料的统一主数据，含 GHS 危化品标注 + MSDS + 保质期 + 安全库存
-- ============================================================================
CREATE TABLE IF NOT EXISTS "chemical_master" (
    "id" SERIAL PRIMARY KEY,
    -- 染化料编码
    "chemical_code" VARCHAR(64) NOT NULL,
    -- 中文名
    "chemical_name" VARCHAR(128) NOT NULL,
    -- 英文名
    "chemical_name_en" VARCHAR(128),
    -- 染化料类型：dye(染料) / auxiliary(助剂) / chemical(化工原料)
    "chemical_type" VARCHAR(32) NOT NULL,
    -- 分类 ID（外键 → chemical_category）
    "category_id" INTEGER,
    -- 染料类别：dispersing(分散) / reactive(活性) / vat(还原) / sulfide(硫化) / acid(酸性) / direct(直接) / cationic(阳离子)，染料专属
    "dye_category" VARCHAR(32),
    -- C.I. 染料索引号，染料专属
    "color_index" VARCHAR(32),
    -- 助剂类别：pretreatment(前处理) / dyeing(染色) / finishing(后整理) / printing(印花)，助剂专属
    "auxiliary_category" VARCHAR(32),
    -- CAS 号
    "cas_number" VARCHAR(32),
    -- 分子式
    "molecular_formula" VARCHAR(64),
    -- 分子量
    "molecular_weight" DECIMAL(14,4),
    -- 规格
    "specification" VARCHAR(128),
    -- 计量单位：kg/L/桶/袋/瓶
    "unit" VARCHAR(16) NOT NULL DEFAULT 'kg',
    -- 标准价
    "standard_price" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 成本价
    "cost_price" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- GHS 分类编码
    "ghs_classification" VARCHAR(64),
    -- UN 编号
    "un_number" VARCHAR(16),
    -- 危险级别
    "hazard_class" VARCHAR(16),
    -- GHS 象形图
    "hazard_pictogram" VARCHAR(128),
    -- 信号词：danger(危险) / warning(警告)
    "signal_word" VARCHAR(16),
    -- MSDS 文件 URL
    "msds_url" VARCHAR(512),
    -- MSDS 版本
    "msds_version" VARCHAR(16),
    -- MSDS 更新时间
    "msds_updated_at" TIMESTAMPTZ,
    -- 保质期天数
    "shelf_life_days" INTEGER,
    -- 存储条件：防潮/防火/防爆/避光
    "storage_condition" VARCHAR(128),
    -- 存储温度
    "storage_temperature" VARCHAR(64),
    -- 安全库存阈值
    "safety_stock" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 再订货点
    "reorder_point" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 再订货量
    "reorder_quantity" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 包装单位：桶/袋/箱/瓶
    "package_unit" VARCHAR(16),
    -- 单包装容量
    "package_capacity" DECIMAL(14,4),
    -- 每托盘件数
    "packages_per_pallet" INTEGER,
    -- 供应商 ID（外键 → suppliers）
    "supplier_id" INTEGER,
    -- 供应商产品编码
    "supplier_product_code" VARCHAR(64),
    -- 日晒牢度，染料专属
    "fastness_light" VARCHAR(8),
    -- 水洗牢度，染料专属
    "fastness_washing" VARCHAR(8),
    -- 有效成分，助剂专属
    "active_ingredient" VARCHAR(128),
    -- 浓度，助剂专属
    "concentration" DECIMAL(8,4),
    -- 状态：active(启用) / inactive(停用) / discontinued(停产)
    "status" VARCHAR(16) NOT NULL DEFAULT 'active',
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：染化料编码
CREATE UNIQUE INDEX IF NOT EXISTS "uk_chemical_master_code" ON "chemical_master" ("chemical_code") WHERE "is_deleted" = FALSE;
-- 索引：按染化料名称查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_name" ON "chemical_master" ("chemical_name") WHERE "is_deleted" = FALSE;
-- 索引：按染化料类型查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_type" ON "chemical_master" ("chemical_type") WHERE "is_deleted" = FALSE;
-- 索引：按分类查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_category" ON "chemical_master" ("category_id") WHERE "is_deleted" = FALSE AND "category_id" IS NOT NULL;
-- 索引：按染料类别查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_dye_category" ON "chemical_master" ("dye_category") WHERE "is_deleted" = FALSE AND "dye_category" IS NOT NULL;
-- 索引：按助剂类别查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_aux_category" ON "chemical_master" ("auxiliary_category") WHERE "is_deleted" = FALSE AND "auxiliary_category" IS NOT NULL;
-- 索引：按 CAS 号查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_cas" ON "chemical_master" ("cas_number") WHERE "is_deleted" = FALSE AND "cas_number" IS NOT NULL;
-- 索引：按供应商查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_supplier" ON "chemical_master" ("supplier_id") WHERE "is_deleted" = FALSE AND "supplier_id" IS NOT NULL;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_chemical_master_status" ON "chemical_master" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按 GHS 分类查询（危化品筛查）
CREATE INDEX IF NOT EXISTS "idx_chemical_master_ghs" ON "chemical_master" ("ghs_classification") WHERE "is_deleted" = FALSE AND "ghs_classification" IS NOT NULL;
-- 索引：按 UN 编号查询（危化品运输分类）
CREATE INDEX IF NOT EXISTS "idx_chemical_master_un" ON "chemical_master" ("un_number") WHERE "is_deleted" = FALSE AND "un_number" IS NOT NULL;

-- 外键约束
ALTER TABLE "chemical_master" ADD CONSTRAINT "fk_chemical_master_category"
    FOREIGN KEY ("category_id") REFERENCES "chemical_category" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "chemical_master" ADD CONSTRAINT "fk_chemical_master_supplier"
    FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "chemical_master" IS '染化料主数据表（染料/助剂/化工原料，含 GHS 危化品标注 + MSDS）';
COMMENT ON COLUMN "chemical_master"."chemical_type" IS '染化料类型 dye/auxiliary/chemical';
COMMENT ON COLUMN "chemical_master"."dye_category" IS '染料类别 dispersing/reactive/vat/sulfide/acid/direct/cationic';
COMMENT ON COLUMN "chemical_master"."auxiliary_category" IS '助剂类别 pretreatment/dyeing/finishing/printing';
COMMENT ON COLUMN "chemical_master"."ghs_classification" IS 'GHS 分类编码（危化品自动标注）';
COMMENT ON COLUMN "chemical_master"."msds_url" IS 'MSDS 安全数据表 URL（二维码扫描直连）';
COMMENT ON COLUMN "chemical_master"."safety_stock" IS '安全库存阈值（低于此值触发预警）';
COMMENT ON COLUMN "chemical_master"."reorder_point" IS '再订货点（低于此值触发采购建议）';

-- ============================================================================
-- 3. 染化料批次表（chemical_lot）
-- 真实业务：每批染化料的批号/供应商批号/生产日期/失效日期/来料检验状态
-- ============================================================================
CREATE TABLE IF NOT EXISTS "chemical_lot" (
    "id" SERIAL PRIMARY KEY,
    -- 批次号
    "lot_no" VARCHAR(64) NOT NULL,
    -- 染化料 ID（外键 → chemical_master）
    "chemical_id" INTEGER NOT NULL,
    -- 供应商 ID（外键 → suppliers）
    "supplier_id" INTEGER,
    -- 供应商批号
    "supplier_lot_no" VARCHAR(64),
    -- 生产日期
    "production_date" DATE,
    -- 失效日期
    "expiry_date" DATE,
    -- 接收日期
    "received_date" DATE,
    -- 接收数量
    "quantity_received" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 可用数量
    "quantity_available" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 已预留数量
    "quantity_reserved" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 来料检验状态：pending(待检) / passed(合格) / failed(不合格) / quarantine(隔离)
    "inspection_status" VARCHAR(32) NOT NULL DEFAULT 'pending',
    -- 检验报告 URL
    "inspection_report_url" VARCHAR(512),
    -- 单位成本
    "unit_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 总成本（quantity_received × unit_cost）
    "total_cost" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 仓库 ID（外键 → warehouses）
    "warehouse_id" INTEGER,
    -- 存储区：hazard(危险品区) / safe(普通区)
    "storage_zone" VARCHAR(16),
    -- 批次状态：active(可用) / consumed(已耗尽) / expired(已过期) / scrapped(已报废)
    "status" VARCHAR(32) NOT NULL DEFAULT 'active',
    -- 备注
    "remarks" TEXT,
    -- 软删除与审计
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：批号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_chemical_lot_no" ON "chemical_lot" ("lot_no") WHERE "is_deleted" = FALSE;
-- 索引：按染化料查询
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_chemical" ON "chemical_lot" ("chemical_id") WHERE "is_deleted" = FALSE;
-- 索引：按供应商查询
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_supplier" ON "chemical_lot" ("supplier_id") WHERE "is_deleted" = FALSE AND "supplier_id" IS NOT NULL;
-- 索引：按供应商批号查询
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_supplier_lot" ON "chemical_lot" ("supplier_lot_no") WHERE "is_deleted" = FALSE AND "supplier_lot_no" IS NOT NULL;
-- 索引：按失效日期查询（效期预警）
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_expiry" ON "chemical_lot" ("expiry_date") WHERE "is_deleted" = FALSE AND "expiry_date" IS NOT NULL;
-- 索引：按生产日期查询
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_production" ON "chemical_lot" ("production_date") WHERE "is_deleted" = FALSE AND "production_date" IS NOT NULL;
-- 索引：按仓库查询
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_warehouse" ON "chemical_lot" ("warehouse_id") WHERE "is_deleted" = FALSE AND "warehouse_id" IS NOT NULL;
-- 索引：按来料检验状态查询
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_inspection" ON "chemical_lot" ("inspection_status") WHERE "is_deleted" = FALSE;
-- 索引：按存储区查询（危化品区筛查）
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_zone" ON "chemical_lot" ("storage_zone") WHERE "is_deleted" = FALSE AND "storage_zone" IS NOT NULL;
-- 索引：按批次状态查询
CREATE INDEX IF NOT EXISTS "idx_chemical_lot_status" ON "chemical_lot" ("status") WHERE "is_deleted" = FALSE;

-- 外键约束
ALTER TABLE "chemical_lot" ADD CONSTRAINT "fk_chemical_lot_chemical"
    FOREIGN KEY ("chemical_id") REFERENCES "chemical_master" ("id") ON UPDATE CASCADE ON DELETE RESTRICT;
ALTER TABLE "chemical_lot" ADD CONSTRAINT "fk_chemical_lot_supplier"
    FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "chemical_lot" ADD CONSTRAINT "fk_chemical_lot_warehouse"
    FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "chemical_lot" IS '染化料批次表（每批次的批号/效期/来料检验状态）';
COMMENT ON COLUMN "chemical_lot"."inspection_status" IS '来料检验状态 pending/passed/failed/quarantine';
COMMENT ON COLUMN "chemical_lot"."storage_zone" IS '存储区 hazard(危险品区) / safe(普通区)';
COMMENT ON COLUMN "chemical_lot"."status" IS '批次状态 active/consumed/expired/scrapped';

-- ============================================================================
-- 4. 染化料领用单表（chemical_requisition）
-- 真实业务：生产/化验室/研发领用染化料，关联染色缸号 dye_batch_id
-- 状态机：draft(草稿) → approved(已审批) → issued(已发料) → partial_returned(部分退回) → closed(已关闭)
--        任意非 closed 状态 → cancelled(已取消)
-- ============================================================================
CREATE TABLE IF NOT EXISTS "chemical_requisition" (
    "id" SERIAL PRIMARY KEY,
    -- 领用单号
    "requisition_no" VARCHAR(64) NOT NULL,
    -- 领用类型：production(生产领用) / lab(化验室领用) / rd(研发领用)
    "requisition_type" VARCHAR(32) NOT NULL,
    -- 部门 ID（外键 → departments）
    "department_id" INTEGER,
    -- 领用日期
    "requisition_date" DATE NOT NULL,
    -- 需求日期
    "required_date" DATE,
    -- 关联染色缸号 ID（外键 → dye_batch，可空）
    "dye_batch_id" INTEGER,
    -- 关联生产订单 ID（外键 → production_orders，可空）
    "production_order_id" INTEGER,
    -- 状态：draft/approved/issued/partial_returned/closed/cancelled
    "status" VARCHAR(32) NOT NULL DEFAULT 'draft',
    -- 总金额
    "total_amount" DECIMAL(14,4) NOT NULL DEFAULT 0,
    -- 备注
    "remarks" TEXT,
    -- 软删除
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    -- 审计
    "created_by" INTEGER,
    "approved_by" INTEGER,
    "issued_by" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 唯一约束：领用单号
CREATE UNIQUE INDEX IF NOT EXISTS "uk_chemical_requisition_no" ON "chemical_requisition" ("requisition_no") WHERE "is_deleted" = FALSE;
-- 索引：按领用类型查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_type" ON "chemical_requisition" ("requisition_type") WHERE "is_deleted" = FALSE;
-- 索引：按部门查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_department" ON "chemical_requisition" ("department_id") WHERE "is_deleted" = FALSE AND "department_id" IS NOT NULL;
-- 索引：按领用日期查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_date" ON "chemical_requisition" ("requisition_date") WHERE "is_deleted" = FALSE;
-- 索引：按需求日期查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_required" ON "chemical_requisition" ("required_date") WHERE "is_deleted" = FALSE AND "required_date" IS NOT NULL;
-- 索引：按染色缸号查询（关联染色批次）
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_dyebatch" ON "chemical_requisition" ("dye_batch_id") WHERE "is_deleted" = FALSE AND "dye_batch_id" IS NOT NULL;
-- 索引：按生产订单查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_order" ON "chemical_requisition" ("production_order_id") WHERE "is_deleted" = FALSE AND "production_order_id" IS NOT NULL;
-- 索引：按状态查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_status" ON "chemical_requisition" ("status") WHERE "is_deleted" = FALSE;
-- 索引：按审批人查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_approved_by" ON "chemical_requisition" ("approved_by") WHERE "is_deleted" = FALSE AND "approved_by" IS NOT NULL;
-- 索引：按发料人查询
CREATE INDEX IF NOT EXISTS "idx_chemical_requisition_issued_by" ON "chemical_requisition" ("issued_by") WHERE "is_deleted" = FALSE AND "issued_by" IS NOT NULL;

-- 外键约束
ALTER TABLE "chemical_requisition" ADD CONSTRAINT "fk_chemical_requisition_department"
    FOREIGN KEY ("department_id") REFERENCES "departments" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "chemical_requisition" ADD CONSTRAINT "fk_chemical_requisition_dyebatch"
    FOREIGN KEY ("dye_batch_id") REFERENCES "dye_batch" ("id") ON UPDATE CASCADE ON DELETE SET NULL;
ALTER TABLE "chemical_requisition" ADD CONSTRAINT "fk_chemical_requisition_order"
    FOREIGN KEY ("production_order_id") REFERENCES "production_orders" ("id") ON UPDATE CASCADE ON DELETE SET NULL;

COMMENT ON TABLE "chemical_requisition" IS '染化料领用单表（生产/化验室/研发领用，关联染色缸号）';
COMMENT ON COLUMN "chemical_requisition"."requisition_type" IS '领用类型 production/lab/rd';
COMMENT ON COLUMN "chemical_requisition"."dye_batch_id" IS '关联染色缸号 ID（生产领用时关联染色批次）';
COMMENT ON COLUMN "chemical_requisition"."status" IS '状态 draft/approved/issued/partial_returned/closed/cancelled';
