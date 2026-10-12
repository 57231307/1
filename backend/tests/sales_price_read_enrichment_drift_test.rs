//! 销售价目列表「JOIN 富化读模型」防漂移回归（确定性源码文本扫描，零依赖）。
//!
//! 背景缺陷类（与 `purchase_read_enrichment_drift_test.rs` 同谱系）：前端列表列绑定
//! 后端 JOIN 派生名（product_name/product_code/customer_name/customer_code），但后端
//! 读模型缺该字段或 service 未真正 JOIN ⇒ 列恒空白，既不报错也不变红。
//! `sales_prices` 表本身**没有**任何名列（m0011 建表列清单可查）⇒ 名称只能取自
//! products/customers 两表的真实列；无真实来源的列禁止臆造（NULL 孤儿行如实输出）。
//!
//! 断言均为「包含」型（源码事实编译期可见），规避「不含某串」被注释自败的坑。

use std::fs;
use std::path::{Path, PathBuf};

fn src(p: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 销售价目列表：读模型 SalesPriceView 补 product_name/product_code/customer_name/customer_code；
/// service 经 models::sales_price::Relation::Product/Customer 两条 LEFT JOIN 取列并 into_model；
/// handler list_prices 出参换装该读模型。
#[test]
fn sales_price_read_enrichment_is_wired() {
    let svc = src("src/services/sales_price_service.rs");
    for field in [
        "pub product_name: Option<String>",
        "pub product_code: Option<String>",
        "pub customer_name: Option<String>",
        "pub customer_code: Option<String>",
    ] {
        assert!(
            svc.contains(field),
            "SalesPriceView 缺 JOIN 列 {}（名列必须 Option：两表无 FK，孤儿行如实为 NULL）",
            field
        );
    }
    // column_as 别名与 View 字段名逐字符一致，否则 FromQueryResult 静默不落值（列恒 NULL 假富化）
    assert!(
        svc.contains("column_as(product::Column::Name, \"product_name\")")
            && svc.contains("column_as(product::Column::Code, \"product_code\")"),
        "get_prices_list 必须 JOIN products 取 name/code 并以 product_name/product_code 别名落列"
    );
    assert!(
        svc.contains("column_as(customer::Column::CustomerName, \"customer_name\")")
            && svc.contains("column_as(customer::Column::CustomerCode, \"customer_code\")"),
        "get_prices_list 必须 JOIN customers 取 customer_name/customer_code 并以同名字段落列"
    );
    assert!(
        svc.contains("Relation::Product.def()") && svc.contains("Relation::Customer.def()"),
        "get_prices_list 必须 LEFT JOIN products/customers（经实体 Relation，不引入第二套手写 ON）"
    );
    assert!(
        svc.contains("into_model::<SalesPriceView>()"),
        "列表查询必须落到 SalesPriceView 读模型，而非退回整 Model（名列丢失即前端幽灵列回归）"
    );

    // JOIN 方向成立的前提：实体侧两条 belongs_to Relation 必须存在（签名与列对向正确）
    let model = src("src/models/sales_price.rs");
    assert!(
        model.contains("belongs_to = \"super::product::Entity\"")
            && model.contains("from = \"Column::ProductId\"")
            && model.contains("to = \"super::product::Column::Id\""),
        "sales_price::Relation 必须保留 Product belongs_to（ProductId -> products.id）"
    );
    assert!(
        model.contains("belongs_to = \"super::customer::Entity\"")
            && model.contains("from = \"Column::CustomerId\"")
            && model.contains("to = \"super::customer::Column::Id\""),
        "sales_price::Relation 必须保留 Customer belongs_to（CustomerId -> customers.id）"
    );

    // handler 出参换装：列表返回读模型；详情/创建/更新/历史仍返回整 Model（本批不扩语义）
    let handler = src("src/handlers/sales_price_handler.rs");
    assert!(
        handler.contains("Json<ApiResponse<Vec<SalesPriceView>>>"),
        "list_prices 出参必须是 Vec<SalesPriceView>（否则富化列在 handler 层丢失）"
    );

    // 反向判据：无真实来源不得臆造列。sales_prices 无供应商维度、users 侧未 JOIN 取操作人名，
    // 任何"图省事"往 View 塞这两个不存在的派生列的改动都会让 FromQueryResult 运行期报错或静默 NULL。
    assert!(
        !svc.contains("pub supplier_name"),
        "supplier_name 在 sales_price 侧无真实来源（销售价目表没有供应商列），禁止臆造该列"
    );
    assert!(
        !svc.contains("pub created_by_name") && !svc.contains("pub creator_name"),
        "created_by_name/creator_name 本批未接 users JOIN，禁止先塞占位列再查库（N+1/假富化两族缺陷成因）"
    );
}
