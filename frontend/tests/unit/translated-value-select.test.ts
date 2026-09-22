/**
 * 「译文当业务值」静态门禁
 *
 * 背景：el-option 的 :value 若绑定 t('…')，提交给后端的就是当前界面语言的文案，
 * 切换语言即改写业务数据（筛选失配、白名单 422、配伍判断失效）。
 * 已收敛的取值一律来自 src/constants/*，本用例只做两件事：
 *   1. 残留点位必须与挂账清单逐一对应（多一处即失败，修完不减清单也失败）；
 *   2. 常量表里的 i18n 键在两门语言中真实存在——check-i18n.mjs 只识别
 *      t('字面量') 调用，键名以常量字段形式存放时逃过它，故在此补齐。
 */
import { describe, it, expect } from 'vitest';
import { readdirSync, readFileSync } from 'node:fs';
import { relative, sep, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import zhCN from '@/locales/zh-CN';
import enUS from '@/locales/en-US';
import {
  QUALITY_INSPECTION_TYPE_LABEL_KEY,
  QUALITY_INSPECTION_TYPE_VALUES,
} from '@/constants/quality-inspection-type';
import {
  RECIPE_FABRIC_TYPE_LABEL_KEY,
  RECIPE_FABRIC_TYPE_VALUES,
} from '@/constants/recipe-fabric-type';
import { DYE_TYPE_LABEL_KEY, DYE_TYPE_VALUES } from '@/constants/dye-type';
import {
  QUALITY_INSPECTION_SOURCE_LABEL_KEY,
  QUALITY_RECORD_RESULT_LABEL_KEY,
  QUALITY_RECORD_RESULT_VALUES,
} from '@/constants/quality-inspection-record';
import { LOGISTICS_EVENT_TYPE_LABEL_KEY } from '@/constants/logistics-event-type';

// 以本文件位置定位 frontend 根，避免依赖 vitest 启动时的 cwd
const FRONTEND = fileURLToPath(new URL('../..', import.meta.url));
const SRC = fileURLToPath(new URL('../../src', import.meta.url));

/** 已挂账待收敛：键为相对 frontend 的路径，值为该文件的残留点位数 */
const KNOWN_PENDING: Record<string, number> = {
  // 物流公司词典未建（后端 logistics_company 为自由文本，存量以中文名落库）
  'src/views/logistics/components/LogisticsFilter.vue': 5,
  'src/views/logistics/components/LogisticsForm.vue': 5,
  // 报价单计量单位同样以中文名单元格落库，需与单位词典一并处理
  'src/views/quotations/components/QuotationItemEditor.vue': 3,
};

const TRANSLATED_VALUE_RE = /:value="\$?t\(\s*'[^']+'\s*\)"/g;

function walkVue(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const p = join(dir, entry.name);
    if (entry.isDirectory()) walkVue(p, out);
    else if (entry.name.endsWith('.vue')) out.push(p);
  }
  return out;
}

function countTranslatedValues(absPath: string): number {
  return readFileSync(absPath, 'utf-8').match(TRANSLATED_VALUE_RE)?.length ?? 0;
}

function hasKey(messages: Record<string, unknown>, dotted: string): boolean {
  return (
    dotted
      .split('.')
      .reduce<unknown>(
        (node, seg) =>
          typeof node === 'object' && node !== null
            ? (node as Record<string, unknown>)[seg]
            : undefined,
        messages
      ) !== undefined
  );
}

describe('「译文当业务值」门禁', () => {
  it('残留点位与挂账清单逐一对应', () => {
    const found: Record<string, number> = {};
    for (const file of walkVue(SRC)) {
      const n = countTranslatedValues(file);
      if (n > 0) found[relative(FRONTEND, file).split(sep).join('/')] = n;
    }
    expect(found).toEqual(KNOWN_PENDING);
  });

  it('常量表引用的 i18n 键在两门语言中都存在', () => {
    const maps = [
      QUALITY_INSPECTION_TYPE_LABEL_KEY,
      RECIPE_FABRIC_TYPE_LABEL_KEY,
      DYE_TYPE_LABEL_KEY,
      QUALITY_RECORD_RESULT_LABEL_KEY,
      QUALITY_INSPECTION_SOURCE_LABEL_KEY,
      LOGISTICS_EVENT_TYPE_LABEL_KEY,
    ];
    for (const map of maps) {
      for (const key of Object.values(map)) {
        expect([key, hasKey(zhCN, key)]).toEqual([key, true]);
        expect([key, hasKey(enUS, key)]).toEqual([key, true]);
      }
    }
  });

  it('选项值与文案键一一对应，无多余键', () => {
    expect(Object.keys(QUALITY_INSPECTION_TYPE_LABEL_KEY).sort()).toEqual(
      [...QUALITY_INSPECTION_TYPE_VALUES].sort()
    );
    expect(Object.keys(RECIPE_FABRIC_TYPE_LABEL_KEY).sort()).toEqual(
      [...RECIPE_FABRIC_TYPE_VALUES].sort()
    );
    expect(Object.keys(DYE_TYPE_LABEL_KEY).sort()).toEqual([...DYE_TYPE_VALUES].sort());
  });

  it('选项词表与后端/写入端口径一致', () => {
    // 染料码必须落在后端白名单内（handlers/ai_extend_handler.rs 的 valid_dyes）
    expect([...DYE_TYPE_VALUES].sort()).toEqual(
      ['reactive', 'disperse', 'acid', 'vat', 'direct', 'cationic', 'sulfur'].sort()
    );
    // 检验类型码必须与质检记录写入端一致（views/quality/index.vue 的 el-option value）
    expect([...QUALITY_INSPECTION_TYPE_VALUES].sort()).toEqual(
      ['incoming', 'process', 'finished', 'outgoing'].sort()
    );
    // 布类名必须是后端配伍表认得的中文写法（services/ai/recipe_opt.rs）
    expect([...RECIPE_FABRIC_TYPE_VALUES]).toEqual(['棉', '涤纶', '丝绸', '羊毛']);
    // 检验结论与库里唯一的自动写入方同源（outsourcing_ops/receipt.rs 写「合格/不合格」）
    expect([...QUALITY_RECORD_RESULT_VALUES]).toEqual(['待检', '合格', '不合格']);
  });
});
