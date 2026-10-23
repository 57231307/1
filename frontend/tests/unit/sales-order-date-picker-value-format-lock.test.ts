/**
 * 销售订单表单日期选择器 value-format 静态锁（防回潮）。
 *
 * 功能：钉死 OrderFormDialog.vue 中两枚 el-date-picker（订单日期、要求交货日期）
 * 必须携带 `value-format="YYYY-MM-DD"` 属性，确保 Element Plus 输出的是 ISO 日期串
 * 而非 Date 对象，杜绝东八区 midnight 被 JSON 序列化为前一天 16:00Z 后触发
 * 后端交付日期日粒度比较的误拒。
 *
 * 同时钉死 OrderForm 接口中 order_date 与 required_date 的 TS 类型为 string，
 * 禁止回潮为 Date | string 的联合类型（值实际永远是字符串）。
 *
 * 判据：fs 读取源码文本，正则匹配属性存在性；不启动 Vue、不发网络请求。
 * 调用方：vitest（tests/unit，CI vitest job）。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、
 * CI 检出为 LF，正则夹具含行边界对形态敏感——不归一则本地 CRLF 下断言静默落空。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const dialogSrc = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/sales/OrderFormDialog.vue')
);

describe('OrderFormDialog el-date-picker value-format 静态锁', () => {
  it('订单日期 picker 必须携带 value-format="YYYY-MM-DD"', () => {
    // 定位 order_date 绑定的 el-date-picker 块
    const orderDatePickerRegex =
      /v-model="localData\.order_date"[\s\S]*?\/>/;
    const match = dialogSrc.match(orderDatePickerRegex);
    expect(match, '找不到 order_date el-date-picker 块').toBeTruthy();
    expect(
      match![0].includes('value-format="YYYY-MM-DD"'),
      'order_date picker 缺少 value-format="YYYY-MM-DD"'
    ).toBe(true);
  });

  it('要求交货日期 picker 必须携带 value-format="YYYY-MM-DD"', () => {
    const requiredDatePickerRegex =
      /v-model="localData\.required_date"[\s\S]*?\/>/;
    const match = dialogSrc.match(requiredDatePickerRegex);
    expect(match, '找不到 required_date el-date-picker 块').toBeTruthy();
    expect(
      match![0].includes('value-format="YYYY-MM-DD"'),
      'required_date picker 缺少 value-format="YYYY-MM-DD"'
    ).toBe(true);
  });

  it('OrderForm 接口中 order_date 类型必须为 string（非 Date | string 联合）', () => {
    // 匹配 interface 内 order_date 字段声明
    const orderDateTypeRegex = /order_date:\s*([^;]+);/;
    const match = dialogSrc.match(orderDateTypeRegex);
    expect(match, '找不到 OrderForm.order_date 类型声明').toBeTruthy();
    const type = match![1].trim();
    expect(type, 'order_date 类型应为 string').toBe('string');
  });

  it('OrderForm 接口中 required_date 类型必须为 string', () => {
    const requiredDateTypeRegex = /required_date:\s*([^;]+);/;
    const match = dialogSrc.match(requiredDateTypeRegex);
    expect(match, '找不到 OrderForm.required_date 类型声明').toBeTruthy();
    const type = match![1].trim();
    expect(type, 'required_date 类型应为 string').toBe('string');
  });
});
