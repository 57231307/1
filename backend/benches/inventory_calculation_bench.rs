//! 库存核算性能基准测试（V15 Batch 487 P0-T07）
//!
//! 基准对象：InventoryStockService::calculate_quantity_kg（双计量单位换算纯函数）
//! 业务场景：面料库存创建时，根据米数 + 克重 + 幅宽自动计算公斤数
//! 性能要求：单次计算 < 1ms（大数据量批量创建时避免阻塞）

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal::Decimal;
use std::str::FromStr;

use bingxi_backend::services::inventory_stock_service::InventoryStockService;

/// 基准：calculate_quantity_kg 克重和幅宽齐全走转换器
fn bench_calculate_quantity_kg_with_gram_weight(c: &mut Criterion) {
    let quantity_meters = Decimal::from_str("1000.5").unwrap();
    let gram_weight = Some(Decimal::from_str("250.0").unwrap());
    let width = Some(Decimal::from_str("1.5").unwrap());
    let fallback = Decimal::from_str("300.0").unwrap();

    c.bench_function("calculate_quantity_kg_克重幅宽齐全", |b| {
        b.iter(|| {
            black_box(InventoryStockService::calculate_quantity_kg(
                black_box(quantity_meters),
                black_box(gram_weight),
                black_box(width),
                black_box(fallback),
            ))
        })
    });
}

/// 基准：calculate_quantity_kg 克重或幅宽缺失走 fallback
fn bench_calculate_quantity_kg_fallback(c: &mut Criterion) {
    let quantity_meters = Decimal::from_str("1000.5").unwrap();
    let fallback = Decimal::from_str("300.0").unwrap();

    c.bench_function("calculate_quantity_kg_走fallback", |b| {
        b.iter(|| {
            black_box(InventoryStockService::calculate_quantity_kg(
                black_box(quantity_meters),
                black_box(None),
                black_box(None),
                black_box(fallback),
            ))
        })
    });
}

/// 基准：批量计算 1000 条库存记录的公斤数（模拟批量入库场景）
fn bench_batch_calculate_quantity_kg(c: &mut Criterion) {
    let records: Vec<(Decimal, Option<Decimal>, Option<Decimal>, Decimal)> = (0..1000)
        .map(|i| {
            let meters = Decimal::from_str(&format!("{}.5", i + 1)).unwrap();
            let gram_weight = Some(Decimal::from_str("250.0").unwrap());
            let width = Some(Decimal::from_str("1.5").unwrap());
            let fallback = Decimal::from_str("300.0").unwrap();
            (meters, gram_weight, width, fallback)
        })
        .collect();

    c.bench_function("批量计算1000条库存公斤数", |b| {
        b.iter(|| {
            let total: Decimal = black_box(records.iter())
                .map(|(m, g, w, f)| {
                    InventoryStockService::calculate_quantity_kg(*m, *g, *w, *f)
                })
                .sum();
            black_box(total)
        })
    });
}

criterion_group!(
    benches,
    bench_calculate_quantity_kg_with_gram_weight,
    bench_calculate_quantity_kg_fallback,
    bench_batch_calculate_quantity_kg,
);
criterion_main!(benches);
