/**
 * 销售合同显示层缺陷锁（列标题响应式 / 详情弹窗转义 / 详情弹窗状态外显）。
 * 功能：
 * 1) SalesContractTable.vue::columns 必须是 computed——语言切换只改 i18n locale、
 *    不重挂载组件，非 computed 的 setup 一次性求值会把列标题冻在旧语言。
 *    退化说明：若源码退回 `const columns = [ title: t(...) ]` 普通数组，列标题在
 *    setup 只求值一次，切到 en-US 后 title 仍是中文，第二条断言必红；
 * 2) 详情弹窗（ElMessageBox.alert + dangerouslyUseHTMLString）的字段插值必须全量
 *    过 escapeHtml。退化说明：若退回未转义插值，含 <img onerror> 的合同名会以原文
 *    进入 HTML 串，&lt;img 断言与不含 <img 原文断言双双必红；
 * 3) 详情弹窗必须外显单据状态且取自 getStatusLabel（i18n 词表）。退化说明：若删掉
 *    状态行，en-US 下取不到 'Active' 必红；若把状态写成硬编码中文，切到 en-US 后
 *    仍出现 '执行中' 且缺 'Active'，两向断言必红。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：真实 locales 文案与内存构造的合同行夹具，不发网络请求、不依赖活库。
 * 传给谁：断言结果只进 vitest 报告；存什么/存哪里：不落盘。
 */
import { nextTick } from 'vue';
import { describe, expect, it, vi, beforeEach, afterEach, type Mock } from 'vitest';
import { mount } from '@vue/test-utils';
import { createI18n } from 'vue-i18n';
import { ElMessageBox } from 'element-plus';

import SalesContractTable from '@/views/sales-contract/components/SalesContractTable.vue';
import V2Table from '@/components/V2Table/index.vue';
import { useScProc } from '@/views/sales-contract/composables/useScProc';
import { i18n } from '@/i18n';
import type { SalesContract } from '@/api/sales-contract';
import zhCN from '@/locales/zh-CN';
import enUS from '@/locales/en-US';

/** 构造列表接口形状齐全的合同行夹具；overrides 注入被测字段 */
function makeRow(overrides: Partial<SalesContract> = {}): SalesContract {
  return {
    id: 1,
    contract_no: 'SC-TEST-0001',
    contract_name: '测试合同',
    contract_type: 'sales',
    customer_id: 10,
    customer_name: '测试客户',
    total_amount: '1000.00',
    signed_date: '2026-01-01',
    effective_date: '2026-01-02',
    expiry_date: '2026-12-31',
    payment_terms: '月结30天',
    payment_method: 'bank_transfer',
    delivery_date: '2026-02-01',
    delivery_location: '测试仓',
    remark: null,
    status: 'active',
    created_by: 1,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    signed_at: null,
    signed_by_user_id: null,
    signature_hash: null,
    signature_image_url: null,
    signature_certificate: null,
    quality_terms: null,
    breach_liability: null,
    dispute_resolution: null,
    performance_period: null,
    stamp_tax_amount: null,
    ...overrides,
  };
}

describe('销售合同列表列定义的语言响应式', () => {
  // 测试内自建 i18n 实例（与 src/i18n 同构：legacy false、真实中英双侧），
  // mount 传入后覆盖 setup.ts 全局实例，locale 翻转不重挂载组件，复现生产切语言路径
  const testI18n = createI18n({
    legacy: false,
    locale: 'zh-CN',
    fallbackLocale: 'en-US',
    messages: { 'zh-CN': zhCN, 'en-US': enUS },
  });

  afterEach(() => {
    testI18n.global.locale.value = 'zh-CN';
  });

  it('切语言后 columns 的 title 跟随重算（先 zh 断、再 en 断）', async () => {
    const wrapper = mount(SalesContractTable, {
      props: {
        contractList: [],
        loading: false,
        total: 0,
        page: 1,
        pageSize: 10,
      },
      // el-card 用透传插槽的轻量桩：VTU 自动桩不渲染默认插槽，会遮蔽 V2Table 挂载点
      global: {
        plugins: [testI18n],
        stubs: { 'el-card': { name: 'ElCardStub', template: '<div><slot /></div>' } },
      },
      // shallow 下 V2Table 自动打桩：只需读它实际收到的 columns prop，不渲染虚拟表格内部
      shallow: true,
    });
    const v2 = () => wrapper.findComponent(V2Table).props('columns');

    const titleZh = v2().map(c => c.title);
    expect(titleZh).toContain(zhCN.salesContract.table.columnContractNo);
    expect(titleZh).toContain(zhCN.salesContract.table.columnStatus);

    testI18n.global.locale.value = 'en-US';
    await nextTick();

    const titleEn = v2().map(c => c.title);
    expect(titleEn).toContain(enUS.salesContract.table.columnContractNo);
    expect(titleEn).toContain(enUS.salesContract.table.columnStatus);
    // 英文侧不得残留中文标题：冻在旧语言正是本缺陷的形态
    expect(titleEn).not.toContain(zhCN.salesContract.table.columnContractNo);

    wrapper.unmount();
  });
});

describe('销售合同详情弹窗的 HTML 转义口径', () => {
  let alertSpy: Mock;

  beforeEach(() => {
    alertSpy = vi
      .spyOn(ElMessageBox, 'alert')
      .mockImplementation(() => Promise.resolve('confirm' as never));
  });

  afterEach(() => {
    vi.restoreAllMocks();
    i18n.global.locale.value = 'zh-CN';
  });

  /** 调 handleView 并取回 ElMessageBox.alert 的完整入参（[0] 内容 HTML，[1] 标题，[2] 选项） */
  function captureAlertArgs(row: SalesContract): unknown[] {
    alertSpy.mockClear();
    const { handleView } = useScProc({ getList: async () => {} });
    handleView(row);
    expect(alertSpy).toHaveBeenCalledTimes(1);
    return alertSpy.mock.calls[0];
  }

  it('含恶意标签的名称与备注进弹窗前必须被转义，不留可执行标签原文', () => {
    const html = String(
      captureAlertArgs(
        makeRow({
          contract_name: '<img src=x onerror=alert(1)>',
          payment_terms: '<script>alert(document.cookie)</script>',
        })
      )[0]
    );
    // 转义产物形态断言：尖括号成实体
    expect(html).toContain('&lt;img src=x onerror=alert(1)&gt;');
    expect(html).toContain('&lt;script&gt;alert(document.cookie)&lt;/script&gt;');
    // 反向断言：不得残留可执行标签原文（dangerouslyUseHTMLString 直渲，残留即 XSS 面）
    expect(html).not.toContain('<img');
    expect(html).not.toContain('<script');
    expect(html).not.toContain('onerror=alert(1)<');
  });

  it('弹窗标题、标签与关闭按钮取自 i18n，中英双侧都有真实文案', () => {
    i18n.global.locale.value = 'zh-CN';
    const argsZh = captureAlertArgs(makeRow());
    expect(String(argsZh[1])).toBe(zhCN.salesContract.proc.detailTitle);
    expect(String(argsZh[0])).toContain(zhCN.salesContract.proc.labelContractNo);
    expect((argsZh[2] as { confirmButtonText?: string }).confirmButtonText).toBe(zhCN.common.close);

    i18n.global.locale.value = 'en-US';
    const argsEn = captureAlertArgs(makeRow());
    expect(String(argsEn[1])).toBe(enUS.salesContract.proc.detailTitle);
    expect(String(argsEn[0])).toContain(enUS.salesContract.proc.labelContractNo);
    expect((argsEn[2] as { confirmButtonText?: string }).confirmButtonText).toBe(enUS.common.close);
    // 英文界面不得残留中文标签：硬编码或语言冻住正是本缺陷形态
    expect(String(argsEn[0])).not.toContain(zhCN.salesContract.proc.labelContractNo);
    i18n.global.locale.value = 'zh-CN';
  });
});

describe('销售合同详情弹窗外显单据状态', () => {
  let alertSpy: Mock;

  beforeEach(() => {
    alertSpy = vi
      .spyOn(ElMessageBox, 'alert')
      .mockImplementation(() => Promise.resolve('confirm' as never));
  });

  afterEach(() => {
    vi.restoreAllMocks();
    i18n.global.locale.value = 'zh-CN';
  });

  it('状态行文案取自 getStatusLabel 词表而非硬编码（zh 与 en 双向断言）', () => {
    const { handleView } = useScProc({ getList: async () => {} });

    i18n.global.locale.value = 'zh-CN';
    handleView(makeRow({ status: 'active' }));
    expect(alertSpy).toHaveBeenCalledTimes(1);
    const htmlZh = String(alertSpy.mock.calls[0][0]);
    expect(htmlZh).toContain(zhCN.salesContract.table.statusActive);
    expect(htmlZh).toContain(zhCN.salesContract.proc.labelStatus);

    alertSpy.mockClear();
    i18n.global.locale.value = 'en-US';
    handleView(makeRow({ status: 'active' }));
    const htmlEn = String(alertSpy.mock.calls[0][0]);
    // 词表来源证明：同一行状态文案随 locale 变为权威英文词表值
    expect(htmlEn).toContain(enUS.salesContract.table.statusActive);
    // 硬编码中文即本断言的红点：英文弹窗不得出现中文状态词
    expect(htmlEn).not.toContain(zhCN.salesContract.table.statusActive);
  });
});
