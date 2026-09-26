use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 供应商商品/色号目录种子（sku-mapping 对照表功能依赖的父级数据）
        //
        // 背景：m0008 建好 supplier_products / supplier_product_colors 两表，但全仓既无
        // 建/查端点也无种子，导致 sku_mapping_service::validate_refs 永远拿不到真实
        // supplier_product_id，对照表建不出任何一条映射。此迁移补齐依赖数据链路。
        //
        // 仓库现状（证据）：migration 全量 grep 无任何 INSERT INTO "suppliers"，
        // demo 供应商仅由前端 e2e globalSetup 在运行时经 API 创建、且无稳定 supplier_code。
        // 因此本种子自建 2 个演示供应商（按 supplier_code 稳定标识、幂等），
        // 供 e2e/集成用例按 (supplier_code → product_code → color_no) 引用造映射。
        //
        // ⚠️ supplier::Model（models/supplier.rs）把大量列声明为非 Option（String/Decimal/
        // NaiveDate/bool），而 suppliers 表这些列由后续 ALTER 以 NULLABLE 加入。SeaORM 读
        // 取时若列为 NULL 会 decode 失败——validate_refs 经 supplier::Entity 读取供应商，
        // 故演示供应商必须把全部非 Option 列填齐，否则反而使对照表 create 报 500。
        //
        // 全部幂等：suppliers 依赖 supplier_code 的 UNIQUE；两目录表无 DB 唯一约束，
        // 故按业务逻辑键 (supplier_id, product_code) / (supplier_product_id, color_no)
        // 用 WHERE NOT EXISTS 去重。
        let sql = r#"
-- 1) 演示供应商（稳定标识：supplier_code；填齐 supplier::Model 全部非 Option 列）
INSERT INTO "suppliers"
    ("supplier_name","supplier_code","supplier_short_name","supplier_type","credit_code",
     "registered_address","legal_representative","registered_capital","establishment_date",
     "taxpayer_type","bank_name","bank_account","contact_phone","is_processor",
     "is_active","is_deleted","created_at","updated_at")
SELECT v.supplier_name, v.supplier_code, v.supplier_short_name, v.supplier_type, v.credit_code,
    v.registered_address, v.legal_representative, v.registered_capital, v.establishment_date,
    v.taxpayer_type, v.bank_name, v.bank_account, v.contact_phone, v.is_processor,
    true, false, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
FROM (VALUES
    ('面料供应演示甲','SUP-DEMO-FAB-01','演示甲','面料供应商','DEMO-CREDIT-00000001',
     '演示注册地址甲','张三',100.00,DATE '2020-01-01','一般纳税人','演示银行','6220000000000001','13800000001',false),
    ('面料供应演示乙','SUP-DEMO-FAB-02','演示乙','面料供应商','DEMO-CREDIT-00000002',
     '演示注册地址乙','李四',200.00,DATE '2020-01-01','一般纳税人','演示银行','6220000000000002','13800000002',false)
) AS v(supplier_name,supplier_code,supplier_short_name,supplier_type,credit_code,
       registered_address,legal_representative,registered_capital,establishment_date,
       taxpayer_type,bank_name,bank_account,contact_phone,is_processor)
WHERE NOT EXISTS (
    SELECT 1 FROM "suppliers" s WHERE s.supplier_code = v.supplier_code
);

-- 2) 供应商商品（2 个演示供应商各 2 条）
INSERT INTO "supplier_products"
    ("supplier_id","product_code","product_name","product_description","unit","is_enabled","created_at","updated_at")
SELECT s.id, v.product_code, v.product_name, v.product_description, v.unit, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
FROM (VALUES
    ('SUP-DEMO-FAB-01','FAB-P001','涤棉弹力面料','演示用涤棉弹力面料','米'),
    ('SUP-DEMO-FAB-01','FAB-P002','全棉斜纹面料','演示用全棉斜纹面料','米'),
    ('SUP-DEMO-FAB-02','FAB-P101','锦纶塔丝隆','演示用锦纶塔丝隆','米'),
    ('SUP-DEMO-FAB-02','FAB-P102','涤纶春亚纺','演示用涤纶春亚纺','米')
) AS v(supplier_code,product_code,product_name,product_description,unit)
JOIN "suppliers" s ON s.supplier_code = v.supplier_code
WHERE NOT EXISTS (
    SELECT 1 FROM "supplier_products" sp
    WHERE sp.supplier_id = s.id AND sp.product_code = v.product_code
);

-- 3) 供应商色号（一商品多色号：每商品 2~3 条）
INSERT INTO "supplier_product_colors"
    ("supplier_product_id","color_no","color_name","pantone_code","extra_cost","is_enabled","created_at","updated_at")
SELECT sp.id, v.color_no, v.color_name, v.pantone_code, v.extra_cost, true, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP
FROM (VALUES
    ('SUP-DEMO-FAB-01','FAB-P001','PC-A01','活性藏青','PANTONE 19-4052',0),
    ('SUP-DEMO-FAB-01','FAB-P001','PC-A02','活性酒红','PANTONE 19-1557',2.50),
    ('SUP-DEMO-FAB-01','FAB-P001','PC-A03','活性墨绿','PANTONE 19-5813',1.50),
    ('SUP-DEMO-FAB-01','FAB-P002','PC-B01','漂白','PANTONE 11-0601',0),
    ('SUP-DEMO-FAB-01','FAB-P002','PC-B02','本白','PANTONE 11-0507',0.50),
    ('SUP-DEMO-FAB-02','FAB-P101','PC-C01','银灰','PANTONE 14-4102',0),
    ('SUP-DEMO-FAB-02','FAB-P101','PC-C02','藏蓝','PANTONE 19-4023',1.00),
    ('SUP-DEMO-FAB-02','FAB-P102','PC-D01','军绿','PANTONE 17-0525',0),
    ('SUP-DEMO-FAB-02','FAB-P102','PC-D02','卡其','PANTONE 16-1120',0.75)
) AS v(supplier_code,product_code,color_no,color_name,pantone_code,extra_cost)
JOIN "suppliers" s ON s.supplier_code = v.supplier_code
JOIN "supplier_products" sp ON sp.supplier_id = s.id AND sp.product_code = v.product_code
WHERE NOT EXISTS (
    SELECT 1 FROM "supplier_product_colors" spc
    WHERE spc.supplier_product_id = sp.id AND spc.color_no = v.color_no
);
        "#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // 仅回滚本种子写入的演示数据（按稳定标识精删，不动用户自建数据）。
        let sql = r#"
DELETE FROM "supplier_product_colors"
WHERE supplier_product_id IN (
    SELECT sp.id FROM "supplier_products" sp
    JOIN "suppliers" s ON s.id = sp.supplier_id
    WHERE s.supplier_code IN ('SUP-DEMO-FAB-01','SUP-DEMO-FAB-02')
);
DELETE FROM "supplier_products"
WHERE supplier_id IN (
    SELECT id FROM "suppliers" WHERE supplier_code IN ('SUP-DEMO-FAB-01','SUP-DEMO-FAB-02')
);
DELETE FROM "suppliers" WHERE supplier_code IN ('SUP-DEMO-FAB-01','SUP-DEMO-FAB-02');
        "#;
        if !sql.trim().is_empty() {
            manager.get_connection().execute_unprepared(sql).await?;
        }
        Ok(())
    }
}
