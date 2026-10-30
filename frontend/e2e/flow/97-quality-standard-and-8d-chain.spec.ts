import { test, expect } from '../diagnose-fixture';
import {
  loginViaUI,
  apiCall,
  apiCallRaw,
  apiCallExpectFail,
  expectStateGateRejection,
  genCode,
  tryCleanup,
  API_BASE,
  API_PREFIX,
} from './helpers';
// 产品类型单源常量（与后端 product_type 词表同源）：建品夹具取值必须经成员引用。
import { PRODUCT_TYPE } from '../../src/constants/product-type';

/**
 * 97 质量域两条链的首个真契约覆盖（质量标准生命周期 与 8D 全阶段推进）
 *
 * 与既有覆盖的分工。54 号在页面上点过 8D 启动与 D1 推进的 UI 通道，
 * 判定器口径下对象端点未计为覆盖；本 spec 以 API 直调把两条链的每个
 * 写口与读口逐段钉死，钉落库字段、状态流转与门禁拒绝。
 *
 * 契约真相（逐段读后端源，非推测）：
 * 质量标准（handlers/quality_standard_handler 与 services/quality_standard_service，
 * 路由 quality-standards 组）：建单入参 standard_name 必填、standard_code 缺省走
 * 单号生成器；初态 draft。approve 门收 draft 或 rejected；publish 门只收 approved，
 * 通过置 active；archive 门收 approved 或 active，通过置 archived；
 * 删除门只按引用拒（被定制品单或版本链后继引用即 400），状态不设限。
 * 版本历史读口返回该标准自身行集合（按 id 过滤，非后继链），断言至少含本行。
 * versions 写口以同码行插入实现，与 standard_code 唯一约束天然冲突，
 * 本波不碰（已在交付说明登记后端缺陷，绕开不等于修好）。
 * 导出口出 xlsx 二进制流，断 HTTP 200、content-type 为表格流、字节非空。
 * 状态词表值 draft、approved、rejected、active、archived 全小写。
 *
 * 8D（handlers/quality_8d_handler 与 services/quality_8d_service，
 * 路由 quality-8d-reports 组）：启动入参 quality_issue_id 与可选 plan，
 *  prepared_by 由服务端取会话 user_id，body 伪造无效。
 * 推进载荷是 tagged enum，键 step 取值 d1_team 至 d8_recognize 八枚，
 * 每枚附带该阶段必填字段；d4 的 method 词表为 5why、fishbone、other。
 * 状态机 eleven 态依次为 d0_plan、d1_team、d2_problem、d3_interim、
 * d4_root_cause、d5_permanent、d6_verify、d7_prevent、d8_recognize、closed。
 * 关闭口无请求体，closed_by 服务端派生；未到 d8_recognize 关闭按状态门 400。
 * 同异常重复启动按 AlreadyExists 拒绝 400。by-issue 查口返回单对象或 null。
 * 前置链：客户、分类、产品、定制订单、质量异常逐级外键，全部自建。
 *
 * 假绿防线。负例一律 apiCallExpectFail 加 expectStateGateRejection 钉 HTTP 400
 * 与状态门机器码族，不读文案。每条被拒写操作紧跟 GET 回读断状态与原值零漂移。
 * 每级推进后回读该行并断本阶段字段等于提交原文。rust_decimal 出参为字符串，
 * 比较前 Number 归一。可空未提交列断 null，不接受空串或 0 占位。
 */

type Row = Record<string, unknown>;

/** 推进链一级定义（载荷键集与后端 AdvanceStepPayload 变体逐键同形） */
type AdvanceStep = { payload: Row; expectStatus: string };

const ymd = (daysFromToday: number): string =>
  new Date(Date.now() + daysFromToday * 86_400_000).toISOString().slice(0, 10);

/**
 * 97-A 质量标准生命周期链。建单到归档逐口真读回，含未审批先发布的门禁负例。
 */
test.describe.serial('97 质量标准生命周期链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('T1 建单回读 未审批发布被拒 更新回读 审批 发布 归档 版本读 导出', async ({ page }) => {
    const tag = genCode('QS97');
    const contentV1 = `初始条款 ${tag}`;
    const contentV2 = `修订条款 ${tag}`;

    // 1) 建单（auto code），初态 draft，回显行与后续 GET 详情互证
    const created = await apiCallRaw<Row>(page, 'POST', '/quality-standards', {
      standard_name: `97号标准${tag}`,
      standard_type: 'product',
      version: '1.0',
      content: contentV1,
      effective_date: ymd(0),
    });
    const id = Number(created.id);
    expect(
      Number.isInteger(id) && id > 0,
      `建单应返回真实主键，实际 ${JSON.stringify(created)}`
    ).toBe(true);
    expect(String(created.status), `初态必须为 draft，实际 ${String(created.status)}`).toBe(
      'draft'
    );
    expect(String(created.standard_code ?? ''), 'standard_code 应由服务端生成非空').not.toBe('');
    expect(String(created.version)).toBe('1.0');
    expect(String(created.content), '建单回显 content 应为提交原文').toBe(contentV1);

    // 2) 详情读口逐列对表（A 档 GET 对象口第一次真读）
    const detail = await apiCallRaw<Row>(page, 'GET', `/quality-standards/${id}`);
    expect(Number(detail.id)).toBe(id);
    expect(String(detail.standard_name)).toBe(`97号标准${tag}`);
    expect(String(detail.standard_code)).toBe(String(created.standard_code));
    expect(String(detail.status)).toBe('draft');

    // 3) 未审批先发布必须 400 且零漂移（publish 门只收 approved）
    const earlyPublish = await apiCallExpectFail(page, 'POST', `/quality-standards/${id}/publish`);
    expectStateGateRejection(earlyPublish, `未审批标准 ${id} 发布`);
    const afterEarlyPublish = await apiCallRaw<Row>(page, 'GET', `/quality-standards/${id}`);
    expect(String(afterEarlyPublish.status), '被拒的发布不许推进状态').toBe('draft');
    expect(String(afterEarlyPublish.content), '被拒的发布不许改内容').toBe(contentV1);

    // 4) 更新口（A 档 PUT）改名称与内容，GET 真读回
    await apiCall(page, 'PUT', `/quality-standards/${id}`, {
      standard_name: `97号标准改${tag}`,
      content: contentV2,
      remark: `97号修订注记${tag}`,
    });
    const updated = await apiCallRaw<Row>(page, 'GET', `/quality-standards/${id}`);
    expect(String(updated.standard_name)).toBe(`97号标准改${tag}`);
    expect(String(updated.content)).toBe(contentV2);
    expect(String(updated.status), '只改文本不许动状态').toBe('draft');

    // 5) 审批（前置口）后发布（A 档），active 回读
    await apiCall(page, 'POST', `/quality-standards/${id}/approve`);
    const approved = await apiCallRaw<Row>(page, 'GET', `/quality-standards/${id}`);
    expect(String(approved.status)).toBe('approved');

    await apiCall(page, 'POST', `/quality-standards/${id}/publish`);
    const published = await apiCallRaw<Row>(page, 'GET', `/quality-standards/${id}`);
    expect(String(published.status), `发布后必须 active，实际 ${String(published.status)}`).toBe(
      'active'
    );

    // 6) 版本读口（A 档）：契约是按 id 过滤自身行，至少含本行且版本同值
    const versions = await apiCallRaw<Row[]>(page, 'GET', `/quality-standards/${id}/versions`);
    expect(Array.isArray(versions), '版本读口必须返回数组').toBe(true);
    expect(versions.length, '版本读口至少含标准本行').toBeGreaterThanOrEqual(1);
    const self = versions.find(v => Number(v.id) === id);
    expect(self, `版本读口应含 id=${id} 的本行，实际 ${JSON.stringify(versions)}`).toBeDefined();
    expect(String(self!.version)).toBe('1.0');

    // 7) 归档（A 档，门收 active）后回读 archived
    await apiCall(page, 'POST', `/quality-standards/${id}/archive`);
    const archived = await apiCallRaw<Row>(page, 'GET', `/quality-standards/${id}`);
    expect(String(archived.status), `归档后必须 archived，实际 ${String(archived.status)}`).toBe(
      'archived'
    );

    // 8) 导出口（A 档）真取字节流：xlsx 响应不是 JSON，走裸请求断类型与非空
    const exportResp = await page.request.get(`${API_BASE}${API_PREFIX}/quality-standards/export`);
    expect(exportResp.status(), `导出应 200，实际 ${exportResp.status()}`).toBe(200);
    const ctype = exportResp.headers()['content-type'] ?? '';
    expect(
      ctype.includes('spreadsheetml') || ctype.includes('octet-stream'),
      `导出 content-type 应为表格流或二进制流，实际 ${ctype}`
    ).toBe(true);
    const exportBody = await exportResp.body();
    expect(exportBody.byteLength, '导出体必须非空').toBeGreaterThan(0);

    // 9) 清理：归档行未被引用可真删
    await tryCleanup(page, 'DELETE', `/quality-standards/${id}`, `[97] standard ${id}`);
  });
});

/**
 * 97-B 8D 全阶段推进链。自建前置到关闭回读，含跳级、错步、重复三族负例。
 */
test.describe.serial('97 8D 全阶段推进链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('T2 前置五级自建 启动 跳级与错步被拒 八级推进逐级回读 按异常查 关闭 重复启动被拒', async ({
    page,
  }) => {
    const tag = genCode('8D97');
    const me = await apiCallRaw<Row>(page, 'GET', '/auth/me');
    const myId = Number(me.id);
    expect(Number.isInteger(myId) && myId > 0, `/auth/me 必须回真实用户 id`).toBe(true);

    // 前置第一级 客户与分类与产品（定制订单两级外键）
    const customer = await apiCallRaw<Row>(page, 'POST', '/crm/customers', {
      customer_name: `97客户${tag}`,
      contact_person: `97联系人${tag}`,
      contact_phone: `139${Date.now().toString().slice(-8)}`,
      customer_type: 'retail',
      status: 'active',
    });
    const customerId = Number(customer.id);
    expect(customerId > 0, `客户自建失败 ${JSON.stringify(customer)}`).toBe(true);

    const category = await apiCallRaw<Row>(page, 'POST', '/categories', {
      name: `97类${tag}`,
      code: `97CAT${tag}`,
    });
    const categoryId = Number(category.id);
    expect(categoryId > 0, `分类自建失败 ${JSON.stringify(category)}`).toBe(true);

    const product = await apiCallRaw<Row>(page, 'POST', '/products', {
      name: `97布${tag}`,
      code: `97P${tag}`,
      category_id: categoryId,
      unit: 'm',
      // 质量标准/8D 链宿主产品不做米↔公斤换算，取非面料类型免带面料三键
      product_type: PRODUCT_TYPE.auxiliary,
    });
    const productId = Number(product.id);
    expect(productId > 0, `产品自建失败 ${JSON.stringify(product)}`).toBe(true);

    // 前置第二级 定制订单（quality_issue 的父链，8D 归属门经它判权）
    const order = await apiCallRaw<Row>(page, 'POST', '/custom-orders', {
      customer_id: customerId,
      product_id: productId,
      spec: `97SPEC${tag}`,
      quantity: 10,
      unit: 'm',
    });
    const orderId = Number(order.id);
    expect(orderId > 0, `定制订单自建失败 ${JSON.stringify(order)}`).toBe(true);

    // 前置第三级 质量异常（8D 的直指父行）
    const issue = await apiCallRaw<Row>(page, 'POST', `/custom-orders/${orderId}/issues`, {
      issue_type: 'color_diff',
      severity: 'high',
      description: `97异常${tag}`,
    });
    const issueId = Number(issue.id);
    expect(issueId > 0, `质量异常自建失败 ${JSON.stringify(issue)}`).toBe(true);

    // 1) 启动（A 档 POST 集合口）。prepared_by 服务端派生，body 未提供也不许伪造
    const started = await apiCallRaw<Row>(page, 'POST', '/quality-8d-reports', {
      quality_issue_id: issueId,
      plan: `97计划${tag}`,
    });
    const reportId = Number(started.id);
    expect(reportId > 0, `启动应返回真实主键，实际 ${JSON.stringify(started)}`).toBe(true);
    expect(String(started.status), `启动后必须 d0_plan，实际 ${String(started.status)}`).toBe(
      'd0_plan'
    );
    expect(Number(started.quality_issue_id), '父异常必须等于提交值').toBe(issueId);
    expect(
      Number(started.d0_prepared_by),
      `d0_prepared_by 必须取会话用户 ${myId}，实际 ${String(started.d0_prepared_by)}`
    ).toBe(myId);
    expect(started.d0_date, 'd0 日期必须由服务端落值').not.toBeNull();
    expect(String(started.d0_plan)).toBe(`97计划${tag}`);

    // 2) 未到 d8_recognize 先关闭必须 400 且零漂移（跳级负例）
    const earlyClose = await apiCallExpectFail(
      page,
      'POST',
      `/quality-8d-reports/${reportId}/close`
    );
    expectStateGateRejection(earlyClose, `d0 阶段直接关闭 8D ${reportId}`);
    const afterEarlyClose = await apiCallRaw<Row>(page, 'GET', `/quality-8d-reports/${reportId}`);
    expect(String(afterEarlyClose.status), '被拒的关闭不许推进状态').toBe('d0_plan');
    expect(afterEarlyClose.closed_at, '被拒的关闭不许落 closed_at').toBeNull();
    expect(afterEarlyClose.closed_by, '被拒的关闭不许落 closed_by').toBeNull();

    // 3) 错步推进必须 400 且零漂移（d0_plan 只收 d1_team，发 d2_problem 即态不匹配）
    const wrongStep = await apiCallExpectFail(
      page,
      'POST',
      `/quality-8d-reports/${reportId}/advance`,
      {
        step: 'd2_problem',
        problem_description: `错步探针${tag}`,
      }
    );
    expectStateGateRejection(wrongStep, `d0 阶段发 d2_problem 推进`);
    const afterWrongStep = await apiCallRaw<Row>(page, 'GET', `/quality-8d-reports/${reportId}`);
    expect(String(afterWrongStep.status), '被拒的错步不许推进状态').toBe('d0_plan');

    // 4) 八级推进链（A 档 advance 口连发八次），每级回读断态与字段原文
    const steps: AdvanceStep[] = [
      { payload: { step: 'd1_team', team_members: `小组${tag}` }, expectStatus: 'd1_team' },
      {
        payload: { step: 'd2_problem', problem_description: `现象${tag}` },
        expectStatus: 'd2_problem',
      },
      {
        payload: { step: 'd3_interim', interim_action: `临时措施${tag}` },
        expectStatus: 'd3_interim',
      },
      {
        payload: {
          step: 'd4_root_cause',
          method: '5why',
          detail: `五问过程${tag}`,
          summary: `根因${tag}`,
        },
        expectStatus: 'd4_root_cause',
      },
      {
        payload: {
          step: 'd5_permanent',
          permanent_action: `永久措施${tag}`,
          action_owner: `责任人${tag}`,
          due_date: ymd(30),
        },
        expectStatus: 'd5_permanent',
      },
      {
        payload: { step: 'd6_verify', verification_result: `验证结论${tag}` },
        expectStatus: 'd6_verify',
      },
      {
        payload: { step: 'd7_prevent', prevention_action: `预防${tag}` },
        expectStatus: 'd7_prevent',
      },
      {
        payload: { step: 'd8_recognize', closure_summary: `结案总结${tag}` },
        expectStatus: 'd8_recognize',
      },
    ];
    for (const step of steps) {
      const advanced = await apiCallRaw<Row>(
        page,
        'POST',
        `/quality-8d-reports/${reportId}/advance`,
        step.payload
      );
      expect(
        String(advanced.status),
        `推进 ${String(step.payload.step)} 后写响应应为 ${step.expectStatus}，实际 ${String(advanced.status)}`
      ).toBe(step.expectStatus);
      const readback = await apiCallRaw<Row>(page, 'GET', `/quality-8d-reports/${reportId}`);
      expect(String(readback.status), `推进 ${String(step.payload.step)} 后回读必须同态`).toBe(
        step.expectStatus
      );
    }
    // 关键阶段字段逐列钉死（读口与写口互证，防"推进只改状态不落内容"的半截实现）
    const finalRow = await apiCallRaw<Row>(page, 'GET', `/quality-8d-reports/${reportId}`);
    expect(String(finalRow.d1_team_members)).toBe(`小组${tag}`);
    expect(String(finalRow.d2_problem_description)).toBe(`现象${tag}`);
    expect(String(finalRow.d3_interim_action)).toBe(`临时措施${tag}`);
    expect(String(finalRow.d4_root_cause_method), `d4 method 必须按词表落 5why`).toBe('5why');
    expect(String(finalRow.d4_root_cause_summary)).toBe(`根因${tag}`);
    expect(String(finalRow.d5_permanent_action)).toBe(`永久措施${tag}`);
    expect(String(finalRow.d5_action_owner)).toBe(`责任人${tag}`);
    expect(String(finalRow.d5_due_date), 'd5 计划完成日期必须等于提交值').toBe(ymd(30));
    expect(String(finalRow.d8_closure_summary)).toBe(`结案总结${tag}`);

    // 5) 按异常查口（A 档）命中本行且状态一致
    const byIssue = await apiCallRaw<Row>(page, 'GET', `/quality-8d-reports/by-issue/${issueId}`);
    expect(byIssue, `by-issue 查口应命中报告，实际 null`).not.toBeNull();
    expect(Number(byIssue.id), 'by-issue 命中行必须是本报告的 id').toBe(reportId);
    expect(String(byIssue.status)).toBe('d8_recognize');

    // 6) 关闭（A 档）置 closed 且 closed_by 派生、closed_at 落值
    const closed = await apiCallRaw<Row>(page, 'POST', `/quality-8d-reports/${reportId}/close`);
    expect(String(closed.status), `关闭后必须 closed，实际 ${String(closed.status)}`).toBe(
      'closed'
    );
    expect(Number(closed.closed_by), 'closed_by 必须取会话用户').toBe(myId);
    expect(closed.closed_at, 'closed_at 必须由服务端落值').not.toBeNull();
    const closedReadback = await apiCallRaw<Row>(page, 'GET', `/quality-8d-reports/${reportId}`);
    expect(String(closedReadback.status)).toBe('closed');

    // 7) 同异常重复启动必须 400 且不产生第二份报告（AlreadyExists 族）
    const dupStart = await apiCallExpectFail(page, 'POST', '/quality-8d-reports', {
      quality_issue_id: issueId,
      plan: `重复探针${tag}`,
    });
    expectStateGateRejection(dupStart, `异常 ${issueId} 重复启动 8D`);
    const byIssueAfterDup = await apiCallRaw<Row>(
      page,
      'GET',
      `/quality-8d-reports/by-issue/${issueId}`
    );
    expect(
      Number(byIssueAfterDup.id),
      '重复启动被拒后 by-issue 命中行必须仍是原报告，不许冒第二份'
    ).toBe(reportId);

    // 8) 清理。8D 报告无删口，异常带结案说明收口；产品与分类与客户走延迟删除。
    // resolve 口要求 resolution 必填体，tryCleanup 无体通道不适用，就地容错直调。
    await apiCall(page, 'PUT', `/custom-orders/issues/${issueId}/resolve`, {
      resolution: `97波随链结案${tag}`,
    }).catch(e => console.warn(`[97] issue ${issueId} resolve 失败: ${(e as Error).message}`));
    await tryCleanup(page, 'DELETE', `/products/${productId}`, `[97] product ${productId}`);
    await tryCleanup(page, 'DELETE', `/categories/${categoryId}`, `[97] category ${categoryId}`);
    await tryCleanup(page, 'DELETE', `/crm/customers/${customerId}`, `[97] customer ${customerId}`);
  });
});
