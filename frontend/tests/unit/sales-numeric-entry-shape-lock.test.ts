/**
 * 销售族手写数值录入形状锁（销售合同 发货 预留 三对话框 五处录入点位）。
 *
 * 钉住的事实：
 * 1) 四个业务文件与本域 e2e 随动文件全文零步进控件名出现（与 check-no-stepper-control
 *    的零出现口径同源 但按文件点名 防回归时只看见全局红不知道是谁破的）。
 * 2) 未填态语义不变：合同明细允差与预留数量是可选模型 清空必回 undefined 不塌成 0 假量
 *    允差提交时该键缺省 后端走默认允差解析 预留数量空由提交校验显式点名。
 * 3) 发货本次发货数量是受控单向绑定 :model-value 必须与 @update:model-value 成对
 *    缺一条输入框就打不了字（本项目真实踩过）动态 max 钳制与 0 不发口径一起钉住。
 * 4) 单源件机制侧：NumericInput 对空文本按 optional 与 null 模型分流回写 undefined 或 null
 *    绑定器对可选模型空文本回 undefined 必填 number 模型回下界 不塞 NaN 不塌空串。
 * 5) 精度与边界承接：允差 min 0 max 100 precision 2 预留 min 0.01 precision 2 与换控件前一致。
 *
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内源码文本，不执行组件、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');

/** 步进控件名字面量 分段拼接构造 本文件自身不携带该字面量 */
const STEPPER_TOKEN = ['el-input-', 'number'].join('');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、CI 检出为 LF，
 * 本锁的逐行判定与标签块正则对行尾形态敏感，不归一会让判据在不同环境给出不同结论。
 */
function readRepoSource(relFromFrontend: string): string {
  return readFileSync(path.join(FRONTEND_ROOT, relFromFrontend), 'utf8').replace(/\r?\n/g, '\n');
}

const SC_FORM_SRC = readRepoSource('src/views/sales-contract/components/SalesContractForm.vue');
const USE_SC_SRC = readRepoSource('src/views/sales-contract/composables/useSc.ts');
const DELIVERY_SRC = readRepoSource('src/views/sales/DeliveryDialog.vue');
const RESERVATION_SRC = readRepoSource('src/views/sales/ReservationCreateDialog.vue');
const SHIP_E2E_SRC = readRepoSource('e2e/sales/04-ship.spec.ts');
const NUMERIC_INPUT_SRC = readRepoSource('src/components/NumericInput.vue');
const NUMBER_INPUT_UTILS_SRC = readRepoSource('src/utils/number-input.ts');

/** 取模板区内某个组件的完整开标签块（含全部属性行）供成对绑定与边界承接判据使用 */
function tagBlock(src: string, tagName: string): string {
  const matches = src.match(new RegExp('<' + tagName + '[\\s\\S]*?/>', 'g')) || [];
  return matches.join('\n');
}

describe('销售族数值录入点位 零步进与未填态形状锁', () => {
  it('五个文件全文零步进控件名出现（含注释与模板）', () => {
    for (const [name, src] of [
      ['SalesContractForm.vue', SC_FORM_SRC],
      ['useSc.ts', USE_SC_SRC],
      ['DeliveryDialog.vue', DELIVERY_SRC],
      ['ReservationCreateDialog.vue', RESERVATION_SRC],
      ['04-ship.spec.ts', SHIP_E2E_SRC],
    ] as const) {
      const hits = src.split('\n').filter(line => line.includes(STEPPER_TOKEN));
      expect(hits, `${name} 残留步进控件名 ${JSON.stringify(hits)}`).toEqual([]);
    }
  });

  it('销售合同表单四处数值录入全部走单源件 允差走 optional 未填态', () => {
    expect(
      (SC_FORM_SRC.match(/<NumericInput/g) || []).length,
      '四处录入都必须是 NumericInput'
    ).toBe(4);
    expect(SC_FORM_SRC).toContain("import NumericInput from '@/components/NumericInput.vue'");
    // 允差是三态字段 只有它走 optional 其余必填模型不收 optional
    expect((SC_FORM_SRC.match(/:optional="true"/g) || []).length).toBe(1);
    expect(SC_FORM_SRC).toMatch(
      /v-model="row\.quantity_tolerance_pct"[\s\S]{0,200}:optional="true"/
    );
    // 精度与边界原样承接：总额与明细数量单价都两位小数 允差 0 到 100
    expect(SC_FORM_SRC).toMatch(/total_amount"[\s\S]{0,120}:precision="2"/);
    expect(SC_FORM_SRC).toMatch(/row\.quantity_tolerance_pct"[\s\S]{0,200}:max="100"/);
  });

  it('允差提交消费面原样透传 undefined 缺键语义', () => {
    // 明细行载荷组装：允差直接取表单字段值 未填时保持 undefined 序列化缺省键
    expect(USE_SC_SRC).toMatch(/quantity_tolerance_pct: i\.quantity_tolerance_pct,/);
    // 回显侧出参字符串或 null 归一为 number 或 undefined 三态不塌 0
    expect(USE_SC_SRC).toMatch(
      /quantity_tolerance_pct:[\s\S]{0,120}it\.quantity_tolerance_pct != null \? Number\(it\.quantity_tolerance_pct\) : undefined/
    );
  });

  it('预留数量走单源件 optional 未填由提交校验显式点名', () => {
    expect((RESERVATION_SRC.match(/<NumericInput/g) || []).length).toBe(1);
    expect(RESERVATION_SRC).toContain(':optional="true"');
    expect(RESERVATION_SRC).toContain('import NumericInput from');
    // 边界承接：下界 0.01 与两位小数与换控件前一致 上限沿用动态表达式
    expect(RESERVATION_SRC).toMatch(/:min="0\.01"/);
    expect(RESERVATION_SRC).toMatch(/:precision="2"/);
    expect(RESERVATION_SRC).toMatch(/:max="selectedMax \?\? undefined"/);
    // 未填与超限都走显式拦截 不做静默截断或默认值兜底
    expect(RESERVATION_SRC).toMatch(/if \(!form\.quantity \|\| form\.quantity <= 0\)/);
    expect(RESERVATION_SRC).toMatch(/form\.quantity > max/);
  });

  it('发货本次发货数量受控绑定 :model-value 与 @update:model-value 必须成对', () => {
    expect((DELIVERY_SRC.match(/<NumericInput/g) || []).length).toBe(1);
    expect(DELIVERY_SRC).toContain('import NumericInput from');
    const block = tagBlock(DELIVERY_SRC, 'NumericInput');
    expect(block, 'NumericInput 开标签块应可提取').not.toBe('');
    expect(block).toContain(':model-value="row.deliver_quantity"');
    expect(block).toMatch(
      /@update:model-value="\(v: number\) => updateItem\(row, 'deliver_quantity', v\)"/
    );
    // 动态上限承接：可发量 订单数量 减 已发货 与换控件前同一表达式
    expect(block).toMatch(/:max="row\.quantity - \(row\.delivered_quantity \|\| 0\)"/);
    // 清空与 0 都不发货：发货行筛选按正数判定 未发行为 0 不参与
    expect(DELIVERY_SRC).toMatch(/form\.items\.filter\(i => i\.deliver_quantity > 0\)/);
  });

  it('单源件机制 空文本按 optional 与 null 模型分流回写未填态', () => {
    // 空文本分支：optional 显式回 undefined 可空 null 模型保持 null 必填 number 模型回下界
    expect(NUMERIC_INPUT_SRC).toMatch(/next = props\.optional \? undefined : current;/);
    // 非法文本对可空 null 模型回 null 对可选模型回 undefined 不塞 NaN
    expect(NUMERIC_INPUT_SRC).toMatch(
      /next = bound === undefined && current === null \? null : bound;/
    );
    // 绑定器空文本对可选模型回 undefined 必填模型回下界（缺界回 0 只发生在必填 number 模型）
    expect(NUMBER_INPUT_UTILS_SRC).toMatch(
      /return current === undefined \? undefined : \(opts\.min \?\? 0\);/
    );
    // 回显归一：null 与 undefined 出空串 不给用户看见 null 字样或 0 假量
    expect(NUMBER_INPUT_UTILS_SRC).toMatch(
      /if \(v === null \|\| v === undefined \|\| !Number\.isFinite\(v\)\) return '';/
    );
  });
});
