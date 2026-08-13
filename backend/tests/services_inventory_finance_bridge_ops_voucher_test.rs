use bingxi_backend::services::inventory_finance_bridge_ops::voucher::*;
use rust_decimal::Decimal;

fn d(val: i64, scale: u32) -> Decimal {
    Decimal::new(val, scale)
}

#[test]
fn 首次入库_旧库存为零_新成本等于采购单价() {
    // before_qty=0, old_cost=0, received_qty=100, received_price=10.5
    let new_cost = compute_moving_average_cost(d(0, 0), d(0, 0), d(100, 0), d(105, 1));
    assert_eq!(new_cost, d(105, 1)); // 10.5
}

#[test]
fn 二次入库_加权平均计算正确() {
    // 旧库存 100 米 @ 10 元 = 1000 元
    // 本次入库 50 米 @ 13 元 = 650 元
    // 新成本 = (1000 + 650) / 150 = 11.0
    let new_cost = compute_moving_average_cost(d(100, 0), d(10, 0), d(50, 0), d(13, 0));
    assert_eq!(new_cost, d(110, 1)); // 11.0
}

#[test]
fn 三次入库_持续加权平均() {
    // 第一次：100@10 → 10.0
    let cost1 = compute_moving_average_cost(d(0, 0), d(0, 0), d(100, 0), d(10, 0));
    assert_eq!(cost1, d(10, 0));
    // 第二次：旧 100@10，本次 50@13 → 11.0
    let cost2 = compute_moving_average_cost(d(100, 0), cost1, d(50, 0), d(13, 0));
    assert_eq!(cost2, d(110, 1));
    // 第三次：旧 150@11，本次 200@14.5 → (1650+2900)/350 = 13.0
    let cost3 = compute_moving_average_cost(d(150, 0), cost2, d(200, 0), d(145, 1));
    assert_eq!(cost3, d(130, 1)); // 13.0
}

#[test]
fn 入库数量为零_返回旧成本() {
    // received_qty=0 → new_cost = old_cost_price（不变）
    let new_cost = compute_moving_average_cost(d(100, 0), d(10, 0), d(0, 0), d(15, 0));
    assert_eq!(new_cost, d(10, 0));
}

#[test]
fn 总库存为零_返回旧成本防除零() {
    // before_qty=0, received_qty=0 → total_qty=0，返回 old_cost_price
    let new_cost = compute_moving_average_cost(d(0, 0), d(10, 0), d(0, 0), d(15, 0));
    assert_eq!(new_cost, d(10, 0));
}

#[test]
fn 旧成本为零_首次入库退化为采购单价() {
    // before_qty=50（异常场景：库存存在但成本未维护）, old_cost=0, received_qty=50, received_price=12
    // new_cost = (0 + 600) / 100 = 6.0
    let new_cost = compute_moving_average_cost(d(50, 0), d(0, 0), d(50, 0), d(12, 0));
    assert_eq!(new_cost, d(60, 1)); // 6.0
}

#[test]
fn 精度保留四位小数() {
    // 100@3.3333 + 100@6.6667 → (333.33 + 666.67) / 200 = 5.0
    let new_cost = compute_moving_average_cost(d(100, 0), d(33333, 4), d(100, 0), d(66667, 4));
    assert_eq!(new_cost, d(50000, 4)); // 5.0000
}

#[test]
fn 采购单价为零不影响旧成本() {
    // received_unit_price=0（异常采购），新成本应被稀释但不报错
    // (100*10 + 50*0) / 150 = 6.6667
    let new_cost = compute_moving_average_cost(d(100, 0), d(10, 0), d(50, 0), d(0, 0));
    assert_eq!(new_cost, d(66667, 4)); // 6.6667
}
