/**
 * 产品米数单入口形状锁（防回潮）。
 *
 * 裁定口径：一卷即一匹，产品新建与编辑表单只留「每匹米数」一个可编辑入口，
 * 「每卷米数」改为由它带出的只读项；后端在请求只带其中一个值时按同值双写两列。
 * 硬边界同样钉住：不删列、不改单位词表、不动历史数据。
 *
 * 锁定的契约面：
 * 1. 表单里米数输入只有两枚，且只有每匹米数可编辑，每卷米数带只读标记；
 * 2. 只读项的带出由用户改动触发，回显赋值被抑制，避免打开编辑再保存就改写历史每卷米数；
 * 3. 提交载荷的两键名与后端创建与更新 DTO 的字段名逐字对齐（创建单层两态、更新双层三态），
 *    且请求边界不改名、不并入；
 * 4. 单位词表里匹与卷仍是并列 token，两列仍是实体与建表迁移的在册列；
 * 5. 只读项的用户可见文案只说该做什么，不泄漏表名、列名与实现细节；
 * 6. 表单提交显式收集载荷（不回传整行），清空米数送显式 null（省略键在后端是保持原值）；
 * 7. 批量提交的顶层信封按后端批量 DTO 包 products，不发裸数组。
 *
 * 判据：读源码文本比对，不启动 Vue、不发网络请求、不连库。
 * 调用方：vitest（tests/unit，CI vitest job）。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf=true 下为 CRLF、
 * CI 检出为 LF，正则夹具含行边界对形态敏感——不归一则本地 CRLF 下断言静默落空。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const formSrc = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/product/tabs/ProductFormDialogTab.vue')
);
const apiSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/product.ts'));
const unitSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/constants/quotation-unit.ts'));
const handlerSrc = readRepoSource(path.join(REPO_ROOT, 'backend/src/handlers/product_handler.rs'));
const zhSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/locales/zh-CN.ts'));

/** 取某个 v-model 绑定所在的输入控件块（绑定点到最近的自闭合标签结尾） */
function inputBlock(model: string): string {
  const block = formSrc.match(new RegExp(`v-model="${model}"[\\s\\S]*?/>`));
  expect(block, `找不到绑定 ${model} 的输入控件块`).toBeTruthy();
  return block![0];
}

describe('产品米数单入口形状锁', () => {
  it('米数输入只有两枚，不多出第三枚可编辑入口', () => {
    const metersInputs = formSrc.match(/v-model="formData\.meters_per_/g) || [];
    expect(
      metersInputs.length,
      `表单里绑定米数字段的输入控件应为 2 枚，实际 ${metersInputs.length} 枚`
    ).toBe(2);
  });

  it('每匹米数保持可编辑，不带只读或禁用标记', () => {
    const block = inputBlock('formData.meters_per_piece');
    expect(block.includes('readonly'), '每匹米数是唯一入口，不得被改成只读').toBe(false);
    expect(block.includes('disabled'), '每匹米数是唯一入口，不得被改成禁用').toBe(false);
  });

  it('每卷米数是只读派生项', () => {
    const block = inputBlock('formData.meters_per_roll');
    expect(block.includes('readonly'), '每卷米数必须标记只读，用户不再直接输入').toBe(true);
  });

  it('每卷米数由每匹米数的用户改动带出，回显赋值被抑制', () => {
    expect(
      /watch\(\s*\(\) => formData\.meters_per_piece,[\s\S]*?formData\.meters_per_roll = val/.test(
        formSrc
      ),
      '缺少由每匹米数带出每卷米数的同步逻辑'
    ).toBe(true);

    expect(formSrc.includes('echoing.value = true'), '回显开始时应置起抑制标志').toBe(true);
    expect(formSrc.includes('echoing.value = false'), '回显结束时应放下抑制标志').toBe(true);
    expect(formSrc.includes('if (echoing.value) return'), '带出逻辑应跳过回显触发的赋值').toBe(
      true
    );
    // 抑制标志必须在 finally 里释放，否则异常会让只读派生静默失效且无从察觉
    expect(formSrc.includes('} finally {'), '回显抑制必须用 finally 释放，避免标志卡死').toBe(true);
  });

  it('提交载荷的米数键名与后端创建与更新 DTO 逐字对齐', () => {
    // 创建侧 DTO 为单层两态（带值或缺值），更新侧 DTO 为双层三态（覆盖、清空、不动），
    // 清空通道依赖显式 null 与键缺席可区分，单层形态没有清空通道
    for (const field of ['meters_per_piece', 'meters_per_roll']) {
      expect(
        handlerSrc.includes(`pub ${field}: Option<Decimal>`),
        `后端创建 DTO 应声明 ${field} 为单层两态`
      ).toBe(true);
      expect(
        handlerSrc.includes(`pub ${field}: Option<Option<Decimal>>`),
        `后端更新 DTO 的 ${field} 应为双层三态，塌回单层即清空通道复发`
      ).toBe(true);
    }
    const tristate = handlerSrc.match(/deserialize_with = "double_option"/g) || [];
    expect(
      tristate.length,
      `更新 DTO 的米数两键与价格两列、幅宽、克重、最小起订量、交货期六个主档列都应挂三态反序列化适配器，实际 ${tristate.length} 处`
    ).toBe(8);
    // 三态面逐列点名，防止某一列塌回单层后清空通道静默失效
    for (const field of [
      'meters_per_piece',
      'meters_per_roll',
      'standard_price',
      'cost_price',
      'width',
      'gram_weight',
      'min_order_quantity',
      'lead_time',
    ]) {
      expect(
        handlerSrc.includes(`pub ${field}: Option<Option<`),
        `后端更新 DTO 的 ${field} 应为双层三态，塌回单层即清空通道复发`
      ).toBe(true);
    }

    // 请求边界不得把这两键改名或并进别的键：toProductPayload 只映射四个别名
    const mapper = apiSrc.match(/function toProductPayload[\s\S]*?\n}/);
    expect(mapper, '找不到 toProductPayload 请求边界映射函数').toBeTruthy();
    for (const renamed of ['meters_per_piece:', 'meters_per_roll:']) {
      expect(
        mapper![0].includes(renamed),
        `米数键应在请求边界原样透传，不应在 toProductPayload 里被重新指定为 ${renamed}`
      ).toBe(false);
    }

    // 前端响应接口仍保留两键且仍可空，避免把可空列谎成必填
    for (const field of ['meters_per_piece', 'meters_per_roll']) {
      expect(
        new RegExp(`${field}\\?: number \\| string \\| null;`).test(apiSrc),
        `Product 接口里 ${field} 应保持可选与可空形态`
      ).toBe(true);
    }
  });

  it('表单提交显式收集载荷，不回传整行，清空米数送显式 null', () => {
    expect(
      formSrc.includes('createProduct(formData)'),
      '新建不得再把整个 formData（含回显带入的行内服务端列）原样提交'
    ).toBe(false);
    expect(
      formSrc.includes('updateProduct(formData.id as number, formData)'),
      '编辑不得再把整个 formData 原样提交'
    ).toBe(false);
    for (const field of ['meters_per_piece', 'meters_per_roll']) {
      expect(
        formSrc.includes(`${field}: toSubmitNumber(formData.${field})`),
        `提交载荷里 ${field} 应经 toSubmitNumber 归一后显式送 null（省略键在后端是保持原值，清空会失效）`
      ).toBe(true);
    }
    expect(
      formSrc.includes("return text === '' ? null : Number(text)"),
      'toSubmitNumber 必须把空文本归一为显式 null，清空通道不得塌回键缺席'
    ).toBe(true);
    expect(
      formSrc.includes('buildSubmitPayload'),
      '提交前应经显式载荷构造，字段清单即表单采集面'
    ).toBe(true);
  });

  it('批量提交的顶层信封与后端批量 DTO 对齐（products 包装，不发裸数组）', () => {
    // 源码面判定按语义匹配而不是按排版匹配：Prettier 会按行宽把对象字面量折行，
    // 逐字单行正则在排版变化后恒零命中，会把"包装其实还在"误报成契约破了。
    // 先把连续空白归一成一个空格，包装真被删掉时归一后的串不再命中。
    const flat = apiSrc.replace(/\s+/g, ' ');
    const wrapped = flat.match(/\{ products: data\.map\(toProductPayload\),? \}/g) || [];
    expect(
      wrapped.length,
      `批量创建与批量更新都应按后端顶层 DTO 包一层 products，实际命中 ${wrapped.length} 处`
    ).toBe(2);
    // 同一条锁的反面：直发裸数组在后端顶层 DTO 下必反序列化失败，出现即判红
    const bareArray = flat.match(/\/products\/batch\/(?:create|update)', data\)/g) || [];
    expect(
      bareArray.length,
      `批量创建与更新不得直发裸数组（后端顶层 DTO 是带 products 键的结构体），实际命中 ${bareArray.length} 处`
    ).toBe(0);
  });

  it('硬边界：单位词表未改，匹与卷仍是并列单位 token', () => {
    expect(unitSrc.includes("bolt: '匹'"), '单位词表不得删掉匹').toBe(true);
    expect(unitSrc.includes("roll: '卷'"), '单位词表不得删掉卷').toBe(true);
  });

  it('硬边界：后端两列取值仍经同源补齐，未退回各写各列', () => {
    const callSites =
      handlerSrc.match(/pair_piece_roll_meters\(req\.meters_per_piece, req\.meters_per_roll\)/g) ||
      [];
    expect(callSites.length, '创建与更新都应经同源补齐取值').toBe(2);
    expect(
      handlerSrc.includes('meters_per_piece: req.meters_per_piece'),
      '两列不得绕过补齐直接透传请求字段'
    ).toBe(false);
  });

  it('只读项文案只说该做什么，不泄漏实现细节', () => {
    const tip = zhSrc.match(/tipMetersPerRoll:\s*'([^']*)'/);
    expect(tip, '找不到每卷米数的提示文案').toBeTruthy();
    for (const leak of ['products', 'meters_per_', 'DECIMAL', '库存计量', '换算视图']) {
      expect(tip![1].includes(leak), `用户可见文案不应出现实现细节 ${leak}`).toBe(false);
    }
    expect(tip![1].includes('每匹米数'), '文案应指向唯一入口').toBe(true);
  });
});
