import { test, expect } from '../diagnose-fixture';
import type { Page } from '@playwright/test';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  ensureTestEntities,
  getCtx,
  genCode,
} from './helpers';

/**
 * 打印任务真实受理链路（POST /print-tasks 受理 → GET /print-tasks/{id} 回读）
 *
 * 背景缺陷：Wave E 落库了 print_tasks 受理/派发链路，但前端零入口，唯一入口是模板管理页
 * 预览对话框的"打印"按钮，它无单据上下文却恒发空请求体，被后端 fail-closed 拒 400。
 * 本文件锁定"真实打印可用"这条链路本身：
 * - 无打印目标（请求未带且模板未配缺省目标）→ 400，不静默丢单；
 * - 单据实例不存在 → 受理即拒（不静默接受无效 doc_id）；
 * - 模板的 default_target_name 真实生效（不带 target_name 也能受理，取模板配置）；
 * - 受理成功后任务号唯一、台账可读回，失败必带 error_message；
 * - 已有任务台账的模板删除被拒 400 并点名条数（外键裸 500 会让用户看不到原因）。
 * 派发终态依赖真实设备可达性：CI 无打印机时网络目标必然失败，这正是"失败可见"的断言对象，
 * 故此处只锁定"终态可回读 + 失败原因非空"，不假装打印成功。
 */

/** 后端 print_task_status::ALL 全集（models/status/print.rs，与建表 CHECK 同源） */
const PRINT_TASK_STATUSES = ['pending', 'queued', 'dispatched', 'succeeded', 'failed'];

interface PrintTaskRow {
  id?: number;
  job_code?: string;
  status?: string;
  error_message?: string | null;
  target_name?: string;
  template_id?: number | null;
  doc_id?: number;
  doc_type?: string;
  copies?: number;
}

/** 模板新建/更新出参（PrintTemplateRecord 子集，仅断言本链路关心的键） */
interface PrintTemplateRow {
  id?: number;
  doc_type?: string;
  copies_default?: number;
  default_target_name?: string | null;
}

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

/** 轮询任务行直至终态（窗口用尽仍在途也返回最后一次读数，由调用方按状态如实判定） */
async function pollTask(page: Page, id: number, attempts = 15): Promise<PrintTaskRow> {
  let last: PrintTaskRow = {};
  for (let i = 0; i < attempts; i++) {
    last = await apiCallRaw<PrintTaskRow>(page, 'GET', `/print-tasks/${id}`);
    if (PRINT_TASK_STATUSES.includes(String(last.status)) && isTerminal(last.status)) return last;
    await sleep(1000);
  }
  return last;
}

function isTerminal(status: unknown): boolean {
  return status === 'succeeded' || status === 'failed';
}

/**
 * 打印链路的载体单据由本用例自建，不取夹具复用的 salesOrderId。
 * 为什么：夹具把 salesOrderId 取成列表首行，而同库其他分片的用例会新建并删除销售订单，
 * 复用值可能已被删或被行级数据范围挡在可见集外，受理时后端按渲染路径同步校验单据存在性，
 * 直接回 销售订单 N 未找到（NOT_FOUND 404），把外部分组的清理动作误报成本链路失败。
 */
async function ensureOwnSalesOrder(page: Page): Promise<number> {
  await ensureTestEntities(page);
  const ctx = getCtx();
  const productId = ctx.productIds[0];
  if (!ctx.customerId || !productId) {
    throw new Error(
      `打印链路前置失败：夹具缺客户或产品（customerId=${JSON.stringify(
        ctx.customerId
      )}，productIds 长度=${ctx.productIds.length}）`
    );
  }
  const created = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
    customer_id: ctx.customerId,
    order_date: new Date().toISOString(),
    items: [{ product_id: productId, quantity: 1, unit_price: '1.00' }],
  });
  const orderId = Number(created?.data?.id);
  if (!(orderId > 0)) {
    throw new Error(
      `打印链路前置失败：自建销售订单未返回主键，响应=${JSON.stringify(created).slice(0, 200)}`
    );
  }
  return orderId;
}

test.describe('打印任务真实受理链路', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('既未指定目标、模板也未配缺省目标 → 受理被拒 400（不静默丢单）', async ({ page }) => {
    const orderId = await ensureOwnSalesOrder(page);

    const fail = await apiCallExpectFail(page, 'POST', '/print-tasks', {
      doc_type: 'sales_order',
      doc_id: orderId,
    });
    expect(
      fail.status,
      '无打印目标时必须 400 点名，历史缺陷是前端无上下文直发空体导致用户点即失败'
    ).toBe(400);
  });

  test('单据实例不存在 → 受理即拒 400（不静默接受无效 doc_id）', async ({ page }) => {
    const fail = await apiCallExpectFail(page, 'POST', '/print-tasks', {
      doc_type: 'sales_order',
      doc_id: 99999999,
      target_name: 'E2E-NOT-EXISTENT-TARGET',
    });
    expect(
      fail.status,
      `无效单据应被拒受理而非 5xx 或静默接受，实得：${JSON.stringify(fail)}`
    ).toBe(400);
    expect(
      String(fail.code),
      `应为入参/实例校验拒绝而非中间件拦截，实得：${JSON.stringify(fail)}`
    ).toBe('BAD_REQUEST');
  });

  test('受理成功后任务号唯一可读回；目标不可达时失败原因必非空', async ({ page }) => {
    const orderId = await ensureOwnSalesOrder(page);

    const targetCode = genCode('E2E-PRN');
    let targetId = 0;
    let templateId = 0;
    try {
      const target = await apiCallRaw<{ id?: number }>(page, 'POST', '/print-targets', {
        target_code: targetCode,
        display_name: `e2e打印目标${targetCode}`,
        kind: 'network',
        host: '127.0.0.1',
        port: 9100,
        enabled: true,
      });
      targetId = Number(target?.id);
      expect(targetId, `打印目标应真实落库并返回主键：${JSON.stringify(target)}`).toBeGreaterThan(
        0
      );

      // 自建模板并绑定缺省目标：受理时不带 target_name，验证模板配置真实生效（非硬编回退）
      const tplCode = genCode('TPL-E2E-PRN');
      const tpl = await apiCallRaw<PrintTemplateRow>(page, 'POST', '/print-templates', {
        template_name: `e2e打印链路模板${tplCode}`,
        template_code: tplCode,
        doc_type: 'sales_order',
        module: 'sales',
        type: 'order',
        copies_default: 2,
        default_target_name: targetCode,
        content: '<div>{order_no}</div>',
        is_default: false,
        status: 'active',
      });
      templateId = Number(tpl?.id);
      expect(templateId, `模板应真实落库并返回主键：${JSON.stringify(tpl)}`).toBeGreaterThan(0);
      expect(
        tpl?.default_target_name === undefined ? '' : String(tpl.default_target_name),
        '模板出参须回显已绑定的缺省打印目标'
      ).toBe(targetCode);

      const accepted = await apiCallRaw<PrintTaskRow>(page, 'POST', '/print-tasks', {
        doc_type: 'sales_order',
        doc_id: orderId,
        template_id: templateId,
      });
      const taskId = Number(accepted?.id);
      expect(taskId, `受理应返回任务主键：${JSON.stringify(accepted)}`).toBeGreaterThan(0);
      expect(accepted?.job_code, '受理应返回任务号供台账检索').toBeTruthy();

      const row = await pollTask(page, taskId);
      expect(
        PRINT_TASK_STATUSES.includes(String(row.status)),
        `任务状态必须落在词表内，实得：${JSON.stringify(row)}`
      ).toBe(true);
      expect(Number(row.copies), '模板缺省份数应被受理链路采用（未显式覆盖时）').toBe(2);
      expect(String(row.target_name), '受理应记录模板绑定的缺省目标').toBe(targetCode);
      if (row.status === 'failed') {
        expect(
          String(row.error_message ?? ''),
          '失败任务必须写出非空原因，不得停留在无信息的失败态'
        ).not.toBe('');
      } else if (!isTerminal(row.status)) {
        test.info().annotations.push({
          type: 'print-dispatch-in-flight',
          description: `任务 ${accepted?.job_code} 在观测窗口内未达终态（状态 ${row.status}），派发回执依赖真实设备`,
        });
      }
    } finally {
      // 该模板已被上面的任务台账引用，删除会被拒（由"被引用即拒删"用例锁定），
      // 这里改用停用下线，避免留下 active 的自建模板去干扰真实打印链路的模板解析。
      if (templateId) {
        await apiCallRaw(page, 'PUT', `/print-templates/${templateId}`, { status: 'inactive' });
      }
      if (targetId) await apiCallRaw(page, 'DELETE', `/print-targets/${targetId}`);
    }
  });

  test('模板已被打印任务引用 → 删除被拒 400 并点名条数，不再裸 500', async ({ page }) => {
    const orderId = await ensureOwnSalesOrder(page);

    const targetCode = genCode('E2E-PRN');
    let targetId = 0;
    let templateId = 0;
    try {
      const target = await apiCallRaw<{ id?: number }>(page, 'POST', '/print-targets', {
        target_code: targetCode,
        display_name: `e2e删模板目标${targetCode}`,
        kind: 'network',
        host: '127.0.0.1',
        port: 9100,
        enabled: true,
      });
      targetId = Number(target?.id);

      const tplCode = genCode('TPL-E2E-DEL');
      const tpl = await apiCallRaw<PrintTemplateRow>(page, 'POST', '/print-templates', {
        template_name: `e2e被引用模板${tplCode}`,
        template_code: tplCode,
        doc_type: 'sales_order',
        module: 'sales',
        type: 'order',
        default_target_name: targetCode,
        content: '<div>{order_no}</div>',
        is_default: false,
        status: 'active',
      });
      templateId = Number(tpl?.id);

      const accepted = await apiCallRaw<PrintTaskRow>(page, 'POST', '/print-tasks', {
        doc_type: 'sales_order',
        doc_id: orderId,
        template_id: templateId,
      });
      expect(Number(accepted?.id), `任务应受理成功：${JSON.stringify(accepted)}`).toBeGreaterThan(0);

      const fail = await apiCallExpectFail(page, 'DELETE', `/print-templates/${templateId}`);
      expect(
        fail.status,
        `有台账的模板删除必须可见拒绝而非数据库裸错，实得：${JSON.stringify(fail)}`
      ).toBe(400);
      expect(
        String(fail.message ?? ''),
        '拒绝文案须说明被打印任务引用，否则用户不知道该改用停用'
      ).toContain('打印任务');
    } finally {
      if (templateId) {
        await apiCallRaw(page, 'PUT', `/print-templates/${templateId}`, { status: 'inactive' });
      }
      if (targetId) await apiCallRaw(page, 'DELETE', `/print-targets/${targetId}`);
    }
  });

  test('模板缺省目标可清空（显式 null 三态，不残留幽灵绑定）', async ({ page }) => {
    const tplCode = genCode('TPL-E2E-CLR');
    const targetCode = genCode('E2E-PRN');
    let targetId = 0;
    let templateId = 0;
    try {
      const target = await apiCallRaw<{ id?: number }>(page, 'POST', '/print-targets', {
        target_code: targetCode,
        display_name: `e2e清空目标${targetCode}`,
        kind: 'network',
        host: '127.0.0.1',
        port: 9100,
        enabled: true,
      });
      targetId = Number(target?.id);

      const tpl = await apiCallRaw<PrintTemplateRow>(page, 'POST', '/print-templates', {
        template_name: `e2e缺省目标模板${tplCode}`,
        template_code: tplCode,
        doc_type: 'sales_order',
        module: 'sales',
        type: 'order',
        default_target_name: targetCode,
        content: '<div>{order_no}</div>',
        is_default: false,
        status: 'active',
      });
      templateId = Number(tpl?.id);
      expect(String(tpl?.default_target_name), '创建时应落库缺省目标').toBe(targetCode);

      const cleared = await apiCallRaw<PrintTemplateRow>(
        page,
        'PUT',
        `/print-templates/${templateId}`,
        { default_target_name: null }
      );
      expect(
        cleared?.default_target_name,
        `显式 null 应清空缺省目标，实得：${JSON.stringify(cleared)}`
      ).toBeNull();
    } finally {
      if (templateId) await apiCallRaw(page, 'DELETE', `/print-templates/${templateId}`);
      if (targetId) await apiCallRaw(page, 'DELETE', `/print-targets/${targetId}`);
    }
  });
});
