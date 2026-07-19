//! 凭证生成性能基准测试（V15 Batch 487 P0-T07）
//!
//! 基准对象：VoucherService::available_voucher_types（凭证类型枚举纯函数）
//! 业务场景：前端获取可用凭证类型列表（频繁调用的元数据接口）
//! 性能要求：单次调用 < 100μs（元数据接口应极快响应）

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use bingxi_backend::services::voucher_service::VoucherService;

/// 基准：available_voucher_types 单次调用
fn bench_available_voucher_types(c: &mut Criterion) {
    c.bench_function("available_voucher_types_单次调用", |b| {
        b.iter(|| {
            black_box(VoucherService::available_voucher_types())
        })
    });
}

/// 基准：available_voucher_types 批量调用 1000 次（模拟高并发元数据查询）
fn bench_available_voucher_types_batch(c: &mut Criterion) {
    c.bench_function("available_voucher_types_批量1000次", |b| {
        b.iter(|| {
            let count = (0..1000).fold(0usize, |acc, _| {
                let types = VoucherService::available_voucher_types();
                acc + types.len()
            });
            black_box(count)
        })
    });
}

criterion_group!(
    benches,
    bench_available_voucher_types,
    bench_available_voucher_types_batch,
);
criterion_main!(benches);
