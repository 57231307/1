//! 染整成本归集性能基准测试（V15 Batch 487 P0-T07）
//!
//! 基准对象：ProductionRecipeService::parse_liquor_ratio + calculate_amounts（纯函数）
//! 业务场景：大货处方审核时，根据浴比 + 布重 + 浓度计算各染料/助剂用量并归集成本
//! 性能要求：单次用量计算 < 1ms（一处方含 20-50 种物料时审核不阻塞）

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal::Decimal;

use bingxi_backend::models::production_recipe::RecipeMaterialItem;
use bingxi_backend::services::production_recipe_service::{
    CalculateAmountsRequest, ProductionRecipeService,
};

/// 构造测试用染料明细（5 种染料 + 5 种助剂）
fn sample_items() -> Vec<RecipeMaterialItem> {
    let dyes = ["活性红", "活性黄", "活性蓝", "分散红", "分散蓝"];
    let auxiliaries = ["匀染剂", "分散剂", "固色剂", "皂洗剂", "柔软剂"];

    let mut items: Vec<RecipeMaterialItem> = dyes
        .iter()
        .enumerate()
        .map(|(i, name)| RecipeMaterialItem {
            material_code: format!("D{:03}", i + 1),
            material_name: name.to_string(),
            concentration: Some(Decimal::new((i as i64 + 1) * 2, 0)), // 2%, 4%, 6%, 8%, 10%
            unit: "kg".to_string(),
            amount: Decimal::ZERO,
            category: "dye".to_string(),
        })
        .collect();

    items.extend(auxiliaries.iter().enumerate().map(|(i, name)| {
        RecipeMaterialItem {
            material_code: format!("A{:03}", i + 1),
            material_name: name.to_string(),
            concentration: None, // 助剂无浓度
            unit: "kg".to_string(),
            amount: Decimal::new((i as i64 + 1) * 5, 0), // 5kg, 10kg, 15kg, 20kg, 25kg
            category: "auxiliary".to_string(),
        }
    }));

    items
}

/// 基准：parse_liquor_ratio 浴比解析
fn bench_parse_liquor_ratio(c: &mut Criterion) {
    c.bench_function("parse_liquor_ratio_标准格式", |b| {
        b.iter(|| black_box(ProductionRecipeService::parse_liquor_ratio(black_box("1:8"))))
    });
}

/// 基准：calculate_amounts 单次用量计算（10 种物料）
fn bench_calculate_amounts(c: &mut Criterion) {
    let req = CalculateAmountsRequest {
        fabric_weight: Decimal::new(500, 0), // 500kg
        liquor_ratio: "1:8".to_string(),
        adjustment_factor: Some(Decimal::new(150, 2)), // 1.50 加成
        items: sample_items(),
    };

    c.bench_function("calculate_amounts_10种物料", |b| {
        b.iter(|| {
            black_box(ProductionRecipeService::calculate_amounts(black_box(req.clone())))
        })
    });
}

/// 基准：批量计算 100 个处方的用量（模拟审核批量场景）
fn bench_batch_calculate_amounts(c: &mut Criterion) {
    let base_items = sample_items();
    let requests: Vec<CalculateAmountsRequest> = (0..100)
        .map(|i| {
            let items: Vec<RecipeMaterialItem> = base_items
                .iter()
                .map(|item| RecipeMaterialItem {
                    material_code: item.material_code.clone(),
                    material_name: item.material_name.clone(),
                    concentration: item.concentration.map(|c| c + Decimal::new(i, 0)),
                    unit: item.unit.clone(),
                    amount: item.amount,
                    category: item.category.clone(),
                })
                .collect();
            CalculateAmountsRequest {
                fabric_weight: Decimal::new(500 + i, 0),
                liquor_ratio: "1:8".to_string(),
                adjustment_factor: Some(Decimal::new(150, 2)),
                items,
            }
        })
        .collect();

    c.bench_function("批量计算100个处方用量", |b| {
        b.iter(|| {
            let total_items: usize = black_box(requests.iter())
                .map(|req| {
                    ProductionRecipeService::calculate_amounts(req.clone())
                        .map(|items| items.len())
                        .unwrap_or(0)
                })
                .sum();
            black_box(total_items)
        })
    });
}

criterion_group!(
    benches,
    bench_parse_liquor_ratio,
    bench_calculate_amounts,
    bench_batch_calculate_amounts,
);
criterion_main!(benches);
