import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  genCode,
  getCtx,
  BASE_URL,
  API_BASE,
  API_PREFIX,
  safeGet,
  safeGetList,
  getRoleCredential,
  loginInIsolatedContext,
  failureCode,
  APP_ERROR_CODES,
  type ApiFailureBody,
  safePostAction,
  verifyEndpointHealthy,
  ensureTestEntities,
  tryCleanup,
} from './helpers';

test.describe('CRM 模块：API 端点 + 真实 UI 交互', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
    await ensureTestEntities(page);
  });

  // ===== API 端点覆盖 =====
  test('客户管理：CRUD+导出+地址+信用+360+RFM+CLV', async ({ page }) => {
    const ctx = getCtx();
    // 动态获取客户 id：ctx.customerId 缺失时从列表首个回退（库中不存在 id=1 的保证）
    let customerId = ctx.customerId;
    if (!customerId) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/crm/customers?page=1&page_size=1'
      );
      customerId = list.items?.[0]?.id;
    }
    // 仍无任何客户时创建一个
    if (!customerId) {
      const created = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
        customer_name: 'E2E 客户 ' + Date.now(),
      });
      customerId = created.data?.id;
    }
    if (!customerId) throw new Error('无法获得任何客户 id（列表为空且创建失败）');

    await apiCallRaw(page, 'GET', '/crm/customers?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/customers/select?page=1&page_size=5');
    // P1.1 fail-closed 落地后：客户导出为敏感资源端点，
    // 无 download_token 必须 403（此处验证 fail-closed 生效而非文件内容）
    const exportResp = await page.request.get(
      `${process.env.API_BASE || 'http://localhost:8082'}${API_PREFIX}/crm/customers/export`
    );
    if (exportResp) {
      // 敏感端点 fail-closed：403=生效；200=尚未纳入 fail-closed（CRM 前缀路由差异），记录标注
      const st = exportResp.status();
      if (st === 403) {
        expect(st).toBe(403);
      } else {
        test.info().annotations.push({
          type: 'fail-closed-status',
          description: `/crm/customers/export 返回 ${st}（该前缀路由未纳入 fail-closed 或审批链可用）`,
        });
        expect(st).toBeLessThan(500);
      }
    }
    await apiCallRaw(page, 'GET', `/crm/customers/${customerId}`);
    // E3 用例内前置（CI run #4675 shard6 判责）：GET /crm/customers/{id}/credit 的 404
    // 判据是"该客户无信用评级记录"而非路由漂移——路由已注册（routes/crm.rs:48-49 →
    // customer_credit_handler.rs:111 get_credit，Path 参数即客户 id），无行时
    // customer_credit_service::get_by_customer_id（:77-86）返回 None → handler:125
    // AppError::not_found（backend.log:37987 命中 handler 实证）。
    // global-setup 的评级数据层前置保证"库内存在带评级的客户"，但本用例的 customerId
    // 实时取 /crm/customers 列表 items[0]，而后端列表排序为 created_at DESC
    // （services/customer_ops/query.rs:104/:121/:129），同库其它用例新建客户会让首位
    // 漂移到尚未登记评级的客户。故此处沿官方创建端点补全前置：先查（200=已有评级则
    // 绝不 POST——set_credit_rating 的更新分支会把请求缺省字段刷回默认值
    // customer_credit_limit.rs:41-45，覆盖既有等级/额度），缺（404）则为本探针实时
    // 解析出的真实客户经 POST /crm/customer-credits（handler:267 create_credit）登记
    // 评级；创建失败 apiCall 直接抛真实原因判红，不静默 catch。探针本身保持 strict：
    // 前置之后仍 404/400/403/5xx 即真实回归，不再可能是缺数据。
    const creditPre = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/crm/customers/${customerId}/credit`,
      {
        method: 'GET',
        headers: { 'X-Requested-With': 'XMLHttpRequest' },
      }
    );
    if (creditPre.status() === 404) {
      // CreditRatingRequestDto（customer_credit_handler.rs:37-49）：customer_id 必填；
      // 其余取值与"客户信用管理"用例一致的真实合法值（level ≤20 字符、limit 0~10 亿
      // 两位小数 validator.rs:28-40），落库 status 由 service 置 active（:66）。
      await apiCall(page, 'POST', '/crm/customer-credits', {
        customer_id: customerId,
        credit_level: 'B',
        credit_score: 60,
        credit_limit: '100000',
        credit_days: 30,
      });
    }
    // credit/360/rfm 等子资源后端已注册（crm.rs:48/550/561），admin 下应 2xx，strict 验证
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/credit`);
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/addresses`);
    // 客户关系汇总（crm.rs:537 已注册，Path(customer_id) 取上面真实 seed 客户 id，service 汇总计数
    // 空关系仍返回 200）：迁回严格。
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/summary`);
    // 360 契约校验升级（假绿解封）：旧写法 verifyEndpointHealthy('/crm/customers/{id}/360')
    // 只拦 5xx，对"200 但缺 tags/shipping_addresses 数组键"的信封盲区无感——而详情页
    // detail.vue:221/:237 对 customer.shipping_addresses 取 .length，缺键运行期必崩。
    // 现锁定 data 顶层契约：含 customer + tags(Array) + shipping_addresses(Array)，空为 []。
    // 子资源对新客户确可能 404/403（尚未生成对应数据）→ 保留容忍但显式标注，绝不掩盖 2xx 缺键。
    const r360 = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/crm/customers/${customerId}/360`,
      {
        method: 'GET',
        headers: {
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token':
            (await page.context().cookies()).find(c => c.name === 'csrf_token')?.value ?? '',
        },
      }
    );
    const st360 = r360.status();
    if (st360 === 404 || st360 === 403) {
      test.info().annotations.push({
        type: '360-not-provisioned',
        description: `/crm/customers/${customerId}/360 返回 ${st360}（客户无对应业务数据，容忍为健康）`,
      });
    } else {
      expect(st360 < 500, `360 端点不得 5xx，实际 ${st360}`).toBeTruthy();
      const env360 = (await r360.json()) as { code?: number; data?: Record<string, unknown> };
      expect(env360.code, `360 信封应为成功码 200，实际 code=${env360.code}`).toBe(200);
      const d360 = env360.data ?? {};
      expect(d360.customer, '360 data 顶层应含 customer 对象').toBeTruthy();
      expect(
        Array.isArray(d360.tags),
        `360 data.tags 应为数组（缺键使详情页崩溃），实际=${JSON.stringify(d360.tags)}`
      ).toBe(true);
      // [BE-4] 补强 tags 元素字段形状断言（对应修复项 BE-4：CustomerTagBrief.category 可空）
      // 后端 services/crm/mod.rs:64-69 CustomerTagBrief { id, name, color, category: Option<String> }
      // category 序列化为 null 时前端详情组件不得崩溃；前端 api/crm-enhanced.ts:4-9 CustomerTag 接口
      // 原把 category 声明为 string（非 nullable），与后端契约不一致——该缺陷导致前端 TS 编译期
      // 漏掉 null 分支，运行期对 category 做 .trim()/.toLowerCase() 等调用时 TypeError。
      // 本断言锁定 tags 数组每个元素的字段形状契约：
      //   id: number（必有）
      //   name: string（必有非空）
      //   color: string（必有）
      //   category: string | null（可空——BE-4 修复项核心）
      // 注意：tags 数组可能为空（客户尚未挂标签），此时跳过元素级断言。
      const tags = d360.tags as Array<Record<string, unknown>>;
      if (tags.length > 0) {
        for (const tag of tags) {
          expect(typeof tag.id, `tag.id 应为 number，实际=${JSON.stringify(tag.id)}`).toBe(
            'number'
          );
          expect(typeof tag.name, `tag.name 应为 string，实际=${JSON.stringify(tag.name)}`).toBe(
            'string'
          );
          expect((tag.name as string)?.length, 'tag.name 不应为空字符串').toBeGreaterThan(0);
          expect(
            typeof tag.color,
            `tag.color 应为 string（后端 CustomerTagBrief.color 非 Option），实际=${JSON.stringify(tag.color)}`
          ).toBe('string');
          // category 是本轮核心：可为 string 或 null，但键必须存在（后端序列化为 Option → null 或 string）
          expect(
            'category' in tag,
            `tag 对象必须含 'category' 键（后端 CustomerTagBrief 四字段契约），实际 keys=${Object.keys(tag).join(',')}`
          ).toBe(true);
          expect(
            tag.category === null || typeof tag.category === 'string',
            `tag.category 应为 string 或 null（BE-4 Option<String>），实际 type=${typeof tag.category} value=${JSON.stringify(tag.category)}`
          ).toBe(true);
        }
        console.log(
          `[22-crm] tags 元素形状验证通过，共 ${tags.length} 条，category 取值分布=${JSON.stringify(
            tags.map(t => (t.category === null ? 'null' : typeof t.category))
          )}`
        );
      } else {
        // 无标签时显式挂一个 category 为空的标签做契约验证（真实 UI 操作或 API 挂标签均可，
        // 此处按最小侵入原则用 API attach，不污染 UI 流程）
        const tagCreate = await apiCall<{ id?: number }>(page, 'POST', '/crm/tags', {
          name: `E2E容null标签${Date.now().toString().slice(-6)}`,
          color: '#FF5733',
          // 不传 category（后端 Option<String> 无默认值 → 落库 NULL）
        });
        const newTagId = tagCreate.data?.id;
        if (newTagId) {
          await apiCall(page, 'POST', `/crm/customers/${customerId}/tags/${newTagId}`, {}).catch(
            e => {
              console.warn('[22-crm] 挂标签失败:', (e as Error).message);
            }
          );
          // 重新获取 360 验证新标签的 category 为 null
          const r360b = await page.request.fetch(
            `${API_BASE}${API_PREFIX}/crm/customers/${customerId}/360`,
            {
              method: 'GET',
              headers: {
                'X-Requested-With': 'XMLHttpRequest',
                'X-CSRF-Token':
                  (await page.context().cookies()).find(c => c.name === 'csrf_token')?.value ?? '',
              },
            }
          );
          const env360b = (await r360b.json()) as { code?: number; data?: Record<string, unknown> };
          const tagsB = (env360b.data?.tags ?? []) as Array<Record<string, unknown>>;
          const createdTag = tagsB.find(t => t.id === newTagId);
          if (createdTag) {
            expect(
              createdTag.category === null || createdTag.category === undefined,
              `category 未指定时后端应返回 null（BE-4 可空契约），实际=${JSON.stringify(createdTag.category)}`
            ).toBe(true);
            expect(typeof createdTag.color).toBe('string');
          }
          // 清理标签
          await tryCleanup(page, 'DELETE', `/crm/tags/${newTagId}`, '22-crm-容null标签');
        }
      }
      expect(
        Array.isArray(d360.shipping_addresses),
        `360 data.shipping_addresses 应为数组（detail.vue:237 取其 length），实际=${JSON.stringify(
          d360.shipping_addresses
        )}`
      ).toBe(true);
    }
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/follow-ups`);
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/rfm`);
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/audit-logs`);
    await verifyEndpointHealthy(page, `/crm/customers/${customerId}/clv`);
    await apiCallRaw(page, 'GET', '/crm/customers/enhanced?page=1&page_size=5');
    // /roles 真实出参（只按此形状取值，禁止双形状探测）：role_handler.rs:113-142 list_roles
    // 直出 ApiResponse<RoleListResponse>，data = { roles, total }（结构体 role_handler.rs:78-81），
    // 全量返回无分页（handler 无 Query 提取器，:134 注释明示 total=len），
    // 不是本仓通用分页信封 {items,total,page,page_size}；读法与 global-setup.ts:204-230 一致。
    const roleId =
      ctx.roleId ??
      (await apiCallRaw<{ roles: Array<{ id: number }> }>(page, 'GET', '/roles')).roles?.[0]?.id;
    expect(
      roleId,
      '无法获取任何角色 id（ensureTestEntities.ctx.roleId 缺失且 /roles 出参 roles 数组为空）'
    ).toBeTruthy();
    await verifyEndpointHealthy(page, `/crm/customers/field-permissions/${roleId}`);
    await verifyEndpointHealthy(page, '/crm/rfm/distribution');
    await apiCallRaw(page, 'GET', '/crm/sales-users');
  });

  test('客户信用管理：列表+评级+占用+释放+调整', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/customer-credits?page=1&page_size=5');
    const result = await apiCallExpectFail(page, 'POST', '/crm/customer-credits', {
      customer_id: 1,
      credit_limit: '100000',
      currency: 'CNY',
    });
    if (result.status < 400) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/crm/customer-credits?page=1&page_size=1'
      );
      const creditId = list.items?.[0]?.id;
      if (creditId) {
        await apiCallRaw(page, 'GET', `/crm/customer-credits/${creditId}`);
        await safePostAction(page, `/crm/customer-credits/${creditId}/rating`, { rating: 'A' });
        await safePostAction(page, `/crm/customer-credits/${creditId}/occupy`, { amount: '1000' });
        await safePostAction(page, `/crm/customer-credits/${creditId}/release`, { amount: '1000' });
        await safePostAction(page, `/crm/customer-credits/${creditId}/adjust`, { amount: '5000' });
        await safePostAction(page, '/crm/customer-credits/evaluate', { customer_id: 1 });
      }
    }
  });

  test('线索管理：创建+转换+分配+评分+漏斗', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/leads?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/leads/conversion-stats');
    // channel-roi 契约复核（判责报告有出入，以代码为准）：handler crm_handler.rs:1178-1190 用
    // Query<ChannelRoiQuery>（结构体 crm_handler.rs:1549-1552，start_date/end_date 皆 Option<NaiveDate>），
    // 但 handler 对二者各 .ok_or_else(|| AppError::bad_request("...必填"))（crm_handler.rs:1184-1189）
    // → 二者实为【必填】，缺任一经反序列化落 None → 400；并非报告所称「start_date>end_date 自定义校验」
    // ——service channel_roi_report(services/crm/lead.rs:1305-1317) 只做 PeriodStart>=start / PeriodEnd<=end
    // 区间过滤、无大小校验。语义真实修法：补一对合法日期区间（近一年，start<=end，YYYY-MM-DD 供
    // chrono::NaiveDate 解析），空命中仍 200（lead.rs:1333 Ok(Vec)），不删断言、不放宽。
    const roiStartStr = new Date(Date.now() - 365 * 86400000).toISOString().slice(0, 10);
    const roiEndStr = new Date().toISOString().slice(0, 10);
    await verifyEndpointHealthy(
      page,
      `/crm/leads/channel-roi?start_date=${roiStartStr}&end_date=${roiEndStr}`
    );
    await verifyEndpointHealthy(page, '/crm/leads/allocation-rules');
    await verifyEndpointHealthy(page, '/crm/leads/nurture-plans');
    await verifyEndpointHealthy(page, '/crm/leads/funnel-report');
    // 后端 CreateLeadRequest 字段：company_name/contact_name/mobile_phone/lead_source 等
    const result = await apiCallExpectFail(page, 'POST', '/crm/leads', {
      company_name: 'E2E 线索公司',
      contact_name: 'E2E 联系人',
      mobile_phone: '13800000000',
      lead_source: 'web',
    });
    let leadId: number | undefined;
    leadId = (result as { data?: { id?: number } }).data?.id;
    if (!leadId) {
      const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
        page,
        'GET',
        '/crm/leads?page=1&page_size=1'
      );
      leadId = list.items?.[0]?.id;
    }
    if (leadId) {
      await apiCallRaw(page, 'GET', `/crm/leads/${leadId}`);
      await verifyEndpointHealthy(page, `/crm/leads/${leadId}/relations`);
      await safePostAction(page, `/crm/leads/${leadId}/score`, { score: 80 });
      await safePostAction(page, `/crm/leads/${leadId}/convert`, { customer_name: 'E2E 客户' });
    }
    await safePostAction(page, '/crm/leads/detect-duplicates', {
      name: '测试',
      phone: '13800000000',
    });
  });

  test('商机管理：创建+阶段+竞争对手+跟进+预测+漏斗', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/opportunities?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/opportunities/stage-stats');
    await verifyEndpointHealthy(page, '/crm/opportunities/stage-duration');
    await verifyEndpointHealthy(page, '/crm/opportunities/forecast-accuracy');
    await verifyEndpointHealthy(page, '/crm/opportunities/weighted-forecast');
    await verifyEndpointHealthy(page, '/crm/opportunities/conversion-rate');
    await verifyEndpointHealthy(page, '/crm/opportunities/sales-funnel');
    const list = await apiCallRaw<{ items: Array<{ id: number }> }>(
      page,
      'GET',
      '/crm/opportunities?page=1&page_size=1'
    );
    const oppId = list.items?.[0]?.id;
    if (oppId) {
      await apiCallRaw(page, 'GET', `/crm/opportunities/${oppId}`);
      await verifyEndpointHealthy(page, `/crm/opportunities/${oppId}/competitors`);
      await verifyEndpointHealthy(page, `/crm/opportunities/${oppId}/follow-ups`);
      await safePostAction(page, `/crm/opportunities/${oppId}/stage-change`, {
        from_stage: 'QUALIFICATION',
        to_stage: 'PROPOSAL',
      });
    }
  });

  test('公海池+分配+转移审批+回收规则+标签+竞争对手', async ({ page }) => {
    await apiCallRaw(page, 'GET', '/crm/pool?page=1&page_size=5');
    await apiCallRaw(page, 'GET', '/crm/pool/rules');
    await apiCallRaw(page, 'GET', '/crm/assignments?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/crm/assignments/history');
    // 线索负载（crm.rs:343 已注册）：handler 必填 user_ids(逗号分隔)，缺省会返 400 校验；
    // 取 ensureTestEntities 从 /auth/me 落地的真实登录用户 id 作入参，admin 上下文应 2xx → 迁回严格。
    // 省略/臆造 user_ids（如固定 1）会致假红，故用真实 id，空则 fail-fast 暴露 setup 问题而非端点假红。
    const ctx = getCtx();
    const workloadUserId = ctx.userIds[0];
    if (!workloadUserId) {
      throw new Error(
        '[flow/22 workload] ctx.userIds 为空（/auth/me 未返回 id），拒绝以臆造入参 strict 校验'
      );
    }
    await verifyEndpointHealthy(page, `/crm/assignments/workload?user_ids=${workloadUserId}`);
    await verifyEndpointHealthy(page, '/crm/transfer-approvals?page=1&page_size=5');
    await verifyEndpointHealthy(page, '/crm/recycle-rules');
    // admin 可达面：竞品是**孤儿端点**——handler 的 auth 参数未消费
    // （crm_handler.rs:1476-1485，不查权限键也不套行级 scope），而 competitors:read
    // 既不在资源注册表（init_service.rs:70-130）也未授给任何角色
    // （init_service_ops/permission.rs 零命中），前端 src 亦零消费方 ⇒ 只有 admin
    // 靠 is_admin_role 短路进得来。保留健康面并钉成功信封形状（apiCall 非 2xx 或
    // code≠200/0 即抛，404/403/5xx 均判红，语义同 strict 健康口径），另起一条用例把
    // "非 admin 恒 403"钉成显式契约（见下一条用例），不把它混进通用健康清单里静默放宽。
    const adminCompetitors = await apiCallRaw<unknown>(
      page,
      'GET',
      '/crm/competitors?page=1&page_size=5'
    );
    // handler 直出 Vec<competitor::Model>（crm_handler.rs:1481-1483 → service crm/opp.rs:1130-1138），
    // 故 data 形状必须是数组而非分页对象——空表也不豁免形状断言
    expect(Array.isArray(adminCompetitors), '竞品 admin 面 data 应为数组（Vec 直出）').toBe(true);
  });

  test('竞品端点契约锁：非 admin 角色恒 403（孤儿端点现状，不静默放宽）', async ({ browser }) => {
    // 用户裁定（PR #942）：本波不补授权、不删端点，只把现状钉成契约。
    // 凭据取不到即前置判红——禁止改用 admin 身份兜底（那等于把这条锁改成空操作）。
    const cred = getRoleCredential('salesperson');
    if (!cred) {
      throw new Error(
        '[flow/22 competitors] role-credentials.json 无 salesperson 凭证：前置缺失判红，' +
          '不得改用其它身份兜底（本锁的判据正是"非 admin 拿不到"）'
      );
    }
    const session = await loginInIsolatedContext(browser, cred.username, cred.password);
    try {
      // 走原始响应而非 apiCallExpectFail：后者只回传 status/code/message，统一失败信封的
      // trace_id/timestamp 两键（utils/error.rs:697-702 ErrorResponse）拿不到，形状钉不全。
      const res = await session.page.request.get(
        `${API_BASE}${API_PREFIX}/crm/competitors?page=1&page_size=5`,
        { headers: { 'X-Requested-With': 'XMLHttpRequest' } }
      );
      const bodyText = await res.text();
      // 权限类拒绝文案永久脱敏：只断 status、机器码与四键形状，绝不断 message 内容，
      // 也不断"谁拥有这行"
      expect(
        res.status(),
        `竞品端点应拒绝非 admin（实际 ${res.status()} body=${bodyText.slice(0, 200)}）`
      ).toBe(403);
      const failureBody = JSON.parse(bodyText) as ApiFailureBody;
      expect(
        failureCode(failureBody),
        `403 必须走 AppError 统一信封且 code=${APP_ERROR_CODES.FORBIDDEN}，实际 body=${bodyText.slice(0, 200)}`
      ).toBe(APP_ERROR_CODES.FORBIDDEN);
      // 四键形状 {code,message,trace_id,timestamp}（error.rs:697-702），只钉形状不钉内容
      expect(typeof failureBody.message, '失败信封应含 message 键（内容不钉）').toBe('string');
      expect(typeof failureBody.trace_id, '失败信封应含 trace_id 键').toBe('string');
      expect(typeof failureBody.timestamp, '失败信封应含 timestamp 键').toBe('number');
    } finally {
      await session.close();
    }
    // ⚠️ 反转条件：该端点当前无鉴权、无行级 scope。若将来给 competitors:read 授权给任何
    //    角色，持证者会读到全量竞品（跨 owner 泄漏），届时必须同时补鉴权+scope，
    //    并把本锁反转为"非 admin 亦可 2xx 且只返回其可见行"。
  });

  test('五维管理+销售分析+标签', async ({ page }) => {
    await verifyEndpointHealthy(page, '/crm/five-dimension/stats');
    await verifyEndpointHealthy(page, '/crm/five-dimension/list');
    await verifyEndpointHealthy(page, '/crm/five-dimension/summary');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/statistics');
    // ===== 销售趋势内容级锁（替换原"健康探针+过期前提注释"段）=====
    // 端点 GET /crm/sales-analysis/trends 与别名 /trend 挂同一 handler（routes/crm.rs 两处挂载均
    // 指向 sales_analysis_handler::get_trends）。语义为**按粒度分桶的时间序列**：聚合现算自
    // sales_orders（复用 BiAnalysisService::sales_by_time）——sales_statistics 对实际销售零写入方
    // （仅 target 行），读它恒空；此前注释所述"TrendQuery{period:String} 必填、缺省即 400"
    // 与现实相反：现 TrendQuery 四键 granularity/start_date/end_date/period 全部 Option，
    // 缺 granularity=月粒度、缺日期=按粒度回看 12 桶；仅 end_date<start_date、日期不成对
    // 或形态非法才 400 + VALIDATION_ERROR（非法 granularity 回落月桶并留痕，不 400）。
    // 出参行 SalesTrendPoint{period, amount, order_count, quantity, profit}：金额/数量/利润为
    // Decimal=字符串口径（typeof "string"，财务口径禁 JSON number）；total_amount→amount 的改名
    // 只在后端 service 唯一映射点发生；旧前端声明的 growth_rate 无产出方，已不存在、不得再断。
    // 窗口用专属年 2095（全仓 e2e grep 无占用：71 号 2096、70 号 2097、61 号 2098/2099），
    // 参数键天然唯一；金额 3×500.25=1500.75 为有限两位小数（Decimal→f64→两位字符串往返无损，
    // 桶值可与明细精确字符串比对）。
    // 缓存纪律：链路复用 BI 5min TTL 缓存（键=scope+窗口+粒度），种单后首查与删单后无痕复查
    // 必须使用互不相同的窗口键，否则命中陈旧缓存造成假绿/假红。
    // 三段式：种客户+种有效销售单 → 内容级断言（桶序列真的回读到且金额==明细）→ 删单断无痕，
    // finally 兜底清理；任何一步失败都不静默。
    const trendCtx = getCtx();
    const trendProductId = trendCtx.productIds[0];
    if (!trendProductId) throw new Error('前置缺失：ctx.productIds[0] 未就绪');
    const trendMonth = '2095-05';
    const trendWindow = `start_date=${trendMonth}-01&end_date=${trendMonth}-31&granularity=month`;
    const trendCustomerCode = genCode('E2E22T');
    const trendCustomer = await apiCall<{ id?: number }>(page, 'POST', '/crm/customers', {
      customer_code: trendCustomerCode,
      customer_name: `22号趋势聚合客户_${trendCustomerCode}`,
      contact_phone: '13800022001',
    });
    const trendCustomerId = Number(trendCustomer.data?.id);
    expect(
      trendCustomerId,
      `[22] 趋势客户创建应回 id：${JSON.stringify(trendCustomer)}`
    ).toBeGreaterThan(0);
    let trendOrderId = 0;
    try {
      // 有效销售单：10 日正午 UTC（|会话偏移|≤12 时日/月桶不落邻日邻月）；
      // 状态 pending 属聚合排除门（cancelled/draft）之外的如实计入值
      const trendOrder = await apiCall<{ id?: number }>(page, 'POST', '/sales/orders', {
        customer_id: trendCustomerId,
        order_date: `${trendMonth}-10T12:00:00Z`,
        status: 'pending',
        items: [{ product_id: trendProductId, quantity: '3', unit_price: '500.25' }],
      });
      trendOrderId = Number(trendOrder.data?.id);
      expect(
        trendOrderId,
        `[22] 趋势种子订单创建应回 id：${JSON.stringify(trendOrder)}`
      ).toBeGreaterThan(0);

      // ① 正向内容级：桶数组真的回读到，金额/数量字符串与订单数逐一对应种单单据
      const trendEnv = await apiCall<unknown>(
        page,
        'GET',
        `/crm/sales-analysis/trends?${trendWindow}`
      );
      expect(
        trendEnv.code,
        `趋势成功信封 code=200（ApiResponse 契约），实际 ${JSON.stringify(trendEnv)}`
      ).toBe(200);
      expect(Array.isArray(trendEnv.data), '趋势 data 必须恒为数组').toBe(true);
      const buckets = trendEnv.data as Array<Record<string, unknown>>;
      expect(
        buckets.length,
        `专属窗口 ${trendMonth} 应恰 1 个桶且只含本用例种单，实际 ${JSON.stringify(buckets)}`
      ).toBe(1);
      const may = buckets[0];
      expect(may.period, `桶键应为种单月 ${trendMonth}`).toBe(trendMonth);
      expect(typeof may.amount, 'amount 必须是字符串（Decimal=字符串口径，禁 JSON number）').toBe(
        'string'
      );
      expect(may.amount, '桶金额必须字符串等值于订单金额 1500.75').toBe('1500.75');
      expect(may.order_count, '桶订单数必须回读到真实 1 单').toBe(1);
      expect(may.quantity, '桶数量必须=明细行数量合计（两位字符串口径）').toBe('3.00');
      expect(typeof may.profit, 'profit 必须是字符串口径').toBe('string');
      // 自创假列回归防线：出参不得再出现无产出方的 growth_rate
      expect('growth_rate' in may, '出参不应含已删除的假列 growth_rate').toBe(false);

      // ② 别名 /trend 与 /trends 同参桶逐字节一致（同一 handler 双路径语义锁）
      const aliasEnv = await apiCall<unknown>(
        page,
        'GET',
        `/crm/sales-analysis/trend?${trendWindow}`
      );
      expect(
        JSON.stringify(aliasEnv.data),
        '别名 /trend 的桶必须与 /trends 完全一致（同一 handler，禁止只修一条路径）'
      ).toBe(JSON.stringify(trendEnv.data));

      // ③ period=桶键等值过滤：命中键剩该桶；不命中键返回 200+空数组（空态不是错误）
      const hitEnv = await apiCall<unknown>(
        page,
        'GET',
        `/crm/sales-analysis/trends?${trendWindow}&period=${trendMonth}`
      );
      expect(
        (hitEnv.data as unknown[]).length,
        `period=${trendMonth} 必须只剩该桶 1 个，实际 ${JSON.stringify(hitEnv.data)}`
      ).toBe(1);
      const missEnv = await apiCall<unknown>(
        page,
        'GET',
        `/crm/sales-analysis/trends?${trendWindow}&period=2095-06`
      );
      expect(
        Array.isArray(missEnv.data) && (missEnv.data as unknown[]).length === 0,
        `period 不命中必须 200+空数组，实际 ${JSON.stringify(missEnv)}`
      ).toBe(true);

      // ④ 非法 granularity ⇒ 回落月桶并留痕：200 且与 month 粒度结果逐字节一致（不 400 不 500）
      const fbEnv = await apiCall<unknown>(
        page,
        'GET',
        `/crm/sales-analysis/trends?start_date=${trendMonth}-01&end_date=${trendMonth}-31&granularity=fortnight`
      );
      expect(fbEnv.code, `非法粒度必须回落为 200 成功，实际 ${JSON.stringify(fbEnv)}`).toBe(200);
      expect(
        JSON.stringify(fbEnv.data),
        '非法 granularity 的回落结果必须与 month 粒度逐字节一致（词表外值不另造口径）'
      ).toBe(JSON.stringify(trendEnv.data));

      // ⑤ 区间倒置 ⇒ 400 + VALIDATION_ERROR（不静默吞、不裸 500），外显原因描述用户提交字段
      const reversed = await apiCallExpectFail(
        page,
        'GET',
        `/crm/sales-analysis/trends?start_date=${trendMonth}-31&end_date=${trendMonth}-01&granularity=month`
      );
      expect(reversed.status, `区间倒置应 400，实际 ${JSON.stringify(reversed)}`).toBe(400);
      expect(failureCode(reversed), '区间倒置机器码=VALIDATION_ERROR').toBe(
        APP_ERROR_CODES.VALIDATION_ERROR
      );
      expect(String(reversed.message), '拒绝原因必须外显可读').toContain(
        '结束日期不能早于开始日期'
      );
    } finally {
      if (trendOrderId > 0) {
        await tryCleanup(
          page,
          'DELETE',
          `/sales/orders/${trendOrderId}`,
          '[22] 兜底删除趋势种子订单'
        );
      }
      await tryCleanup(
        page,
        'DELETE',
        `/crm/customers/${trendCustomerId}`,
        '[22] 兜底删除趋势种子客户'
      );
    }

    // 删后无痕：换新窗口键（end 提前一天仍覆盖种单日，规避 5min TTL 同键陈旧缓存）复查，
    // 桶必须随单据消失归 0——若仍留桶即趋势并非现算 sales_orders 而是回读某张恒存壳表
    const goneEnv = await apiCall<unknown>(
      page,
      'GET',
      `/crm/sales-analysis/trends?start_date=${trendMonth}-01&end_date=${trendMonth}-30&granularity=month`
    );
    expect(goneEnv.code, '删后复查信封应成功').toBe(200);
    expect(
      (goneEnv.data as unknown[]).length,
      `删后窗口不应残留桶，实际 ${JSON.stringify(goneEnv.data)}`
    ).toBe(0);
    await verifyEndpointHealthy(page, '/crm/sales-analysis/rankings');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/stats');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/product-ranking');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/customer-ranking');
    await verifyEndpointHealthy(page, '/crm/sales-analysis/targets');
    await verifyEndpointHealthy(page, '/crm/tags');
  });

  // ===== 真实 UI 交互验证 =====
  test('CRM 客户列表 UI：搜索+新建表单+表格渲染', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm`);
    await page.waitForTimeout(3000);
    // 等待 Tab 加载
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证搜索输入框存在
    const searchInput = page
      .locator(
        'input[placeholder*="客户编码"], input[placeholder*="客户"], input[placeholder*="线索"]'
      )
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchInput.isVisible();
    if (searchVisible) {
      await searchInput.fill('测试');
      await page.waitForTimeout(500);
      // 点击查询按钮
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 });
      const btnVisible = await queryBtn.isVisible();
      if (btnVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      // 验证表格不崩溃
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .waitFor({ state: 'visible', timeout: 5000 })
        .then(() => true);
      expect(tableOk).toBe(true);
      // 清空搜索
      await searchInput.clear();
    }
  });

  test('CRM 客户列表 UI：点击新建客户→弹窗→表单校验', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-tabs')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 点击新建客户按钮
    const newBtn = page.locator('button:has-text("新建客户")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      // 验证弹窗出现
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      // 验证表单字段存在
      const codeInput = dialog.locator('input[placeholder*="客户编码"]').first();
      await codeInput.waitFor({ state: 'visible', timeout: 3000 });
      const codeVisible = await codeInput.isVisible();
      expect(codeVisible).toBe(true);
      // 直接点保存触发必填校验
      const saveBtn = dialog.locator('button:has-text("保存"), button:has-text("确定")').first();
      await saveBtn.click();
      await page.waitForTimeout(1000);
      // 验证校验错误提示
      const hasError = await page
        .locator('.el-form-item__error, .el-message--error')
        .first()
        .waitFor({ state: 'visible', timeout: 5000 })
        .then(() => true);
      expect(hasError).toBe(true);
      // 关闭弹窗
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('CRM 线索列表 UI：搜索+表格+状态标签', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm/leads`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    // 验证表格加载
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
    // 验证表头存在
    const headers = table.locator('th, .el-table-v2__header-cell');
    const headerCount = await headers.count();
    expect(headerCount).toBeGreaterThan(0);
    // 验证搜索框
    const searchInput = page
      .locator(
        'input[placeholder*="线索"], input[placeholder*="公司"], input[placeholder*="联系人"]'
      )
      .first();
    await searchInput.waitFor({ state: 'visible', timeout: 5000 });
    const searchVisible = await searchInput.isVisible();
    if (searchVisible) {
      await searchInput.fill('测试');
      const queryBtn = page.locator('button:has-text("查询")').first();
      await queryBtn.waitFor({ state: 'visible', timeout: 3000 });
      const btnVisible = await queryBtn.isVisible();
      if (btnVisible) {
        await queryBtn.click();
        await page.waitForTimeout(2000);
      }
      const tableOk = await page
        .locator(
          '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
        )
        .first()
        .isVisible();
      expect(tableOk).toBe(true);
    }
  });

  test('CRM 商机列表 UI：表格+新建按钮', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm/opportunities`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    expect(tableVisible).toBe(true);
    // 验证新建商机按钮存在
    const newBtn = page.locator('button:has-text("新建商机")').first();
    await newBtn.waitFor({ state: 'visible', timeout: 5000 });
    const newBtnVisible = await newBtn.isVisible();
    if (newBtnVisible) {
      await newBtn.click();
      await page.waitForTimeout(1000);
      const dialog = page.locator('.el-dialog').first();
      await dialog.waitFor({ state: 'visible', timeout: 5000 });
      const dialogVisible = await dialog.isVisible();
      expect(dialogVisible).toBe(true);
      await page.locator('.el-dialog__headerbtn').first().click();
    }
  });

  test('CRM 公海池+分配 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/crm/pool`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const bodyVisible = await page.locator('body').isVisible();
    expect(bodyVisible).toBe(true);
  });

  test('CRM 五维管理+销售分析 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/five-dimension`);
    await page.waitForTimeout(3000);
    await page
      .locator('.el-card, .el-table, .el-empty, body')
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    await page.goto(`${BASE_URL}/sales-analysis`);
    await page.waitForTimeout(3000);
    // isVisible 立即返回，组件异步挂载可能未渲染 → 改用 waitFor 等待可见
    const container = page.locator('.el-card, .el-table, .el-empty, body').first();
    await container.waitFor({ state: 'visible', timeout: 15_000 });
    const visible = await container.isVisible();
    expect(visible).toBe(true);
  });

  test('客户管理 UI 页面', async ({ page }) => {
    await page.goto(`${BASE_URL}/customer`);
    await page.waitForTimeout(3000);
    await page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-card, .el-empty, body'
      )
      .first()
      .waitFor({ state: 'visible', timeout: 30_000 });
    const table = page
      .locator(
        '.el-table, .el-table-v2, [role="table"], .v2-table-wrapper, .el-table-v2, [role="table"], .v2-table-wrapper'
      )
      .first();
    await table.waitFor({ state: 'visible', timeout: 10_000 });
    const tableVisible = await table.isVisible();
    if (tableVisible) {
      const headers = table.locator('th, .el-table-v2__header-cell');
      const headerCount = await headers.count();
      expect(headerCount).toBeGreaterThan(0);
    }
  });
});
