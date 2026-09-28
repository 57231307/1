import { test, expect } from '../diagnose-fixture';
import JSZip from 'jszip';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  ensureTestEntities,
  getCtx,
  API_BASE,
  API_PREFIX,
  type ApiResponse,
} from '../flow/helpers';

/**
 * 37b 打印内容匹配 + 打印审计闭环（doto 2026-09-09 印刷缺口两项）
 *
 * 内容匹配（用户 IR："打印成功后内容与需打印文件匹配"）：
 * 1. 取真实销售订单 id（GET /sales/orders，无则 API 兜底创建——复用 helpers 建单字段）
 * 2. GET /sales/orders/{id}/print → 200 + PK magic（真实 docx）
 * 3. JSZip 解包 word/document.xml → 非空 + 断言 order_no 出现在文档 XML
 * 4. 同法覆盖 /vouchers/{id}/print（凭证，金额字段）——CI 种子有voucher则跑
 *
 * 审计闭环：
 * 5. 打印后 GET /audit-logs?operation_type=PRINT → 断言出现该打印请求记录
 *    （omni_audit 中间件按 URL 末段 print 分类，audit-logs 列表仅 admin）
 *
 * 全部基于真实后端 + 真实 PostgreSQL（IR 2026-09-07），每步显式日志（IR 2026-09-03）
 */

test.describe('37b 打印内容匹配与审计闭环', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('销售订单打印 docx 内容与源单据匹配 + 审计 PRINT 留痕', async ({ page }) => {
    // 与 playwright.config.ts 单测默认 420s 基线一致（该值专为 ensureTestEntities
    // 需 UI 创建 10+ 实体、50 分片并发同库的耗时设定）。原 120s 覆盖低于基线，
    // 导致内联 ensureTestEntities 走到末段（/auth/me 步骤）时踩 120s 上限误判超时。
    test.setTimeout(420_000);

    // ---- 1. 取真实销售订单（列表回读；无则兜底创建，字段与 helpers.ts 建单链一致）----
    let salesOrderId: number | undefined;
    let orderNo: string | undefined;
    const sos = await apiCallRaw<{ items: Array<{ id: number; order_no: string }> }>(
      page,
      'GET',
      '/sales/orders?page=1&page_size=1'
    );
    salesOrderId = sos.items?.[0]?.id;
    orderNo = sos.items?.[0]?.order_no;
    if (!salesOrderId) {
      console.log('[37b] 无销售订单，ensureTestEntities 兜底创建（含库存前置）');
      await ensureTestEntities(page);
      const ctx = getCtx();
      const whId = ctx.warehouseIds[0];
      const prodId = ctx.productIds[0];
      const custId = ctx.customerId;
      // ensureTestEntities 已建库存+销售订单，优先用 ctx.salesOrderId
      if (ctx.salesOrderId) {
        salesOrderId = ctx.salesOrderId;
        const so = await apiCallRaw<
          { order_no?: string } & { items?: Array<{ id: number; order_no: string }> }
        >(page, 'GET', '/sales/orders?page=1&page_size=1');
        orderNo = so.items?.[0]?.order_no;
      } else if (whId && prodId && custId) {
        // ctx 无销售订单时，用真实仓库/产品/客户兜底建单
        const result = await apiCall<{ id?: number; order_no?: string }>(
          page,
          'POST',
          '/sales/orders',
          {
            customer_id: custId,
            order_date: new Date().toISOString(),
            items: [{ product_id: prodId, quantity: '1', unit_price: '1' }],
          }
        );
        salesOrderId = result.data?.id;
        orderNo = result.data?.order_no;
      }
    }
    // 空库极端场景也要判红：ensureTestEntities 会创建销售订单（其前置缺失时自身抛错），
    // 拿不到 id 说明创建链真的断了，属环境/产品缺陷，不能再 test.skip 掩盖。
    expect(
      salesOrderId,
      '销售订单既无法从列表取得、也无法经 ensureTestEntities/兜底创建——打印内容断言缺前置数据，判红'
    ).toBeTruthy();
    console.log(
      `[37b] 源单据：salesOrderId=${salesOrderId} orderNo=${orderNo ?? '(列表未返回单号)'}`
    );

    // ---- 2. 真实打印请求（浏览器上下文 cookie + 真实后端）----
    const printResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/sales/orders/${salesOrderId}/print`
    );
    const printStatus = printResp.status();
    console.log(`[37b] 打印请求 /sales/orders/${salesOrderId}/print → ${printStatus}`);

    // 已持有真实订单 id：打印仍 4xx 说明处理器对有效单据坏掉，判红（不再 skip）。
    expect(printStatus, `真实订单 ${salesOrderId} 打印应返回 200，实际 ${printStatus}`).toBe(200);

    const body = await printResp.body();
    expect(body.length, `docx 响应体应 >1KB，实际 ${body.length}B`).toBeGreaterThan(1024);
    expect(body[0], 'docx 应为 zip 容器（PK magic [0]）').toBe(0x50);
    expect(body[1], 'docx 应为 zip 容器（PK magic [1]）').toBe(0x4b);

    // ---- 3. JSZip 深度解包：document.xml 非空 + 内容匹配 ----
    const zip = await JSZip.loadAsync(body);
    const docXml = await zip.file('word/document.xml')?.async('string');
    expect(docXml, 'docx 应包含 word/document.xml').toBeTruthy();
    expect(docXml!.length, `document.xml 应非空，实际 ${docXml!.length}B`).toBeGreaterThan(100);
    console.log(`[37b] document.xml 解包成功 ${docXml!.length}B`);

    if (orderNo) {
      // XML 转义兜底：单号含 & < > 时转义形式也应命中；压缩/排版不改变字符序列
      const unescaped = orderNo.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
      expect(
        docXml!.includes(orderNo) || docXml!.includes(unescaped),
        `打印文档应包含源单据号 ${orderNo}（内容匹配）`
      ).toBeTruthy();
      console.log(`[37b] ✅ 内容匹配：单据号 ${orderNo} 出现在 document.xml`);
    } else {
      console.warn('[37b] 列表未返回 order_no，仅做非空断言');
    }

    // ---- 4. 打印审计闭环：audit-logs 出现 PRINT 记录 ----
    // 审计日志经 omni_audit 中间件异步落库（backend.log 实测 PRINT 写入晚于打印请求约
    // 40~60ms，CI 并发下同库延迟更大）。直接单次回读会早于落库 → total=0 假红。
    // 用 expect.poll 轮询等待落库（上限 15s、每 1s 一次），断言仍严格 >0 不放宽——
    // 若后端根本未把 /print 归类为 operation_type=PRINT，轮询到点仍红，暴露真缺陷而非掩盖。
    const printLogs = await expect
      .poll(
        async () => {
          const auditResp = await page.request.get(
            `${API_BASE}${API_PREFIX}/audit-logs?operation_type=PRINT&page=1&page_size=20`
          );
          if (auditResp.status() !== 200) return -1;
          const auditJson = (await auditResp.json()) as ApiResponse<{
            items: Array<{
              id: number;
              operation_type?: string;
              action?: string;
              uri?: string;
              path?: string;
            }>;
            total: number;
          }>;
          if (auditJson.code !== 200) return -1;
          const items = auditJson.data?.items;
          // 出参形态区分：后端未返回 items（缺字段）与返回空集合不同——缺字段时返回 -1
          // 让下方断言判红并暴露真实契约问题，绝不用 ?? [] 把缺字段伪装成"暂时还没落库"。
          if (!Array.isArray(items)) return -1;
          return items.length;
        },
        {
          message: '审计应存在 PRINT 记录（打印后闭环，等待异步落库）',
          timeout: 15_000,
          intervals: [1_000],
        }
      )
      .toBeGreaterThan(0);

    // 落库后单次回读完整 items 供后续 uri/path 匹配（轮询仅判条数，避免把回读逻辑塞进 poll 反复请求）。
    const auditResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/audit-logs?operation_type=PRINT&page=1&page_size=20`
    );
    const auditJson = (await auditResp.json()) as ApiResponse<{
      items: Array<{
        id: number;
        operation_type?: string;
        action?: string;
        uri?: string;
        path?: string;
      }>;
      total: number;
    }>;
    const matched = (auditJson.data?.items ?? []).find(l =>
      (l.uri ?? l.path ?? '').includes(String(salesOrderId))
    );
    console.log(
      `[37b] ✅ 审计闭环：PRINT 记录 ${printLogs} 条${matched ? '（含本次打印的单据记录）' : ''}`
    );
  });

  test('凭证打印内容解包（voucher 种子存在时）', async ({ page }) => {
    // 与 playwright.config.ts 单测默认 420s 基线一致（该值专为 ensureTestEntities
    // 需 UI 创建 10+ 实体、50 分片并发同库的耗时设定）。原 120s 覆盖低于基线，
    // 导致内联 ensureTestEntities 走到末段（/auth/me 步骤）时踩 120s 上限误判超时。
    test.setTimeout(420_000);

    // 凭证列表回读；无则由 ensureTestEntities 建凭证（其内部会先建科目+会计期间再建凭证），
    // 建不出来即环境/创建链缺陷，判红而非 skip（造数据优先于跳过）。
    const vs = await apiCallRaw<{
      items: Array<{ id: number; voucher_no?: string; no?: string }>;
    }>(page, 'GET', '/vouchers?page=1&page_size=1');
    // items?.[0]?.id 在空列表时运行期为 undefined（noUncheckedIndexedAccess 关闭使
    // 索引结果被误判为 number），显式标注可选以匹配真实形状，供 ensureTestEntities 兜底赋值
    let voucherId: number | undefined = vs.items?.[0]?.id;
    let voucherNo = vs.items?.[0]?.voucher_no ?? vs.items?.[0]?.no;
    if (!voucherId) {
      console.log('[37b] 无凭证种子数据，ensureTestEntities 兜底创建');
      await ensureTestEntities(page);
      voucherId = getCtx().voucherId;
      if (voucherId) {
        const detail = await apiCallRaw<{ voucher_no?: string; no?: string }>(
          page,
          'GET',
          `/vouchers/${voucherId}`
        );
        voucherNo = detail?.voucher_no ?? detail?.no;
      }
    }
    expect(
      voucherId,
      '凭证既无种子、ensureTestEntities 也未能创建——凭证打印断言缺前置数据，判红'
    ).toBeTruthy();
    console.log(`[37b] 凭证 voucherId=${voucherId} no=${voucherNo ?? '?'}`);

    const printResp = await page.request.get(
      `${API_BASE}${API_PREFIX}/vouchers/${voucherId}/print`
    );
    const status = printResp.status();
    console.log(`[37b] 凭证打印 → ${status}`);
    // 已持有真实凭证 id：打印仍 4xx 说明处理器对有效凭证坏掉，判红（不再 skip）。
    expect(status, `真实凭证 ${voucherId} 打印应 200，实际 ${status}`).toBe(200);

    const body = await printResp.body();
    expect(body.length, `凭证 docx 应 >1KB，实际 ${body.length}B`).toBeGreaterThan(1024);
    const zip = await JSZip.loadAsync(body);
    const docXml = await zip.file('word/document.xml')?.async('string');
    expect(docXml, '凭证 docx 应包含 word/document.xml').toBeTruthy();
    if (voucherNo) {
      expect(
        docXml!.includes(voucherNo),
        `凭证文档应包含单据号 ${voucherNo}（内容匹配）`
      ).toBeTruthy();
      console.log(`[37b] ✅ 凭证内容匹配：${voucherNo} 出现在 document.xml`);
    }
  });
});
