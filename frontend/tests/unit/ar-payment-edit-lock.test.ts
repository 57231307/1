/**
 * AR 收款金额/收款日期编辑链路形状锁（前后端配对，源码文本级）。
 * 功能：把 backend/src/handlers/ar_payment_handler.rs 的 UpdateArPaymentRequest
 * 双层三态字段、backend/src/services/ar_ops/collection.rs 的 update_payment
 * 金额/日期应用段与同源函数引用、verification_ops/manual.rs 的分配求和收敛、
 * frontend/src/api/ar.ts 更新载荷声明、frontend/src/views/ar/tabs/PaymentTab.vue
 * 编辑对话框真实上送与 Decimal→字符串口径互锁；任一侧单边回退（字段被移出 DTO、
 * 更新链路不再应用该键、前端不再上送、出现第二套分配求和、字符串直调 toFixed）即判红。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内五份源码文本，不执行后端代码、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/** 读仓库内源码并统一行尾为 LF（Windows 检出 CRLF / CI 检出 LF，正则跨行判据对行尾敏感） */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const HANDLER_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/handlers/ar_payment_handler.rs')
);
const COLLECTION_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/ar_ops/collection.rs')
);
const MANUAL_SRC = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/ar_ops/verification_ops/manual.rs')
);
const AR_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/ar.ts'));
const PAYMENT_TAB_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/ar/tabs/PaymentTab.vue')
);

describe('后端 UpdateArPaymentRequest 金额/日期三态声明', () => {
  const dtoBlock = HANDLER_SRC.match(/pub struct UpdateArPaymentRequest \{[\s\S]*?\n\}/)?.[0] ?? '';

  it('DTO 承载 amount 与 payment_date 双层三态字段', () => {
    expect(dtoBlock).toContain('pub amount: Option<Option<rust_decimal::Decimal>>');
    expect(dtoBlock).toContain('pub payment_date: Option<Option<chrono::NaiveDate>>');
  });

  it('两个 NOT NULL 列字段挂 double_option 适配器与 skip_serializing_if', () => {
    const amountField = dtoBlock.match(/#\[serde\(([\s\S]*?)\)\]\s*pub amount: Option/)?.[1] ?? '';
    expect(amountField).toContain('deserialize_with = "double_option"');
    expect(amountField).toContain('skip_serializing_if = "Option::is_none"');
    const dateField =
      dtoBlock.match(/#\[serde\(([\s\S]*?)\)\]\s*pub payment_date: Option/)?.[1] ?? '';
    expect(dateField).toContain('deserialize_with = "double_option"');
    expect(dateField).toContain('skip_serializing_if = "Option::is_none"');
  });

  it('handler 更新入口真实调用 validate()（注解不是死码）', () => {
    const updateFn = HANDLER_SRC.match(/pub async fn update_payment\([\s\S]*?\n\}/)?.[0] ?? '';
    expect(updateFn).toContain('payload.validate()?');
  });
});

describe('后端 update_payment 金额/日期真实应用与同源判据', () => {
  const updateFn =
    COLLECTION_SRC.match(/pub async fn update_payment\([\s\S]*?\n {4}\}\n/)?.[0] ?? '';

  it('更新链路应用 amount 与 payment_date 两键', () => {
    expect(updateFn).toContain('payload.get("amount")');
    expect(updateFn).toContain('payload.get("payment_date")');
    expect(updateFn).toContain('active.collection_amount = Set(new_amount)');
    expect(updateFn).toContain('active.collection_date = Set(new_date)');
  });

  it('显式 null 对 NOT NULL 列业务拒绝（不落默认值）', () => {
    expect(updateFn).toContain('收款金额不能清空：该字段为必填项');
    expect(updateFn).toContain('收款日期不能清空：该字段为必填项');
  });

  it('金额校验与一致性门引用同源函数，不自写第二套', () => {
    expect(updateFn).toContain('Self::validate_payment_amount(customer_id, new_amount)?');
    expect(updateFn).toContain('Self::payment_verified_total(&txn, payment_id)');
    expect(updateFn).toContain('new_amount < verified_total');
    expect(updateFn).toContain('business_displayable');
  });

  it('日期变更复用创建路径同一关账入口 check_payment_period_locked', () => {
    expect(updateFn).toContain('self.check_payment_period_locked(&txn, new_date)');
  });

  it('分配账本求和只有一份实现（manual.rs 可用余额门收敛到 payment_verified_total）', () => {
    expect(COLLECTION_SRC).toContain('pub(super) async fn payment_verified_total(');
    expect(updateFn).toContain('Self::payment_verified_total(&txn, payment_id)');
    expect(MANUAL_SRC).toContain('Self::payment_verified_total(txn, payment_id)');
    expect(MANUAL_SRC).not.toContain('.map(|i| i.amount.abs())');
  });
});

describe('前端 ar.ts 更新载荷与 PaymentTab 编辑对话框真实上送', () => {
  const updateIface =
    AR_API_SRC.match(/export interface UpdateArPaymentRequest \{[\s\S]*?\n\}/)?.[0] ?? '';

  it('UpdateArPaymentRequest 声明 amount 与 payment_date', () => {
    expect(updateIface).toContain('amount?: number;');
    expect(updateIface).toContain('payment_date?: string;');
  });

  it('编辑分支载荷真实上送 amount/payment_date（后端更新链路有消费方）', () => {
    const editCall = PAYMENT_TAB_SRC.match(/if \(editId\.value\) \{[\s\S]*?\n {6}\}\);/)?.[0] ?? '';
    expect(editCall).toContain('amount: form.amount');
    expect(editCall).toContain('payment_date: form.payment_date');
    // 对话框未承载 bank_account ⇒ 载荷键不出现（三态缺席=保持），空串覆盖旧值的路径已移除
    expect(editCall).not.toContain('bank_account:');
  });

  it('NOT NULL 列控件为必填形态：日期不可 clearable、金额规则 type number + min', () => {
    expect(PAYMENT_TAB_SRC).toContain(':clearable="false"');
    expect(PAYMENT_TAB_SRC).toContain("type: 'number'");
    expect(PAYMENT_TAB_SRC).toContain('min: 0.01');
  });

  it('金额显示走 Decimal→字符串 typeof 口径，且不对字符串直调 toFixed', () => {
    const fmtFn = PAYMENT_TAB_SRC.match(/const formatMoney = \([\s\S]*?\n\};/)?.[0] ?? '';
    expect(fmtFn).toContain("typeof amount === 'number'");
    expect(fmtFn).toContain("typeof amount === 'string'");
    expect(fmtFn).toContain('Number.isFinite(n)');
    expect(PAYMENT_TAB_SRC).not.toMatch(/\.toFixed\(/);
  });
});
