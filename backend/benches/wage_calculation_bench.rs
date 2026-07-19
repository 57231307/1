//! 产量工资计算性能基准测试（V15 Batch 487 P0-T07）
//!
//! 基准对象：wage_service 的 compute_qualification_rate + calculate_wage_for_step（纯函数）
//! 业务场景：车间工序完工后，根据实际产量 + 合格产量 + 工时计算工资
//! 性能要求：单条工序工资计算 < 100μs（日产万条工序记录时批量计算不阻塞）

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_decimal::Decimal;

use bingxi_backend::models::process_wage_rate::Model as RateModel;
use bingxi_backend::services::wage_service::{
    calculate_wage_for_step, compute_qualification_rate,
};

/// 构造测试用 RateModel（计件工价，A 级全额 1.0）
fn sample_rate() -> RateModel {
    RateModel {
        id: 1,
        rate_no: "PWR-BENCH-001".to_string(),
        process_route_id: 1,
        route_code: "DYE".to_string(),
        route_name: "染色".to_string(),
        wage_type: "piece".to_string(),
        piece_price: Decimal::new(5, 0), // 5 元/kg
        time_price: Decimal::ZERO,
        grade_a_ratio: Decimal::new(10, 1), // 1.0
        grade_b_ratio: Decimal::new(8, 1),  // 0.8
        grade_c_ratio: Decimal::ZERO,
        effective_date: chrono::NaiveDate::from_ymd_opt(2026, 1, 1).unwrap(),
        expiry_date: None,
        workshop: None,
        status: "active".to_string(),
        remarks: None,
        is_deleted: false,
        created_by: None,
        created_at: chrono::Utc::now().into(),
        updated_at: chrono::Utc::now().into(),
    }
}

/// 基准：compute_qualification_rate 单次调用
fn bench_compute_qualification_rate(c: &mut Criterion) {
    let actual = Some(Decimal::new(100, 0));
    let qualified = Some(Decimal::new(95, 0));

    c.bench_function("compute_qualification_rate_单次调用", |b| {
        b.iter(|| {
            black_box(compute_qualification_rate(
                black_box(actual),
                black_box(qualified),
            ))
        })
    });
}

/// 基准：calculate_wage_for_step 单次调用（计件 A 级）
fn bench_calculate_wage_for_step(c: &mut Criterion) {
    let rate = sample_rate();
    let actual = Some(Decimal::new(100, 0));
    let qualified = Some(Decimal::new(95, 0));
    let minutes = Some(120);

    c.bench_function("calculate_wage_for_step_计件A级", |b| {
        b.iter(|| {
            black_box(calculate_wage_for_step(
                black_box(&rate),
                black_box(actual),
                black_box(qualified),
                black_box(minutes),
            ))
        })
    });
}

/// 基准：批量计算 1000 条工序记录的工资（模拟日结场景）
fn bench_batch_calculate_wage(c: &mut Criterion) {
    let rate = sample_rate();
    let records: Vec<(Option<Decimal>, Option<Decimal>, Option<i32>)> = (0..1000)
        .map(|i| {
            let actual = Some(Decimal::new(100 + i, 0));
            let qualified = Some(Decimal::new(95 + (i % 5), 0));
            let minutes = Some(120 + (i % 30) as i32);
            (actual, qualified, minutes)
        })
        .collect();

    c.bench_function("批量计算1000条工序工资", |b| {
        b.iter(|| {
            let total: Decimal = black_box(records.iter())
                .map(|(a, q, m)| {
                    let (_, _, _, _, wage) = calculate_wage_for_step(&rate, *a, *q, *m);
                    wage
                })
                .sum();
            black_box(total)
        })
    });
}

criterion_group!(
    benches,
    bench_compute_qualification_rate,
    bench_calculate_wage_for_step,
    bench_batch_calculate_wage,
);
criterion_main!(benches);
