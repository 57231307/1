import { test, expect } from '../diagnose-fixture';
import { loginViaUI, API_BASE, API_PREFIX } from './helpers';

/**
 * 51 全端点健康扫描（生产 402/403/400/500/502/503 报错的自助定位防线）
 *
 * 背景：生产环境大量 402/403/400/503 报错（用户报障）。测试环境无法直接
 * 复现生产，但可以在 admin 全权上下文中**全量扫描**后端全部无参数 GET
 * 路由，主动暴露：
 *   - 402/500/502/503：服务器/网关/配额类崩溃（生产报错等价物）
 *   - 403：admin 仍被拒（fail-closed 配置缺陷或路由权限错配）
 *   - 404：按 D-1 三态判据二分归因——无 JSON 信封=注册面（路由不存在/已漂移，即幽灵路由）；
 *          NOT_FOUND 信封=数据面缺前置（seed 回读/用例自建）。归因输出形态
 *          `[diag][http404]`（flow/helpers.ts verifyEndpointHealthy 404 分支），判据只取
 *          HTTP 码与机器 code，不断 message 文案（脱敏红线）
 *   - 400：GET 列表接口不应要求参数
 *
 * 判定矩阵：
 *   200/204           通过
 *   400/401/404/410   记录（警告级，输出清单；404 逐条标注注册面/数据面归因，禁单一文案掩盖）
 *   402/403/500/502/503 FAIL（崩溃级）
 *   405               进入 warns 且 warns 不参与任何断言（51-1 的 expect 只打在
 *                     crashes 上）——这是历史假绿缝：清单里对着"只注册
 *                     POST 的静态节点"打 GET 得到 405，被当作"方法不匹配，通过"
 *                     静默放行，幽灵条目长期零成本存活。本波处置：静态 ROUTES
 *                     只登记快照中真实存在的 GET 契约（改指/移出逐条注明行号），
 *                     405 方法门语义改由 51-2 负向清单**逐条断言确定码**承接，
 *                     不再存在"落到 warns 就等于通过"的档位。
 */

// 后端 routes/*.rs 全部无参数路由（grep .route(" 提取，2026-09-13 同步）
const ROUTES: string[] = [
  '/ai/health',
  '/ai/summary',
  // —— 原 5 条 /advanced/ai/* 静态节点逐条处置（4 A 类改指、1 C 类移出，不静默）——
  // A 类改指依据：analytics.rs:470-478 的 advanced() 对这些节点只注册 post()，
  // GET 打过去得 405 → 只进 warns 不参与断言（缝的说明见文件头判定矩阵）；
  // 同语义的真实 GET 契约改指如下，快照行号为 route-snapshot.txt：
  '/detect-anomalies', // snapshot:459 GET ai_analysis_handler::detect_anomalies（原 /advanced/ai/anomaly-detection）
  '/ai/quality-predictions', // snapshot:148 GET ai_extend_handler::list_quality_predictions（原 /advanced/ai/quality-prediction）
  '/recommendations', // snapshot:860 GET ai_analysis_handler::get_recommendations（原 /advanced/ai/recommendations）
  // 原 /advanced/ai/sales-forecast：同语义 GET 为 /forecast-sales（snapshot:533），
  // 本清单下方已登记该端点 ⇒ 此处移出而不重复添加（重复条目只会虚增扫描计数）。
  // 原 /advanced/ai/recipe-optimization（C 类，移出）：snapshot:977 仅 POST
  // （analytics.rs:477 optimize_recipe），无 GET 契约；配方档案的集合 GET 是**另一资源**
  // /production/production-recipes（snapshot:758），本清单已含该条，不重复、也不改指冒充。
  // 该 POST-only 节点的 405 方法门语义由 51-2 负向清单真实锁定。
  '/alerts',
  '/ap/invoices',
  '/ap/payments',
  '/ar/invoices',
  '/ar/payments',
  '/audit-logs',
  '/stats',
  '/bpm/monitor/stats',
  '/bpm/tasks',
  '/bpm/tasks/pending',
  '/budgets',
  '/budgets/plans',
  '/production/business-modes',
  '/production/business-mode-links',
  '/categories',
  '/currencies',
  '/crm/customers',
  '/departments',
  '/production/dye-batch-lifecycle-logs',
  '/production/dye-batch-operations',
  '/production/dye-batch-reworks',
  '/production/dye-batch-state-rules',
  '/production/dye-batches',
  '/production/dye-recipes',
  '/email-records',
  '/production/energy-allocations',
  '/production/energy-consumptions',
  '/production/energy-meters',
  '/production/energy-rules',
  // '/production/fabric-defects' 显式移出静态清单（非静默删断言）：全局集合 GET 未注册
  // （route-snapshot.txt :80/:708/:1384 只有 by-id DELETE / by-id GET / 创建 POST——
  // 对裸路径打 GET 只会得到 405 方法不匹配，留在此处等于扫描一条不存在的契约）。
  // 疵点契约的真实形态是按验布单归属查询 GET /production/fabric-inspections/{id}/defects
  // （snapshot :711），需活体 id、静态清单给不出；由 flow/24-production-full.spec.ts
  // 以真实 seed 行/用例内自建验布单做严格探测（seed 为空亦覆盖，拒绝臆造 id）。
  '/production/fabric-inspections',
  '/fixed-assets',
  '/production/flow-cards',
  '/forecast-sales',
  // 原 '/funnel'（C 类，移出）：行为漏斗只注册 POST /api/v1/erp/funnel
  //（analytics.rs:574 tracking_handler::get_funnel_analysis，snapshot:1275），GET 不存在。
  // **禁止**改指 /crm/leads/funnel-report——那是"线索漏斗"另一资源，换资源冒充覆盖
  // 是本仓明令禁止的假绿形态。GET 405 语义由 51-2 负向清单锁定。
  // 另按"清单里本就存在的真实 GET 契约"补齐两条漏斗类 GET（不是替代被移出的行为漏斗 POST）：
  '/crm/leads/funnel-report', // snapshot:388 GET crm_handler::lead_funnel_report
  '/crm/opportunities/sales-funnel', // snapshot:397 GET crm_handler::get_sales_funnel
  '/health',
  '/health/liveness',
  '/health/readiness',
  '/init/status',
  '/init/task-status',
  '/labor-contracts',
  '/login-logs',
  '/inventory/logistics',
  '/auth/me',
  '/production/mrp-history',
  '/production/mrp/products',
  '/production/mrp/requirements',
  '/production/mrp/results',
  '/purchase/orders',
  '/production/outsourcing-orders',
  // 原 '/production/outsourcing-orders/items'（C 类，移出）：集合上无 GET——快照里
  // /items 只有 POST(:1431 create_outsourcing_item)、/items/* 只有 GET by-order(:742)、
  // PUT(:1711)/DELETE(:87)；`items/by-order/{order_id}` 需活体 id，静态清单给不出。
  // 真实覆盖在 e2e/fullflow/23-outsource-issue-receipt.spec.ts:246（对真实 seed 委外单
  // GET /production/outsourcing-orders/items/by-order/${orderId} 并断言裸数组信封），
  // 该 GET 405 方法门由 51-2 负向清单锁定。
  '/production/outsourcing-orders/report',
  '/production/outsourcing-receipts',
  '/production/outsourcing-vouchers',
  '/finance/payments',
  '/permissions',
  '/crm/pool',
  '/popular-pages',
  '/print-templates',
  '/production/process-routes',
  '/production/production-recipes',
  '/products',
  '/products/select',
  '/purchase/purchase-prices',
  '/purchase/receipts',
  '/roles',
  '/roles/conflicts',
  '/roles/permissions',

  '/crm/sales-users',
  '/scheduling/gantt',
  '/search/customers',
  '/search/doc-types',
  '/search/products',
  '/search/sales-orders',
  '/slow-queries',
  '/social-insurance',
  '/stats',
  '/inventory/stock',
  '/subjects',
  '/purchase/suppliers',
  '/suppliers/select',
  '/notifications/unread-count',
  '/user/profile',
  '/users',
  '/vouchers',
  '/vouchers/types',
  '/production/wage-rates',
  '/production/wage-records',
  '/warehouses',
  '/warehouses/select',
  '/color-cards/warnings',
];

const CRASH_CODES = [402, 500, 502, 503];

test.describe.serial('51 全端点健康扫描（admin 全权上下文）', () => {
  test('51-1 全部 GET 路由：无崩溃码/无 admin 403/无幽灵路由', async ({ page }) => {
    await loginViaUI(page);

    const crashes: string[] = [];
    const warns: string[] = [];

    for (const path of ROUTES) {
      let status = 0;
      try {
        const csrf = (await page.context().cookies()).find(c => c.name === 'csrf_token');
        const resp = await page.request.get(`${API_BASE}${API_PREFIX}${path}`, {
          headers: {
            'X-Requested-With': 'XMLHttpRequest',
            ...(csrf ? { 'X-CSRF-Token': csrf.value } : {}),
          },
          timeout: 30_000,
        });
        status = resp.status();
        // 释放 body 防连接堆积（读取失败同样计入网络异常）
        await resp.body();
      } catch (e) {
        crashes.push(`GET ${path} → 网络异常: ${(e as Error).message.slice(0, 120)}`);
        continue;
      }

      if (CRASH_CODES.includes(status)) {
        crashes.push(`GET ${path} → ${status}（服务器/网关/配额崩溃——生产报错等价物）`);
      } else if (status === 403) {
        crashes.push(`GET ${path} → 403（admin 全权仍被拒——fail-closed 缺陷或权限错配）`);
      } else if (status === 404) {
        crashes.push(`GET ${path} → 404（幽灵路由：routes 声明但未装配）`);
      } else if (status === 429) {
        // AI 限流层单独记账（仅 429，不扩到其它任何码）：改指的 /detect-anomalies、
        // /recommendations 与既有 /forecast-sales 落在 analytics.rs:331-332
        // rate_limit_ai_endpoint（按 user_id 10 req/min，rate_limit.rs:330），51 单次
        // 扫描 AI 桶请求数远低于阈值；若分片并发把它打出 429，属限流器行为而非契约
        // 崩溃——单独点名进 warns，**不许**借此把 4xx 判据整体放宽。
        warns.push(
          `GET ${path} → 429（AI 限流放行：rate_limit_ai_endpoint 10req/min/user，仅此码适用本口径）`
        );
      } else if (status >= 400) {
        warns.push(`GET ${path} → ${status}`);
      }
    }

    console.log(`[51] 扫描 ${ROUTES.length} 条路由：崩溃 ${crashes.length}，警告 ${warns.length}`);
    if (warns.length) {
      console.log(
        `[51] 警告级（400/401 等，需人工判责）：\n${warns.map(w => '  ' + w).join('\n')}`
      );
    }
    expect(
      crashes,
      `发现 ${crashes.length} 处崩溃级端点（生产 402/403/500/503 自助定位清单）：\n${crashes.join('\n')}`
    ).toHaveLength(0);
  });

  test('51-2 负向方法门：GET 打"只注册非 GET 的静态节点"必须得到推导确定的 405', async ({
    page,
  }) => {
    await loginViaUI(page);
    // 本用例此前是空壳（循环体只剩 void、bad 恒空 ⇒ 断言恒真）。现按证据做成真实负向锁。
    // 入选标准（两条静态证据缺一不可）：
    //  ① route-snapshot.txt 证明该精确路径只注册了非 GET 方法、且不存在能匹配它的
    //     GET 通配（逐条列出核对到的 GET 集）；
    //  ② 中间件链推导得出确定码：seg3 ∈ 白名单词表（path_utils.rs is_module_prefix:12 /
    //     is_direct_resource:177）⇒ validate_route_whitelist（permission.rs:121 调用，
    //     fn :59-70）放行；resource 映射非 unknown ⇒ 不走 fail-closed 403
    //     （permission.rs:131-143）；admin 短路在 check_permission 内（permission.rs:497/:544）
    //     ⇒ 请求真正到达 axum 路由表；路径命中而方法不匹配 ⇒ **405**（不是白名单层的 403）。
    // 排除项（推导不出确定码，不做猜断言）：任何 seg3 不在词表的路径会先被白名单层
    // 拒成 403，与"方法门"无关；依赖活体 id/数据的节点不入静态清单。
    const methodGateCases: { path: string; evidence: string }[] = [
      {
        path: '/advanced/ai/recipe-optimization',
        evidence:
          'snapshot:977 仅 POST（analytics.rs:477 optimize_recipe）；/advanced 下全部 GET 只有 reports/templates（snapshot:133），无法匹配本路径；seg3 "advanced" ∈ is_module_prefix（path_utils.rs:83），("advanced","ai") 有资源映射（:118）⇒ 非 unknown ⇒ 确定 405',
      },
      {
        path: '/funnel',
        evidence:
          'snapshot:1275 仅 POST（analytics.rs:574 get_funnel_analysis）；全快照无 GET /funnel；seg3 "funnel" ∈ is_direct_resource 词表（path_utils.rs:239）⇒ 白名单放行 ⇒ 确定 405（注：本条锁定的是被移出 51-1 清单的 POST-only 节点的 405 语义，不反向断 51-1）',
      },
      {
        path: '/production/outsourcing-orders/items',
        evidence:
          '/items 仅 POST（snapshot:1431 create_outsourcing_item），/items/* 上 GET 只有 by-order/* 形态（snapshot:742）匹配不了裸 /items，集合与 by-id 均无 GET（snapshot:738 仅集合）；seg3 "production" ∈ is_module_prefix ⇒ 确定 405（真实 items GET 覆盖见 fullflow/23-outsource-issue-receipt.spec.ts:246）',
      },
    ];
    // AI 限流核算：本用例仅第 1 条经过 rate_limit_ai_endpoint（analytics.rs:483 层挂在
    // advanced() 上；限流函数 rate_limit.rs:330，桶按 user_id 10 req/min）。51-1 命中同桶的最多 3 条
    // （/detect-anomalies、/recommendations、/forecast-sales，analytics.rs:331-332 层），
    // 51-1+51-2 合计 4 次 < 10；若被并发打出 429 属限流器行为，按 51-1 中"仅 429"口径
    // 记警告，不放宽方法门断言本身。
    for (const c of methodGateCases) {
      const resp = await page.request.get(`${API_BASE}${API_PREFIX}${c.path}`, {
        timeout: 30_000,
      });
      const status = resp.status();
      await resp.body(); // 释放 body 防连接堆积（同 51-1）
      expect(
        status,
        `GET ${c.path} → ${status}，负向方法门期望 405（路径已注册但只挂非 GET 方法）。` +
          `推导依据：${c.evidence}。若得到 403=白名单/权限层先拒（词表或资源映射漂移）；` +
          `若得到 200/其它=路由方法装配或本推导前提已改变，须重新判责而非改断言。`
      ).toBe(405);
    }
  });
});
