-- 秉羲管理系统 - 初始数据库架构
-- 创建时间: 2026-03-23
-- 描述: 系统初始化数据库表结构

-- ============================================
-- 1. 系统管理模块 - 用户表
-- ============================================
CREATE TABLE "users" (
    "id" SERIAL PRIMARY KEY,
    "username" VARCHAR(100) NOT NULL UNIQUE,
    "password_hash" VARCHAR(255) NOT NULL,
    "email" VARCHAR(255),
    "phone" VARCHAR(50),
    "role_id" INTEGER,
    "department_id" INTEGER,
    "is_active" BOOLEAN DEFAULT true,
    "last_login_at" TIMESTAMP,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 用户索引
CREATE INDEX "idx_users_username" ON "users" ("username");
CREATE INDEX "idx_users_role_id" ON "users" ("role_id");
CREATE INDEX "idx_users_department_id" ON "users" ("department_id");

COMMENT ON TABLE "users" IS '用户表 - 存储系统用户信息';
COMMENT ON COLUMN "users"."username" IS '用户名 - 登录账号';
COMMENT ON COLUMN "users"."password_hash" IS '密码哈希 - 使用bcrypt加密';
COMMENT ON COLUMN "users"."email" IS '邮箱';
COMMENT ON COLUMN "users"."phone" IS '手机号';
COMMENT ON COLUMN "users"."role_id" IS '角色ID - 关联roles表';
COMMENT ON COLUMN "users"."department_id" IS '部门ID - 关联departments表';

-- ============================================
-- 2. 系统管理模块 - 角色表
-- ============================================
CREATE TABLE "roles" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "description" TEXT,
    "permissions" TEXT,
    "is_system" BOOLEAN DEFAULT false,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "roles" IS '角色表 - 存储系统角色信息';
COMMENT ON COLUMN "roles"."name" IS '角色名称';
COMMENT ON COLUMN "roles"."code" IS '角色代码 - 用于权限判断';
COMMENT ON COLUMN "roles"."permissions" IS '权限JSON - 存储权限列表';
COMMENT ON COLUMN "roles"."is_system" IS '是否系统角色 - 系统角色不可删除';

-- ============================================
-- 3. 系统管理模块 - 部门表
-- ============================================
CREATE TABLE "departments" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "parent_id" INTEGER,
    "manager_id" INTEGER,
    "description" TEXT,
    "sort_order" INTEGER DEFAULT 0,
    "is_active" BOOLEAN DEFAULT true,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 部门索引
CREATE INDEX "idx_departments_parent_id" ON "departments" ("parent_id");
CREATE INDEX "idx_departments_manager_id" ON "departments" ("manager_id");

COMMENT ON TABLE "departments" IS '部门表 - 存储组织架构信息';
COMMENT ON COLUMN "departments"."name" IS '部门名称';
COMMENT ON COLUMN "departments"."code" IS '部门代码';
COMMENT ON COLUMN "departments"."parent_id" IS '上级部门ID - 支持多级部门';
COMMENT ON COLUMN "departments"."manager_id" IS '部门经理ID - 关联users表';

-- ============================================
-- 4. 基础数据模块 - 产品表
-- ============================================
CREATE TABLE "products" (
    "id" SERIAL PRIMARY KEY,
    "product_no" VARCHAR(50) NOT NULL UNIQUE,
    "name" VARCHAR(200) NOT NULL,
    "category_id" INTEGER,
    "spec" VARCHAR(100),
    "unit" VARCHAR(20),
    "color" VARCHAR(50),
    "weight" DECIMAL(10, 2),
    "width" DECIMAL(10, 2),
    "length" DECIMAL(10, 2),
    "price" DECIMAL(12, 2) DEFAULT 0,
    "cost" DECIMAL(12, 2) DEFAULT 0,
    "stock" INTEGER DEFAULT 0,
    "warehouse_id" INTEGER,
    "supplier_id" INTEGER,
    "is_active" BOOLEAN DEFAULT true,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 产品索引
CREATE INDEX "idx_products_product_no" ON "products" ("product_no");
CREATE INDEX "idx_products_category_id" ON "products" ("category_id");
CREATE INDEX "idx_products_warehouse_id" ON "products" ("warehouse_id");
CREATE INDEX "idx_products_supplier_id" ON "products" ("supplier_id");

COMMENT ON TABLE "products" IS '产品表 - 存储面料产品信息';
COMMENT ON COLUMN "products"."product_no" IS '产品编号';
COMMENT ON COLUMN "products"."name" IS '产品名称';
COMMENT ON COLUMN "products"."category_id" IS '产品分类ID';
COMMENT ON COLUMN "products"."spec" IS '规格';
COMMENT ON COLUMN "products"."unit" IS '单位';
COMMENT ON COLUMN "products"."color" IS '颜色/色号';
COMMENT ON COLUMN "products"."weight" IS '克重 (g/m²)';
COMMENT ON COLUMN "products"."width" IS '幅宽 (cm)';

-- ============================================
-- 5. 基础数据模块 - 产品分类表
-- ============================================
CREATE TABLE "product_categories" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "parent_id" INTEGER,
    "description" TEXT,
    "sort_order" INTEGER DEFAULT 0,
    "is_active" BOOLEAN DEFAULT true,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "product_categories" IS '产品分类表';
COMMENT ON COLUMN "product_categories"."name" IS '分类名称';
COMMENT ON COLUMN "product_categories"."code" IS '分类代码';
COMMENT ON COLUMN "product_categories"."parent_id" IS '上级分类ID';

-- ============================================
-- 6. 基础数据模块 - 仓库表
-- ============================================
CREATE TABLE "warehouses" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "address" TEXT,
    "manager_id" INTEGER,
    "description" TEXT,
    "is_active" BOOLEAN DEFAULT true,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "warehouses" IS '仓库表';
COMMENT ON COLUMN "warehouses"."name" IS '仓库名称';
COMMENT ON COLUMN "warehouses"."code" IS '仓库代码';
COMMENT ON COLUMN "warehouses"."address" IS '仓库地址';
COMMENT ON COLUMN "warehouses"."manager_id" IS '仓库管理员ID';

-- ============================================
-- 7. 基础数据模块 - 供应商表
-- ============================================
CREATE TABLE "suppliers" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(200) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "contact" VARCHAR(100),
    "phone" VARCHAR(50),
    "email" VARCHAR(255),
    "address" TEXT,
    "description" TEXT,
    "is_active" BOOLEAN DEFAULT true,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "suppliers" IS '供应商表';
COMMENT ON COLUMN "suppliers"."name" IS '供应商名称';
COMMENT ON COLUMN "suppliers"."code" IS '供应商代码';
COMMENT ON COLUMN "suppliers"."contact" IS '联系人';
COMMENT ON COLUMN "suppliers"."phone" IS '联系电话';

-- ============================================
-- 8. 基础数据模块 - 客户表
-- ============================================
CREATE TABLE "customers" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(200) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "contact" VARCHAR(100),
    "phone" VARCHAR(50),
    "email" VARCHAR(255),
    "address" TEXT,
    "customer_type" VARCHAR(20),
    "credit_limit" DECIMAL(12, 2) DEFAULT 0,
    "description" TEXT,
    "is_active" BOOLEAN DEFAULT true,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "customers" IS '客户表';
COMMENT ON COLUMN "customers"."name" IS '客户名称';
COMMENT ON COLUMN "customers"."code" IS '客户代码';
COMMENT ON COLUMN "customers"."customer_type" IS '客户类型 - 批发/零售';
COMMENT ON COLUMN "customers"."credit_limit" IS '信用额度';

-- ============================================
-- 9. 库存模块 - 库存表
-- ============================================
CREATE TABLE "inventory_stocks" (
    "id" SERIAL PRIMARY KEY,
    "product_id" INTEGER NOT NULL,
    "warehouse_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50),
    "quantity" INTEGER NOT NULL DEFAULT 0,
    "reserved_quantity" INTEGER DEFAULT 0,
    "available_quantity" INTEGER GENERATED ALWAYS AS (quantity - reserved_quantity) STORED,
    "unit_cost" DECIMAL(12, 2) DEFAULT 0,
    "total_cost" DECIMAL(12, 2) DEFAULT 0,
    "location" VARCHAR(100),
    "shelf_life" DATE,
    "inbound_date" DATE,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 库存索引
CREATE INDEX "idx_inventory_product_id" ON "inventory_stocks" ("product_id");
CREATE INDEX "idx_inventory_warehouse_id" ON "inventory_stocks" ("warehouse_id");
CREATE INDEX "idx_inventory_batch_no" ON "inventory_stocks" ("batch_no");

COMMENT ON TABLE "inventory_stocks" IS '库存表';
COMMENT ON COLUMN "inventory_stocks"."product_id" IS '产品ID';
COMMENT ON COLUMN "inventory_stocks"."warehouse_id" IS '仓库ID';
COMMENT ON COLUMN "inventory_stocks"."batch_no" IS '批次号';
COMMENT ON COLUMN "inventory_stocks"."quantity" IS '库存数量';
COMMENT ON COLUMN "inventory_stocks"."reserved_quantity" IS '预留数量';
COMMENT ON COLUMN "inventory_stocks"."available_quantity" IS '可用数量';
COMMENT ON COLUMN "inventory_stocks"."unit_cost" IS '单位成本';

-- ============================================
-- 10. 销售模块 - 销售订单表
-- ============================================
CREATE TABLE "sales_orders" (
    "id" SERIAL PRIMARY KEY,
    "order_no" VARCHAR(50) NOT NULL UNIQUE,
    "customer_id" INTEGER NOT NULL,
    "order_date" DATE NOT NULL,
    "delivery_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
    "total_amount" DECIMAL(14, 2) DEFAULT 0,
    "discount_amount" DECIMAL(14, 2) DEFAULT 0,
    "final_amount" DECIMAL(14, 2) DEFAULT 0,
    "paid_amount" DECIMAL(14, 2) DEFAULT 0,
    "notes" TEXT,
    "created_by" INTEGER,
    "approved_by" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 销售订单索引
CREATE INDEX "idx_sales_orders_order_no" ON "sales_orders" ("order_no");
CREATE INDEX "idx_sales_orders_customer_id" ON "sales_orders" ("customer_id");
CREATE INDEX "idx_sales_orders_status" ON "sales_orders" ("status");
CREATE INDEX "idx_sales_orders_order_date" ON "sales_orders" ("order_date");

COMMENT ON TABLE "sales_orders" IS '销售订单表';
COMMENT ON COLUMN "sales_orders"."order_no" IS '订单编号';
COMMENT ON COLUMN "sales_orders"."customer_id" IS '客户ID';
COMMENT ON COLUMN "sales_orders"."order_date" IS '订单日期';
COMMENT ON COLUMN "sales_orders"."status" IS '订单状态 - draft/confirmed/produced/shipped/completed/cancelled';
COMMENT ON COLUMN "sales_orders"."total_amount" IS '订单总额';
COMMENT ON COLUMN "sales_orders"."final_amount" IS '最终金额 - 减去优惠';

-- ============================================
-- 11. 销售模块 - 销售订单明细表
-- ============================================
CREATE TABLE "sales_order_items" (
    "id" SERIAL PRIMARY KEY,
    "order_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "warehouse_id" INTEGER,
    "batch_no" VARCHAR(50),
    "quantity" INTEGER NOT NULL,
    "unit_price" DECIMAL(12, 2) NOT NULL,
    "discount_rate" DECIMAL(5, 2) DEFAULT 0,
    "tax_rate" DECIMAL(5, 2) DEFAULT 0,
    "subtotal" DECIMAL(14, 2) NOT NULL,
    "delivered_quantity" INTEGER DEFAULT 0,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 销售订单明细索引
CREATE INDEX "idx_sales_order_items_order_id" ON "sales_order_items" ("order_id");
CREATE INDEX "idx_sales_order_items_product_id" ON "sales_order_items" ("product_id");

COMMENT ON TABLE "sales_order_items" IS '销售订单明细表';
COMMENT ON COLUMN "sales_order_items"."order_id" IS '订单ID';
COMMENT ON COLUMN "sales_order_items"."product_id" IS '产品ID';
COMMENT ON COLUMN "sales_order_items"."quantity" IS '订单数量';
COMMENT ON COLUMN "sales_order_items"."delivered_quantity" IS '已发货数量';

-- ============================================
-- 12. 采购模块 - 采购订单表
-- ============================================
CREATE TABLE "purchase_orders" (
    "id" SERIAL PRIMARY KEY,
    "order_no" VARCHAR(50) NOT NULL UNIQUE,
    "supplier_id" INTEGER NOT NULL,
    "order_date" DATE NOT NULL,
    "delivery_date" DATE,
    "status" VARCHAR(20) NOT NULL DEFAULT 'draft',
    "total_amount" DECIMAL(14, 2) DEFAULT 0,
    "paid_amount" DECIMAL(14, 2) DEFAULT 0,
    "notes" TEXT,
    "created_by" INTEGER,
    "approved_by" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "purchase_orders" IS '采购订单表';
COMMENT ON COLUMN "purchase_orders"."order_no" IS '采购单号';
COMMENT ON COLUMN "purchase_orders"."supplier_id" IS '供应商ID';
COMMENT ON COLUMN "purchase_orders"."status" IS '状态 - draft/confirmed/partial_received/received/completed/cancelled';

-- ============================================
-- 13. 采购模块 - 采购订单明细表
-- ============================================
CREATE TABLE "purchase_order_items" (
    "id" SERIAL PRIMARY KEY,
    "order_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "quantity" INTEGER NOT NULL,
    "unit_price" DECIMAL(12, 2) NOT NULL,
    "subtotal" DECIMAL(14, 2) NOT NULL,
    "received_quantity" INTEGER DEFAULT 0,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "purchase_order_items" IS '采购订单明细表';
COMMENT ON COLUMN "purchase_order_items"."order_id" IS '采购单ID';
COMMENT ON COLUMN "purchase_order_items"."product_id" IS '产品ID';
COMMENT ON COLUMN "purchase_order_items"."received_quantity" IS '已收货数量';

-- ============================================
-- 14. 财务模块 - 收款单表
-- ============================================
CREATE TABLE "finance_payments" (
    "id" SERIAL PRIMARY KEY,
    "payment_no" VARCHAR(50) NOT NULL UNIQUE,
    "customer_id" INTEGER,
    "supplier_id" INTEGER,
    "payment_type" VARCHAR(20) NOT NULL,
    "amount" DECIMAL(14, 2) NOT NULL,
    "payment_date" DATE NOT NULL,
    "payment_method" VARCHAR(20),
    "bank_account" VARCHAR(50),
    "status" VARCHAR(20) DEFAULT 'pending',
    "notes" TEXT,
    "created_by" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- 收款单索引
CREATE INDEX "idx_finance_payments_payment_no" ON "finance_payments" ("payment_no");
CREATE INDEX "idx_finance_payments_customer_id" ON "finance_payments" ("customer_id");
CREATE INDEX "idx_finance_payments_supplier_id" ON "finance_payments" ("supplier_id");
CREATE INDEX "idx_finance_payments_payment_date" ON "finance_payments" ("payment_date");

COMMENT ON TABLE "finance_payments" IS '收付款表 - 记录收款和付款';
COMMENT ON COLUMN "finance_payments"."payment_type" IS '类型 - income/expense';
COMMENT ON COLUMN "finance_payments"."customer_id" IS '客户ID - 收款时使用';
COMMENT ON COLUMN "finance_payments"."supplier_id" IS '供应商ID - 付款时使用';
COMMENT ON COLUMN "finance_payments"."amount" IS '金额';

-- ============================================
-- 15. 库存调拨表
-- ============================================
CREATE TABLE "inventory_transfers" (
    "id" SERIAL PRIMARY KEY,
    "transfer_no" VARCHAR(50) NOT NULL UNIQUE,
    "from_warehouse_id" INTEGER NOT NULL,
    "to_warehouse_id" INTEGER NOT NULL,
    "transfer_date" DATE NOT NULL,
    "status" VARCHAR(20) DEFAULT 'draft',
    "total_quantity" INTEGER DEFAULT 0,
    "notes" TEXT,
    "created_by" INTEGER,
    "approved_by" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "inventory_transfers" IS '库存调拨表';
COMMENT ON COLUMN "inventory_transfers"."from_warehouse_id" IS '源仓库ID';
COMMENT ON COLUMN "inventory_transfers"."to_warehouse_id" IS '目标仓库ID';
COMMENT ON COLUMN "inventory_transfers"."status" IS '状态 - draft/approved/in_transit/completed';

-- ============================================
-- 16. 库存调拨明细表
-- ============================================
CREATE TABLE "inventory_transfer_items" (
    "id" SERIAL PRIMARY KEY,
    "transfer_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50),
    "quantity" INTEGER NOT NULL,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "inventory_transfer_items" IS '库存调拨明细表';

-- ============================================
-- 17. 库存盘点表
-- ============================================
CREATE TABLE "inventory_counts" (
    "id" SERIAL PRIMARY KEY,
    "count_no" VARCHAR(50) NOT NULL UNIQUE,
    "warehouse_id" INTEGER NOT NULL,
    "count_date" DATE NOT NULL,
    "status" VARCHAR(20) DEFAULT 'draft',
    "total_discrepancy" DECIMAL(14, 2) DEFAULT 0,
    "notes" TEXT,
    "created_by" INTEGER,
    "approved_by" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "inventory_counts" IS '库存盘点表';
COMMENT ON COLUMN "inventory_counts"."total_discrepancy" IS '总差异金额';

-- ============================================
-- 18. 库存盘点明细表
-- ============================================
CREATE TABLE "inventory_count_items" (
    "id" SERIAL PRIMARY KEY,
    "count_id" INTEGER NOT NULL,
    "product_id" INTEGER NOT NULL,
    "batch_no" VARCHAR(50),
    "system_quantity" INTEGER DEFAULT 0,
    "actual_quantity" INTEGER DEFAULT 0,
    "discrepancy_quantity" INTEGER DEFAULT 0,
    "unit_cost" DECIMAL(12, 2) DEFAULT 0,
    "discrepancy_amount" DECIMAL(14, 2) DEFAULT 0,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

COMMENT ON TABLE "inventory_count_items" IS '库存盘点明细表';

-- ============================================
-- 19. 初始化默认管理员账号
-- ============================================
INSERT INTO "roles" ("name", "code", "description", "permissions", "is_system") VALUES
('管理员', 'admin', '系统管理员', '["*"]', true),
('部门经理', 'manager', '部门经理', '["user:view", "product:*", "inventory:*", "sales:*"]', true),
('操作员', 'operator', '普通操作员', '["product:view", "inventory:view", "sales:view"]', true);

INSERT INTO "departments" ("name", "code", "parent_id", "description", "sort_order", "is_active") VALUES
('总经办', 'D001', NULL, '公司管理层', 1, true),
('财务部', 'D002', NULL, '财务管理', 2, true),
('销售部', 'D003', NULL, '销售业务', 3, true),
('仓储部', 'D004', NULL, '库存管理', 4, true),
('生产部', 'D005', NULL, '生产管理', 5, true);

-- 创建管理员用户 (密码: admin123)
-- 密码哈希使用bcrypt加密
INSERT INTO "users" ("username", "password_hash", "email", "role_id", "department_id", "is_active") VALUES
('admin', '$2b$12$LQv3c1yqBWVHxkd0LHAkCOYz6TtxMQJqhN8/LewY5GyYzpLhJ3K2i', 'admin@example.com', 1, 1, true);

-- ============================================
-- 20. 创建外键约束
-- ============================================
ALTER TABLE "users" ADD CONSTRAINT "fk_users_role" FOREIGN KEY ("role_id") REFERENCES "roles" ("id");
ALTER TABLE "users" ADD CONSTRAINT "fk_users_department" FOREIGN KEY ("department_id") REFERENCES "departments" ("id");
ALTER TABLE "departments" ADD CONSTRAINT "fk_departments_parent" FOREIGN KEY ("parent_id") REFERENCES "departments" ("id");
ALTER TABLE "products" ADD CONSTRAINT "fk_products_category" FOREIGN KEY ("category_id") REFERENCES "product_categories" ("id");
ALTER TABLE "products" ADD CONSTRAINT "fk_products_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id");
ALTER TABLE "products" ADD CONSTRAINT "fk_products_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id");
ALTER TABLE "inventory_stocks" ADD CONSTRAINT "fk_inventory_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id");
ALTER TABLE "inventory_stocks" ADD CONSTRAINT "fk_inventory_warehouse" FOREIGN KEY ("warehouse_id") REFERENCES "warehouses" ("id");
ALTER TABLE "sales_orders" ADD CONSTRAINT "fk_sales_orders_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id");
ALTER TABLE "sales_orders" ADD CONSTRAINT "fk_sales_orders_created_by" FOREIGN KEY ("created_by") REFERENCES "users" ("id");
ALTER TABLE "sales_order_items" ADD CONSTRAINT "fk_sales_order_items_order" FOREIGN KEY ("order_id") REFERENCES "sales_orders" ("id");
ALTER TABLE "sales_order_items" ADD CONSTRAINT "fk_sales_order_items_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id");
ALTER TABLE "purchase_orders" ADD CONSTRAINT "fk_purchase_orders_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id");
ALTER TABLE "purchase_order_items" ADD CONSTRAINT "fk_purchase_order_items_order" FOREIGN KEY ("order_id") REFERENCES "purchase_orders" ("id");
ALTER TABLE "purchase_order_items" ADD CONSTRAINT "fk_purchase_order_items_product" FOREIGN KEY ("product_id") REFERENCES "products" ("id");
ALTER TABLE "finance_payments" ADD CONSTRAINT "fk_finance_payments_customer" FOREIGN KEY ("customer_id") REFERENCES "customers" ("id");
ALTER TABLE "finance_payments" ADD CONSTRAINT "fk_finance_payments_supplier" FOREIGN KEY ("supplier_id") REFERENCES "suppliers" ("id");
ALTER TABLE "inventory_transfers" ADD CONSTRAINT "fk_inventory_transfers_from_warehouse" FOREIGN KEY ("from_warehouse_id") REFERENCES "warehouses" ("id");
ALTER TABLE "inventory_transfers" ADD CONSTRAINT "fk_inventory_transfers_to_warehouse" FOREIGN KEY ("to_warehouse_id") REFERENCES "warehouses" ("id");
ALTER TABLE "inventory_transfer_items" ADD CONSTRAINT "fk_inventory_transfer_items_transfer" FOREIGN KEY ("transfer_id") REFERENCES "inventory_transfers" ("id");
ALTER TABLE "inventory_count_items" ADD CONSTRAINT "fk_inventory_count_items_count" FOREIGN KEY ("count_id") REFERENCES "inventory_counts" ("id");

-- ============================================
-- 完成
-- ============================================


CREATE TABLE IF NOT EXISTS "audit_logs" (
    "id" SERIAL PRIMARY KEY,
    "table_name" VARCHAR(100) NOT NULL,
    "record_id" INTEGER NOT NULL,
    "action" VARCHAR(20) NOT NULL,
    "old_data" JSONB,
    "new_data" JSONB,
    "changed_fields" JSONB,
    "user_id" INTEGER,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);


CREATE TABLE IF NOT EXISTS "bpm_process_definition" (
    "id" SERIAL PRIMARY KEY,
    "name" VARCHAR(100) NOT NULL,
    "code" VARCHAR(50) NOT NULL UNIQUE,
    "description" TEXT,
    "category" VARCHAR(50),
    "version" VARCHAR(20),
    "config" JSONB,
    "status" VARCHAR(20) NOT NULL DEFAULT 'DRAFT',
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "bpm_process_instance" (
    "id" SERIAL PRIMARY KEY,
    "process_definition_id" INTEGER NOT NULL REFERENCES "bpm_process_definition"("id"),
    "instance_no" VARCHAR(50) NOT NULL UNIQUE,
    "business_type" VARCHAR(50),
    "business_id" INTEGER,
    "business_no" VARCHAR(50),
    "applicant_id" INTEGER NOT NULL,
    "status" VARCHAR(20) NOT NULL DEFAULT 'PROCESSING',
    "variables" JSONB,
    "start_time" TIMESTAMPTZ,
    "end_time" TIMESTAMPTZ,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS "bpm_task" (
    "id" SERIAL PRIMARY KEY,
    "process_instance_id" INTEGER NOT NULL REFERENCES "bpm_process_instance"("id"),
    "task_no" VARCHAR(50) NOT NULL UNIQUE,
    "node_id" VARCHAR(50) NOT NULL,
    "node_name" VARCHAR(100) NOT NULL,
    "name" VARCHAR(100) NOT NULL,
    "task_type" VARCHAR(50) NOT NULL,
    "assignee_id" INTEGER,
    "status" VARCHAR(20) NOT NULL DEFAULT 'PENDING',
    "comment" TEXT,
    "business_type" VARCHAR(50),
    "business_id" INTEGER,
    "is_deleted" BOOLEAN NOT NULL DEFAULT FALSE,
    "created_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "updated_at" TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    "completed_at" TIMESTAMPTZ
);
