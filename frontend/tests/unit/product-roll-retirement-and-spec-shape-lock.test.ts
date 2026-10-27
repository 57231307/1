/**
 * 产品每卷米数列下线与规格拼接形态的静态防回潮锁。
 *
 * 裁定口径（产品档案规格与每匹米数收口工单）：每卷米数是每匹米数的照抄假字段，
 * 一卷即一匹是唯一现网口径；products.meters_per_roll 先改代码再探测式迁移废弃。
 * 每匹米数保留为换算默认值列，但不再由产品新建与编辑表单采集。
 * 规格提交与显示统一为「幅宽 cm 星号 克重 g」字面拼接（星号是连接符不是乘法），
 * 由 utils/fabric-yardage.ts buildFabricSpecification 单源生成。
 *
 * 锁定的契约面（判据读源码结构，不按中文字面量判界面文案）：
 * 1. 产品表单不再绑定任何米数字段，也不再引用米数相关语言包键；
 * 2. 前端类型、后端 handler 创建与更新 DTO、服务参数对象、批量链路均无每卷米数键；
 *    每匹米数保持创建单层两态与更新双层三态形态；
 * 3. 同源补齐函数 pair_piece_roll_meters 整体退场（函数体与全部调用点都不复存在）；
 * 4. 换算器卷制腿与报价转订单卷制腿共用每匹米数单源；
 * 5. 探测式下线迁移在册，携带「先探测后 DROP、脏值即点名中止、绝不 UPDATE」的失败可见形态；
 * 6. 规格拼接是字符串模板而非数值乘法，且提交端与显示端同用一个派生源；
 * 7. 组织词表把针织拆为纬编与经编两组，落库值携带前缀可检可分；
 * 8. 表单数值录入不使用带加减步进控件（全项目硬红线，本锁钉新增面）；
 * 9. 批量提交顶层信封按后端批量 DTO 包 products，不发裸数组（旧锁仍有效，随迁保留）；
 * 10. 请求边界 toProductPayload 不把米数键改名或注入。
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
const yardageSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/utils/fabric-yardage.ts'));
const vocabSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/constants/product-spec-vocab.ts'));
const zhSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/locales/zh-CN.ts'));
const enSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/locales/en-US.ts'));
const handlerSrc = readRepoSource(path.join(REPO_ROOT, 'backend/src/handlers/product_handler.rs'));
const serviceSrc = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/product_service.rs'));
const crudSrc = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/product_ops/crud.rs'));
const batchSrc = readRepoSource(path.join(REPO_ROOT, 'backend/src/services/batch_service.rs'));
const converterSrc = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/utils/dual_unit_converter.rs')
);
const convertSrc = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/services/quotation_convert_service.rs')
);
const modelSrc = readRepoSource(path.join(REPO_ROOT, 'backend/src/models/product.rs'));
const migrationSrc = readRepoSource(
  path.join(
    REPO_ROOT,
    'backend/migration/src/domain/product_roll_column_drop/m0109_drop_product_meters_per_roll.rs'
  )
);

describe('产品每卷米数列下线与规格拼接形态锁', () => {
  it('表单不再绑定任何米数字段，也不引用米数语言包键', () => {
    for (const leak of [
      'meters_per_piece',
      'meters_per_roll',
      'labelMetersPerPiece',
      'tipMetersPerPiece',
      'labelMetersPerRoll',
      'tipMetersPerRoll',
    ]) {
      expect(formSrc.includes(leak), `产品表单不应再出现米数采集面残留 ${leak}`).toBe(false);
    }
    expect(
      formSrc.match(/v-model="formData\.meters_per_/g) || [],
      '产品表单里绑定米数字段的输入控件必须为 0 枚'
    ).toHaveLength(0);
  });

  it('表单数值录入不使用带加减步进控件（全项目硬红线在新增面的锁）', () => {
    expect(
      formSrc.includes('el-input-number'),
      '产品表单出现 el-input-number 即违反全项目禁步进控件红线'
    ).toBe(false);
  });

  it('前端 Product 类型不再有每卷米数键，每匹米数保持可选可空', () => {
    expect(apiSrc.includes('meters_per_roll'), '前端类型里每卷米数键必须整键退场').toBe(false);
    expect(
      /meters_per_piece\?: number \| string \| null;/.test(apiSrc),
      '每匹米数在 Product 里应保持可选与可空形态（出参读值，表单不采集）'
    ).toBe(true);
  });

  it('后端 handler 与参数对象与实体均无每卷米数键，每匹米数三态形态保持', () => {
    for (const [name, src] of [
      ['handler', handlerSrc],
      ['product_service', serviceSrc],
      ['crud', crudSrc],
      ['batch_service', batchSrc],
      ['models/product', modelSrc],
    ] as const) {
      expect(
        src.includes('meters_per_roll'),
        `${name} 里不应再有每卷米数键的任何声明或读写（列由 m0109 探测式下线）`
      ).toBe(false);
      expect(
        src.includes('pair_piece_roll_meters'),
        `${name} 里同源补齐函数必须整体退场，不得半退留壳`
      ).toBe(false);
    }
    expect(
      handlerSrc.includes('pub meters_per_piece: Option<Decimal>'),
      '创建 DTO 的每匹米数应保持单层两态（带值或缺值）'
    ).toBe(true);
    expect(
      handlerSrc.includes('pub meters_per_piece: Option<Option<Decimal>>'),
      '更新 DTO 的每匹米数应保持双层三态（覆盖、清空、不动），塌回单层即清空通道复发'
    ).toBe(true);
  });

  it('换算器与报价转订单的卷制腿共用每匹米数单源', () => {
    const rollBody = converterSrc.slice(converterSrc.indexOf('pub fn rolls_to_meters'));
    expect(rollBody.includes('meters_per_piece'), '卷制腿的换算入参必须是每匹米数').toBe(true);
    expect(rollBody.includes('meters_per_roll'), '卷制腿不得再消费每卷米数').toBe(false);
    // 换算腿的选取面按 match 臂标记取，不能按枚举名首次出现处取：
    // 单位字符串到 strategy 的映射块里也出现同样的枚举名，按首现取会截到映射块，
    // 断言读不到换算腿本体，把「腿仍在读每匹米数」误判成红。
    const rollsArmStart = convertSrc.indexOf('UnitStrategy::Rolls =>');
    const rollsArmEnd = convertSrc.indexOf('UnitStrategy::Kilograms =>');
    expect(rollsArmStart, '报价转订单必须有卷制换算腿').toBeGreaterThan(-1);
    expect(rollsArmEnd, '卷制腿之后必须紧跟公斤腿作为选取终点').toBeGreaterThan(rollsArmStart);
    const rollsArm = convertSrc.slice(rollsArmStart, rollsArmEnd);
    expect(
      rollsArm.includes('product.meters_per_piece'),
      '报价转订单卷制腿必须读每匹米数列（一卷即一匹）'
    ).toBe(true);
    expect(rollsArm.includes('meters_per_roll'), '报价转订单卷制腿不得再读每卷米数').toBe(false);
  });

  it('探测式下线迁移在册且携带失败可见形态', () => {
    expect(
      migrationSrc.includes("column_name = 'meters_per_roll'"),
      '迁移必须先探测列存在性，不存在即 NOTICE 跳过（幂等）'
    ).toBe(true);
    expect(
      migrationSrc.includes('IS DISTINCT FROM meters_per_roll'),
      '迁移必须先探测每卷米数非空且与每匹米数不等的脏行'
    ).toBe(true);
    expect(migrationSrc.includes('RAISE EXCEPTION'), '脏行必须点名中止，不得静默 DROP').toBe(true);
    expect(migrationSrc.includes('DROP COLUMN "meters_per_roll"'), '探测通过才下线本列').toBe(true);
    // 文档注释里允许出现 UPDATE 字样（说明"绝不 UPDATE"），判据只看 SQL 语句形态：
    // 迁移体不得出现对 products 的 UPDATE 或 DELETE，脏值只准 RAISE EXCEPTION 点名
    expect(
      /UPDATE\s+"?products"?/i.test(migrationSrc),
      '迁移绝不 UPDATE 业务数据，脏值交人工核对'
    ).toBe(false);
    expect(/DELETE\s+FROM\s+"?products"?/i.test(migrationSrc), '迁移绝不 DELETE 业务数据').toBe(
      false
    );
  });

  it('规格拼接是字符串模板单源，不是数值乘法', () => {
    const specFn = yardageSrc.slice(yardageSrc.indexOf('export function buildFabricSpecification'));
    const body = specFn.slice(0, specFn.indexOf('\n}\n'));
    expect(
      body.includes('${widthCm}cm*${gramWeight}g'),
      '拼接形态必须是 幅宽 cm 星号 克重 g 的字面模板'
    ).toBe(true);
    expect(/widthCm\s*\*\s*gramWeight/.test(body), '两段值出现数值乘法即违反裁定').toBe(false);
    expect(
      formSrc.includes('specification: specificationAuto.value ?? formData.specification'),
      '提交端必须消费同一派生源（禁止只拼显示端而提交端存空）'
    ).toBe(true);
    expect(
      formSrc.includes('buildFabricSpecification'),
      '表单必须从 fabric-yardage 单源引入拼接函数，不得本地再写一份'
    ).toBe(true);
  });

  it('组织词表把针织拆为纬编与经编两组，落库值携前缀可检', () => {
    expect(vocabSrc.includes('纬编汗布'), '词表必须含纬编前缀组织').toBe(true);
    expect(vocabSrc.includes('经编特里科'), '词表必须含经编前缀组织').toBe(true);
    expect(
      vocabSrc.includes('structureGroupWeftKnit') && vocabSrc.includes('structureGroupWarpKnit'),
      '纬编与经编分组必须走 i18n 键，不得裸中文分组名'
    ).toBe(true);
    expect(formSrc.includes('PRODUCT_STRUCTURE_GROUPS'), '表单组织框必须消费词表单源').toBe(true);
  });

  it('语言包不再出现每卷米数键（中英双侧）', () => {
    for (const [name, src] of [
      ['zh', zhSrc],
      ['en', enSrc],
    ] as const) {
      expect(src.includes('labelMetersPerRoll'), `${name} 里每卷米数标签键必须退场`).toBe(false);
      expect(src.includes('tipMetersPerRoll'), `${name} 里每卷米数提示键必须退场`).toBe(false);
      expect(
        src.includes('labelMetersPerPiece'),
        `${name} 里产品表单每匹米数标签键随采集面退场`
      ).toBe(false);
    }
  });

  it('批量提交的顶层信封与后端批量 DTO 对齐（products 包装，不发裸数组）', () => {
    // 源码面判定按语义匹配而不是按排版匹配：Prettier 会按行宽把对象字面量折行，
    // 逐字单行正则在排版变化后恒零命中，会把"包装其实还在"误报成契约破了。
    const flat = apiSrc.replace(/\s+/g, ' ');
    const wrapped = flat.match(/\{ products: data\.map\(toProductPayload\),? \}/g) || [];
    expect(
      wrapped.length,
      `批量创建与批量更新都应按后端顶层 DTO 包一层 products，实际命中 ${wrapped.length} 处`
    ).toBe(2);
    const bareArray = flat.match(/\/products\/batch\/(?:create|update)', data\)/g) || [];
    expect(
      bareArray.length,
      `批量创建与更新不得直发裸数组（后端顶层 DTO 是带 products 键的结构体），实际命中 ${bareArray.length} 处`
    ).toBe(0);
  });

  it('请求边界不把米数键改名或注入', () => {
    const mapper = apiSrc.match(/function toProductPayload[\s\S]*?\n}/);
    expect(mapper, '找不到 toProductPayload 请求边界映射函数').toBeTruthy();
    for (const injected of ['meters_per_piece:', 'meters_per_roll:']) {
      expect(
        mapper![0].includes(injected),
        `米数键不应在 toProductPayload 里被重新指定或注入（出现 ${injected} 即请求边界越权改载荷）`
      ).toBe(false);
    }
  });
});
