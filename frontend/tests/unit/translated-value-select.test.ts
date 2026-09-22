/**
 * 「译文当业务值」静态门禁
 *
 * 背景：el-option 的 :value 若绑定 t('…')，提交给后端的就是当前界面语言的文案，
 * 切换语言即改写业务数据（筛选失配、白名单 422、配伍判断失效）。
 * 已收敛的取值一律来自 src/constants/*，本用例做三件事：
 *   1. 全量扫描 :value="t('…')" 形态，残留点位必须与挂账清单逐一对应——
 *      多一处即失败，修完却不把清单清空也失败（清单已空即表示该类缺陷全仓收敛完）；
 *   2. 常量表里的 i18n 键在两门语言中真实存在——check-i18n.mjs 只识别
 *      t('字面量') 调用，键名以常量字段形式存放时逃过它，故在此补齐；
 *   3. 各常量表的取值与后端/写入端的真实词表逐项相等，防止把界面码改成库里没有的值。
 */
import { describe, it, expect } from 'vitest';
import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { relative, sep, join } from 'node:path';
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
import {
  LOGISTICS_EVENT_TYPE_LABEL_KEY,
  LOGISTICS_EVENT_TYPE_VALUES,
} from '@/constants/logistics-event-type';
import {
  LOGISTICS_COMPANY_LABEL_KEY,
  LOGISTICS_COMPANY_VALUES,
} from '@/constants/logistics-company';
import { QUOTATION_UNIT_LABEL_KEY, QUOTATION_UNIT_VALUES } from '@/constants/quotation-unit';

// 以启动目录定位 frontend 根：CI 与 `npm test` 都在 frontend/ 下运行。
// 不能用 import.meta.url + fileURLToPath —— vitest 的 jsdom 环境下 import.meta.url 不是
// file: 协议，会抛 ERR_INVALID_URL_SCHEME 把整个用例文件带崩。
const FRONTEND = process.cwd();
const SRC = join(FRONTEND, 'src');
if (!existsSync(join(SRC, 'views'))) {
  throw new Error(
    `该门禁需在 frontend 目录下运行以扫描 src/**/*.vue，当前目录 ${FRONTEND} 内没有 src/views`
  );
}

/**
 * 挂账清单：键为相对 frontend 的路径，值为该文件的残留点位数。
 * 27 处已全部核完并改完（物流公司、报价单位两组按「库里存的中文名即稳定值」收敛，
 * 质检/处方/工艺三组按后端词表收敛），故清单为空——此后任何新增点位都会让本用例失败。
 */
const KNOWN_PENDING: Record<string, number> = {};

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
      LOGISTICS_COMPANY_LABEL_KEY,
      QUOTATION_UNIT_LABEL_KEY,
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
    // 物流公司与报价单位：两列都是无字典自由文本，库里存的就是这套中文名
    expect([...LOGISTICS_COMPANY_VALUES]).toEqual([
      '顺丰速运',
      '中通快递',
      '圆通速递',
      '韵达快递',
      '京东物流',
    ]);
    expect([...QUOTATION_UNIT_VALUES]).toEqual(['米', '卷', '公斤', '件']);
    // 轨迹事件为小写码，与运单主状态的大写码分属两域（后端用例同口径钉住）
    expect([...LOGISTICS_EVENT_TYPE_VALUES]).toEqual([
      'pickup',
      'in_transit',
      'arrived',
      'delivered',
    ]);
    // 检验结论与库里唯一的自动写入方同源（outsourcing_ops/receipt.rs 写「合格/不合格」）
    expect([...QUALITY_RECORD_RESULT_VALUES]).toEqual(['待检', '合格', '不合格']);
  });
});
