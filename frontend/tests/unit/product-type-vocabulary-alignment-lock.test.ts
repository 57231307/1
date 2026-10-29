/**
 * 产品类型词表前后端同源锁与产品表单面料必录门接线锁。
 *
 * 背景口径：后端以 FABRIC_TYPES（坯布、成品布）判定面料类并要求幅宽、克重、面料成分必录；
 * 前端表单按同一集合动态加必填。词表一旦两侧漂移，就会出现「前端放行、后端拒」或反之。
 * 此锁用源码文本比对钉住三处：后端唯一词表、前端常量清单、表单必录接线。
 *
 * 锁定的契约面：
 * 1. constants/product-type.ts 的全量清单与面料集合取值等于后端 general.rs 的 ALL_TYPES/FABRIC_TYPES
 *    常量所指中文值（坯布/成品布/辅料，面料=坯布+成品布）；
 * 2. 表单组件必须接线面料必录门：引入 isFabricProductType 判定源（而非另抄一套类型比较），
 *    且面料分支引用幅宽/克重/成分的必录校验文案键；
 * 3. 类型 label 引用的 i18n 键在 zh-CN 与 en-US 两份语言包都存在，防止键悬空；
 * 4. 反向自证：面料集合不得混入辅料（混入即把非面料类也门成必填，判据与后端分叉）。
 *
 * 判据：读源码文本比对，不启动 Vue、不发网络请求、不连库。
 * 调用方：vitest（tests/unit，CI vitest job）。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

import {
  PRODUCT_TYPE_VALUES,
  FABRIC_PRODUCT_TYPE_VALUES,
  PRODUCT_TYPE_LABEL_KEY,
} from '@/constants/product-type';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/** 读仓库内源码并统一行尾为 LF：Windows 检出 CRLF、CI 检出 LF，归一只统一输入形态 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const backendVocabSrc = readRepoSource(
  path.join(REPO_ROOT, 'backend/src/models/status/general.rs')
);
const formSrc = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/product/tabs/ProductFormDialogTab.vue')
);
const zhLocaleSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/locales/zh-CN.ts'));
const enLocaleSrc = readRepoSource(path.join(FRONTEND_ROOT, 'src/locales/en-US.ts'));

describe('产品类型词表前后端同源锁', () => {
  it('后端唯一词表的中文值与前端常量清单逐项一致', () => {
    // 后端 general.rs 的三个类型常量定义（判据按定义行取中文值，不按顺序猜）
    for (const token of ['坯布', '成品布', '辅料']) {
      expect(
        backendVocabSrc.includes(`: &str = "${token}"`),
        `后端词表应含中文类型值「${token}」`
      ).toBe(true);
    }
    expect([...PRODUCT_TYPE_VALUES].sort()).toEqual(['坯布', '成品布', '辅料'].sort());
  });

  it('前端面料集合等于后端 FABRIC_TYPES（坯布+成品布），且不含辅料', () => {
    expect(
      backendVocabSrc.includes('pub const FABRIC_TYPES: &[&str] = &[GREIGE, FINISHED];'),
      '后端面料集合定义应为坯布加成品布'
    ).toBe(true);
    expect([...FABRIC_PRODUCT_TYPE_VALUES].sort()).toEqual(['坯布', '成品布'].sort());
    expect(
      FABRIC_PRODUCT_TYPE_VALUES.includes('辅料'),
      '辅料不得混入面料集合（混入即把非面料类也门成必填，与后端判据分叉）'
    ).toBe(false);
  });

  it('类型 label 的 i18n 键在两份语言包都存在，键不悬空', () => {
    for (const key of Object.values(PRODUCT_TYPE_LABEL_KEY)) {
      const leaf = key.split('.').pop() as string;
      expect(zhLocaleSrc, `zh-CN 应含类型文案键 ${key}`).toContain(leaf);
      expect(enLocaleSrc, `en-US 应含类型文案键 ${key}`).toContain(leaf);
    }
  });
});

describe('产品表单面料必录门接线锁', () => {
  it('表单接入选型判定源 isFabricProductType，不另抄类型比较', () => {
    expect(
      formSrc.includes('isFabricProductType'),
      '表单必须使用 constants/product-type.ts 的同源判定，禁止就地抄一份类型名单'
    ).toBe(true);
  });

  it('面料分支引用幅宽、克重、成分三项必录校验键，缺一即门失守', () => {
    for (const key of [
      'validateProductTypeRequired',
      'validateWidthRequiredFabric',
      'validateGramWeightRequiredFabric',
      'validateCompositionRequiredFabric',
    ]) {
      expect(formSrc, `表单应引用必录校验文案键 ${key}`).toContain(key);
      expect(zhLocaleSrc, `zh-CN 应定义必录校验文案键 ${key}`).toContain(key);
      expect(enLocaleSrc, `en-US 应定义必录校验文案键 ${key}`).toContain(key);
    }
  });

  it('主档文本列在表单有真实输入框并提交（成分/纱支/密度/组织结构等键双向不悬空）', () => {
    for (const field of [
      'fabric_composition',
      'yarn_count',
      'density',
      'structure',
      'finish',
      'min_order_quantity',
      'lead_time',
      'execution_standard',
      'factory_name',
      'factory_address',
      'product_grade',
    ]) {
      expect(formSrc, `表单应采集主档列 ${field}`).toContain(`formData.${field}`);
    }
  });

  it('表单提交十进制主档列经数值归一显式发 null 清空，且只读派生出米数不入载荷', () => {
    expect(
      formSrc,
      '幅宽必须经 toSubmitNumber 归一后提交（空文本显式 null 走三态清空通道）'
    ).toContain('width: toSubmitNumber(formData.width)');
    expect(
      formSrc,
      '克重必须经 toSubmitNumber 归一后提交（空文本显式 null 走三态清空通道）'
    ).toContain('gram_weight: toSubmitNumber(formData.gram_weight)');
    expect(
      formSrc.includes('meters_per_kg:'),
      'meters_per_kg 是后端派生只读键，不得进入提交载荷'
    ).toBe(false);
  });

  it('产品表单数字列禁用带加减按钮的数字输入框（el-input-number 与隐藏按钮写法都不许出现）', () => {
    expect(
      formSrc.includes('el-input-number'),
      '产品表单的数量、单价、米数、克重、幅宽、交货期等数字列必须用 el-input 加校验规则'
    ).toBe(false);
    expect(
      formSrc.includes(':controls="false"'),
      '不得用 controls=false 隐藏加减按钮来冒充合规（全项目禁带加减按钮数字输入框）'
    ).toBe(false);
  });
});
