/**
 * sc-contract-detail-xss.test.ts — 销售合同详情弹层的出参注入锁。
 * 功能：锁定详情弹层的渲染契约：行对象里人工录入字段（编号/名称/客户/付款条件/付款方式/交货地点）
 * 携带 HTML 片段时，弹层渲染结果必须以字面文本呈现且不产生 img/script 等元素；消息载体必须是
 * VNode 结构化通道而非「HTML 字符串 + dangerouslyUseHTMLString」通道；同时锁正常形态：
 * 11 个字段行齐全、Decimal 字符串金额显示为格式化值、空字段占位 '-'。
 * 调用方：vitest run（tests/unit）。
 * 入参：自造 SalesContract 行对象（六个字段各嵌一枚可辨识 XSS 载荷，其余取正常值/null）。
 * 传给谁：expect 断言。ElMessageBox.alert 用间谍捕获实参，不建真实弹层。
 * 存什么：DOM 渲染判据（无 img/script 元素、载荷逐枚以字面文本在场）与消息形态判据
 * （isVNode 为真、dangerouslyUseHTMLString 不为真）；renderMessageBoxMessage 复刻 Element Plus
 * MessageBox 的消息处理分支（字符串且 dangerouslyUseHTMLString ⇒ innerHTML，字符串否则 ⇒ 文本，
 * VNode ⇒ 组件渲染），判据实现单源在弹层调用方源码，本文件只做消费侧核验。
 * 存哪里：本文件；被测实现 src/views/sales-contract/composables/useScProc.ts 的 handleView。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { h, isVNode, defineComponent, type VNode } from 'vue';
import { mount } from '@vue/test-utils';
import { ElMessageBox } from 'element-plus';

// 出参 API 与后端解耦：本用例只验弹层渲染契约，不触网。
vi.mock('@/api/sales-contract', () => ({
  deleteSalesContract: vi.fn(),
  approveSalesContract: vi.fn(),
  rejectSalesContract: vi.fn(),
  executeSalesContract: vi.fn(),
}));

import type { SalesContract } from '@/api/sales-contract';
import { useScProc } from '@/views/sales-contract/composables/useScProc';

/** 一枚按宿主字段命名的载荷：被当 HTML 解析会造出 img，当文本渲染则原样在场 */
const xssPayload = (tag: string) => `<img alt="${tag}" src=x onerror="xss_${tag}">`;

/** 六个手工录入字段各嵌一枚载荷；其余字段取正常值或 null */
function makePoisonedRow(): SalesContract {
  return {
    id: 1,
    contract_no: `HT-${xssPayload('no')}`,
    contract_name: `合同${xssPayload('name')}`,
    contract_type: null,
    customer_id: 7,
    customer_name: `客户${xssPayload('customer')}`,
    total_amount: '12345.5',
    signed_date: '2026-01-02',
    effective_date: null,
    expiry_date: null,
    payment_terms: `月结${xssPayload('terms')}`,
    payment_method: `电汇${xssPayload('method')}`,
    delivery_date: null,
    delivery_location: `仓库${xssPayload('location')}`,
    remark: null,
    status: 'draft',
    created_by: 1,
    created_at: '2026-01-02',
    updated_at: '2026-01-02',
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
  } as unknown as SalesContract;
}

/**
 * 按 ElMessageBox 的消息处理分支把实参落成 DOM：
 * 字符串 + dangerouslyUseHTMLString ⇒ innerHTML（注入通道），字符串否则 ⇒ 纯文本，VNode ⇒ 渲染。
 */
function renderMessageBoxMessage(
  message: string | VNode,
  options: Record<string, unknown> | undefined
): HTMLElement {
  if (typeof message === 'string' && options?.dangerouslyUseHTMLString === true) {
    const el = document.createElement('div');
    el.innerHTML = message;
    return el;
  }
  if (typeof message === 'string') {
    const el = document.createElement('div');
    el.textContent = message;
    return el;
  }
  const wrapper = mount(defineComponent({ render: () => message }));
  return wrapper.element as HTMLElement;
}

describe('销售合同详情弹层（handleView 出参注入锁）', () => {
  let alertSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    alertSpy = vi
      .spyOn(ElMessageBox, 'alert')
      .mockImplementation(() => Promise.resolve('ok') as never);
  });

  afterEach(() => {
    alertSpy.mockRestore();
  });

  it('恶意字段经弹层渲染后不产生 img/script 元素且逐枚以字面文本在场', () => {
    const { handleView } = useScProc({ getList: vi.fn().mockResolvedValue(undefined) });
    handleView(makePoisonedRow());

    expect(alertSpy).toHaveBeenCalledTimes(1);
    const [message, , options] = alertSpy.mock.calls[0] as [
      string | VNode,
      string,
      Record<string, unknown> | undefined,
    ];

    const dom = renderMessageBoxMessage(message, options);
    expect(dom.querySelector('img')).toBeNull();
    expect(dom.querySelector('script')).toBeNull();
    for (const tag of ['no', 'name', 'customer', 'terms', 'method', 'location']) {
      expect(dom.textContent).toContain(xssPayload(tag));
    }
  });

  it('消息载体走 VNode 结构化通道，不得开启 dangerouslyUseHTMLString', () => {
    const { handleView } = useScProc({ getList: vi.fn().mockResolvedValue(undefined) });
    handleView(makePoisonedRow());

    const [message, , options] = alertSpy.mock.calls[0] as [
      string | VNode,
      string,
      Record<string, unknown> | undefined,
    ];
    expect(isVNode(message)).toBe(true);
    expect(options?.dangerouslyUseHTMLString).not.toBe(true);
  });

  it('正常行渲染：11 个字段行、Decimal 字符串金额格式化、null 字段占位为 -', () => {
    const { handleView } = useScProc({ getList: vi.fn().mockResolvedValue(undefined) });
    handleView(makePoisonedRow());

    const [message, , options] = alertSpy.mock.calls[0] as [
      string | VNode,
      string,
      Record<string, unknown> | undefined,
    ];
    const dom = renderMessageBoxMessage(message, options);
    const texts = [...dom.querySelectorAll('p')];
    expect(texts.length).toBe(11);
    expect(dom.textContent).toContain('合同金额：¥12345.50');
    expect(dom.textContent).toContain('生效日期：-');
    expect(dom.textContent).toContain('签订日期：2026-01-02');
  });
});
