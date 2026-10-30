/**
 * 面料每公斤出米数前端算式与后端 dual_unit_converter.rs 同源静态锁。
 *
 * 背景口径：产品表单在用户改动未落库的幅宽或克重时，必须即时带出只读「每公斤出米数」，
 * 这一即时值只能在前端算（落库后的读值才由后端随出参下发 meters_per_kg）。前端算式一旦与
 * 后端 DualUnitConverter::meters_per_kg 漂移，就会出现「表单显示的出米数」与「列表/详情出参
 * 的出米数」两套答案。此锁用源码文本比对钉住两侧同源，任一侧改公式即红。
 *
 * 锁定的契约面：
 * 1. 算式常量同源：公斤→克(1000)、米→厘米(100)、放大取整倍率(10) 三值在前后端一致；
 * 2. 取整口径同源：后端用 (meters × 10).floor() ÷ 10 向下截断一位小数，前端用
 *    Math.floor(meters × 10) ÷ 10，都是 floor 而非 round（四舍五入会在 .5 边界与后端错开）；
 * 3. 缺值/非正的返回形态同源：后端缺列或值 ≤0 返回 None（出参 null），前端返回 null，不返回 0；
 * 4. 前端 computeMetersPerKg 对给定克重与幅宽的实际取值，等于按算式手算的向下取整一位小数。
 *
 * 判据：读源码文本比对 + 纯函数取值断言，不启动 Vue、不发网络请求、不连库。
 * 检测力自证：把前端 FLOOR_SCALE 改成别的值、或把 Math.floor 改成 Math.round、
 * 或把 GRAM_PER_KG 改掉，第 2/4 组断言立即变红；把后端 meters_per_kg 的 floor 改成 round_dp(1)
 * 或改常量，第 1/2 组比对立即变红。
 *
 * 调用方：vitest（tests/unit，CI vitest job）。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import { computeMetersPerKg } from '@/utils/fabric-yardage';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 归一只统一输入形态，不改变任何判据语义。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const backendSrc = readRepoSource(path.join(REPO_ROOT, 'backend/src/utils/dual_unit_converter.rs'));
const frontendSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/utils/fabric-yardage.ts'));

/** 截出后端 meters_per_kg 方法体，避免与同文件其它换算方法（kg_to_meters 的 round_dp）误比 */
function backendMetersPerKgBody(src: string): string {
  const start = src.indexOf('pub fn meters_per_kg');
  expect(start, '后端应存在 meters_per_kg 方法（前端只读值的算式源）').toBeGreaterThan(-1);
  // 取到下一个 pub fn 或文件结束之间即为该方法体
  const rest = src.slice(start);
  const nextFn = rest.slice(1).search(/\n\s*pub fn /);
  return nextFn === -1 ? rest : rest.slice(0, nextFn + 1);
}

describe('面料每公斤出米数前后端算式同源锁', () => {
  it('后端 meters_per_kg 用 floor 向下截断一位小数，不是 round', () => {
    const body = backendMetersPerKgBody(backendSrc);
    expect(
      /\(meters \* Decimal::from\(10\)\)\.floor\(\) \/ Decimal::from\(10\)/.test(body),
      '后端出米数必须是「放大 10 倍取 floor 再缩回」的向下截断一位小数'
    ).toBe(true);
    expect(
      body.includes('round_dp'),
      '后端出米数不得用 round_dp（四舍五入会在 .5 边界与前端 floor 错开）'
    ).toBe(false);
  });

  it('前后端算式常量同源：1000 克/公斤、100 厘米/米、放大倍率 10', () => {
    const body = backendMetersPerKgBody(backendSrc);
    // 后端：Decimal::from(1000) 为公斤→克，Decimal::from(100) 为米→厘米，Decimal::from(10) 为取整倍率
    expect(body.includes('Decimal::from(1000)'), '后端算式应含公斤→克的 1000').toBe(true);
    expect(body.includes('Decimal::from(100)'), '后端算式应含米→厘米的 100').toBe(true);
    // 前端：三个同名常量必须取同样的值
    expect(frontendSrc).toMatch(/const GRAM_PER_KG = 1000;/);
    expect(frontendSrc).toMatch(/const CM_PER_M = 100;/);
    expect(frontendSrc).toMatch(/const FLOOR_SCALE = 10;/);
  });

  it('前端用 Math.floor 截断（与后端 floor 同口径），且除以 FLOOR_SCALE 缩回', () => {
    expect(
      /Math\.floor\(meters \* FLOOR_SCALE\) \/ FLOOR_SCALE/.test(frontendSrc),
      '前端必须是 Math.floor(meters × 10) ÷ 10，与后端 (meters × 10).floor() ÷ 10 同形'
    ).toBe(true);
    expect(
      /Math\.round/.test(frontendSrc),
      '前端不得用 Math.round（会把 .x5 进位，与后端 floor 分叉）'
    ).toBe(false);
  });

  it('前端按算式对具体克重幅宽取到向下截断的一位小数值', () => {
    // 手算期望，不从被测实现反推：
    // 1000 ÷ (200 × 1.5) = 3.3333… → 向下截断一位小数 = 3.3
    expect(computeMetersPerKg(200, 150)).toBe(3.3);
    // 1000 ÷ (170 × 1.8) = 3.2679… → floor 一位小数 = 3.2（round 会变 3.3，用于分辨口径）
    expect(computeMetersPerKg(170, 180)).toBe(3.2);
  });

  it('缺列或非正值前端返回 null（与后端返回 None、出参 null 同源，不返回 0）', () => {
    expect(computeMetersPerKg(null, 150)).toBeNull();
    expect(computeMetersPerKg(200, null)).toBeNull();
    expect(computeMetersPerKg(0, 150)).toBeNull();
    expect(computeMetersPerKg(200, -1)).toBeNull();
  });
});
