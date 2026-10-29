// [法#3 范围锁定] 目标：glob='frontend/e2e/enhanced/labor-contracts-readback.spec.ts'
// 劳动合同域零覆盖补齐 E2E — 建约 详情 在有效合同查 局部更新 终止 到期预警 打印
//
// 功能：把 labor-contracts 域在册但 e2e 字面量零命中的七枚端点钉成
// 写后同作用域回读真值链，并把《劳动合同法》校验门的拒绝钉到 HTTP 与机器码。
// 调用方：路由注册在 backend/src/routes/labor_contract.rs（path 前缀 labor-contracts），
// handler 在 backend/src/handlers/labor_contract_handler.rs，业务规则与落库在
// backend/src/services/labor_contract_service.rs，打印腿在
// backend/src/handlers/print_handler.rs 的 labor_contract_print_docx。
// 入参：建约必填 worker_id contract_no contract_type start_date probation_salary
// regular_salary working_hours_system sign_date；fixed_term 必带 end_date；
// 终止必带 termination_date 与 termination_reason。worker_id 无外键，
// 用例取 9_1xx_xxx 大段唯一号自建自管，绝不与真实工人共用。
// 传给谁：labor_contracts 表（status 服务端写死 active 初值，created_by 取会话用户）；
// 到期预警扫描把命中的过期 active 行回写 expired；打印走 PrintService 装配 docx。
// 存什么：合同全列（合同号 类型 起止 试用期止 试用与转正工资 岗位 部门 地点
// 工时制度 签订日 状态 终止日期与原因 备注）；缺值列写 NULL，不写 0。
// 存哪里：labor_contracts 表；打印审计写 audit_logs（异步 best-effort）。
// 本文件覆盖端点：POST labor-contracts、GET labor-contracts/{id}、
// GET labor-contracts/active-by-worker/{worker_id}、PUT labor-contracts/{id}、
// POST labor-contracts/{id}/terminate、POST labor-contracts/scan-expiry-warnings、
// GET labor-contracts/{id}/print。列表腿已有覆盖，本文件仅用其做负例回读通道。
// 清理与隔离事实：域内无删除端点，用例把自建合同全部推到 terminated 终态收口
// （terminated 不再被 active-by-worker 与到期扫描命中）；每用例独立 worker_id 段，
// 合同号取本轮唯一标记，断言只按自建 id 或唯一键回读。
// 脱敏事实：AppError 默认脱敏——bad_request 出参恒为请求参数错误，
// business 恒为业务处理失败，not_found 恒为资源未找到；本域四类拒绝全部
// 只钉 HTTP 与机器码，文案按脱敏常量逐字断言。打印腿属可达性与文件流证据，
// 不是数据行契约；到期扫描是数据范围全量动作，会把库中已过期的 active 行
// 回写 expired，属端点自身设计行为，本文件只按自建行断言命中。
import { test, expect } from '../diagnose-fixture';
import { applyAuthMocks } from '../smoke/_helpers';
import {
  APP_ERROR_CODES,
  API_BASE,
  API_PREFIX,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  failureCode,
  genCode,
} from '../flow/helpers';

type Row = Record<string, unknown>;
const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

/** 到期扫描按服务器本地日计算剩余天数，跨时区有 1 日抖动，容差双端点给出 */
function inNDays(offsetDays: number): { ymd: string; minDays: number; maxDays: number } {
  const target = new Date(Date.now() + offsetDays * 86_400_000);
  return {
    ymd: target.toISOString().slice(0, 10),
    minDays: offsetDays - 1,
    maxDays: offsetDays + 1,
  };
}

/** 本轮唯一 worker_id 段：与真实工人号段错开，用例间互不串扰 */
const WORKER_BASE = 9_100_000 + (Date.now() % 90_000);
const workerFor = (slot: number): number => WORKER_BASE + slot;

/** 建一份字段齐全的 2 年期固定期限合同入参（可按需覆盖个别字段做负例） */
function contractPayload(workerId: number, overrides: Record<string, unknown> = {}) {
  const start = new Date();
  const end = new Date(start.getTime() + 730 * 86_400_000);
  const sign = new Date(start.getTime() + 5 * 86_400_000);
  const probationEnd = new Date(start.getTime() + 30 * 86_400_000);
  return {
    worker_id: workerId,
    contract_no: genCode('E2E-LC'),
    contract_type: 'fixed_term',
    start_date: start.toISOString().slice(0, 10),
    end_date: end.toISOString().slice(0, 10),
    probation_end_date: probationEnd.toISOString().slice(0, 10),
    probation_salary: 8000,
    regular_salary: 10000,
    position: '织机操作工',
    department: '织造车间',
    work_location: '一号厂区',
    working_hours_system: 'standard',
    sign_date: sign.toISOString().slice(0, 10),
    remarks: 'E2E 劳动合同契约用例',
    ...overrides,
  };
}

const todayYmd = (): string => new Date().toISOString().slice(0, 10);

test.describe('劳动合同 建约与全生命周期回读', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('LC-01 建约 详情 在有效查 局部更新保原值 终止 全链回读', async ({ page }) => {
    const workerId = workerFor(1);
    const payload = contractPayload(workerId);
    const created = await apiCallRaw<Row>(page, 'POST', '/labor-contracts', payload);
    const id = num(created.id);
    expect(
      Number.isInteger(id) && id > 0,
      `建约应回自增主键，实际 ${JSON.stringify(created)}`
    ).toBe(true);
    expect(str(created.status), `建约服务端初值状态应为 active`).toBe('active');
    expect(str(created.contract_no), `建约出参合同号应逐字回显提交值`).toBe(
      str(payload.contract_no)
    );
    expect(num(created.worker_id), `建约出参 worker_id 应落提交值`).toBe(workerId);

    // 详情腿逐列回读
    const detail = await apiCallRaw<Row>(page, 'GET', `/labor-contracts/${created.id}`);
    expect(num(detail.id), `详情 id 应为自建合同`).toBe(id);
    expect(str(detail.contract_no), `详情合同号应与建约同源`).toBe(str(payload.contract_no));
    expect(str(detail.status), `详情初始状态 active`).toBe('active');
    expect(str(detail.start_date), `详情开始日期逐字落库`).toBe(str(payload.start_date));
    expect(str(detail.end_date), `详情结束日期逐字落库`).toBe(str(payload.end_date));
    expect(str(detail.working_hours_system), `详情工时制度逐字落库`).toBe('standard');
    expect(
      num(detail.regular_salary),
      `详情转正工资应落 10000，实际 ${detail.regular_salary}`
    ).toBe(10000);
    expect(detail.termination_date, `未终止合同终止日期应为 NULL`).toBe(null);
    expect(detail.termination_reason, `未终止合同终止原因应为 NULL`).toBe(null);

    // 在有效合同查
    const active = await apiCallRaw<Row>(
      page,
      'GET',
      `/labor-contracts/active-by-worker/${workerId}`
    );
    expect(num(active.id), `active-by-worker 应命中自建合同`).toBe(id);

    // 局部更新只提交 position：未提交列必须保原值
    const putRes = await apiCall(page, 'PUT', `/labor-contracts/${created.id}`, {
      position: '挡车工',
    });
    expect(str((putRes.data as Row).position), `更新出参 position 应为提交值`).toBe('挡车工');
    const afterUpdate = await apiCallRaw<Row>(page, 'GET', `/labor-contracts/${created.id}`);
    expect(str(afterUpdate.position), `提交的 position 应变更`).toBe('挡车工');
    expect(str(afterUpdate.department), `未提交 department 保持原值`).toBe('织造车间');
    expect(str(afterUpdate.work_location), `未提交 work_location 保持原值`).toBe('一号厂区');
    expect(str(afterUpdate.contract_no), `未提交 contract_no 保持原值`).toBe(
      str(payload.contract_no)
    );
    expect(str(afterUpdate.end_date), `未提交 end_date 保持原值`).toBe(str(payload.end_date));
    expect(num(afterUpdate.regular_salary), `未提交 regular_salary 保持原值`).toBe(10000);
    expect(str(afterUpdate.status), `更新不改状态机`).toBe('active');

    // 终止正腿：状态与终止两列同腿回读
    const termDate = todayYmd();
    const terminated = await apiCallRaw<Row>(
      page,
      'POST',
      `/labor-contracts/${created.id}/terminate`,
      {
        termination_date: termDate,
        termination_reason: '合同到期双方协商一致',
      }
    );
    expect(str(terminated.status), `终止后出参状态应为 terminated`).toBe('terminated');
    const afterTerm = await apiCallRaw<Row>(page, 'GET', `/labor-contracts/${created.id}`);
    expect(str(afterTerm.status), `终止回读状态 terminated`).toBe('terminated');
    expect(str(afterTerm.termination_date), `终止日期逐字落库`).toBe(termDate);
    expect(str(afterTerm.termination_reason), `终止原因逐字落库`).toBe('合同到期双方协商一致');
    expect(str(afterTerm.position), `终止不覆盖此前更新写入的 position`).toBe('挡车工');

    // 终止后 active-by-worker 查无
    const activeAfterTerm = await apiCallRaw<Row>(
      page,
      'GET',
      `/labor-contracts/active-by-worker/${workerId}`
    );
    expect(
      activeAfterTerm,
      `terminated 后在有效合同应为 null，实际 ${JSON.stringify(activeAfterTerm)}`
    ).toBe(null);

    // 重复终止与终止后更新撞状态门（business 脱敏），回读钉原值
    const termAgain = await apiCallExpectFail(
      page,
      'POST',
      `/labor-contracts/${created.id}/terminate`,
      {
        termination_date: termDate,
        termination_reason: '第二次终止',
      }
    );
    expect(termAgain.status, `已终止合同重复终止应恰 400，实际 ${termAgain.status}`).toBe(400);
    expect(failureCode(termAgain)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(termAgain.message ?? '')).toBe('业务处理失败');
    const putAfterTerm = await apiCallExpectFail(page, 'PUT', `/labor-contracts/${created.id}`, {
      position: '幽灵岗位',
    });
    expect(putAfterTerm.status).toBe(400);
    expect(failureCode(putAfterTerm)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    const finalCheck = await apiCallRaw<Row>(page, 'GET', `/labor-contracts/${created.id}`);
    expect(str(finalCheck.termination_reason), `被拒的重复终止不得覆盖终止原因`).toBe(
      '合同到期双方协商一致'
    );
    expect(str(finalCheck.position), `被拒的更新不得覆盖 position`).toBe('挡车工');
    expect(str(finalCheck.status), `终态保持 terminated`).toBe('terminated');
  });

  test('LC-02 创建与终止入参门：六类拒绝钉码并回读证零落库', async ({ page }) => {
    const workerId = workerFor(2);

    // 门一：无效合同类型（bad_request 构造，出参脱敏）
    const badType = await apiCallExpectFail(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerId, { contract_type: 'wrong_type' })
    );
    expect(badType.status, `无效类型应恰 400，实际 ${badType.status}`).toBe(400);
    expect(failureCode(badType)).toBe(APP_ERROR_CODES.BAD_REQUEST);
    expect(String(badType.message ?? ''), `bad_request 出参必须脱敏`).toBe('请求参数错误');

    // 门二：无效工时制度
    const badHours = await apiCallExpectFail(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerId, { working_hours_system: 'night_only' })
    );
    expect(badHours.status).toBe(400);
    expect(failureCode(badHours)).toBe(APP_ERROR_CODES.BAD_REQUEST);

    // 门三：固定期限缺结束日期
    const noEnd = await apiCallExpectFail(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerId, { end_date: null })
    );
    expect(noEnd.status).toBe(400);
    expect(failureCode(noEnd)).toBe(APP_ERROR_CODES.BAD_REQUEST);

    // 门四：负工资
    const negSalary = await apiCallExpectFail(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerId, { regular_salary: -1 })
    );
    expect(negSalary.status).toBe(400);
    expect(failureCode(negSalary)).toBe(APP_ERROR_CODES.BAD_REQUEST);

    // 回读证零落库：四连拒后该 worker 在有效合同仍为查无
    const active = await apiCallRaw<Row>(
      page,
      'GET',
      `/labor-contracts/active-by-worker/${workerId}`
    );
    expect(active, `被拒建约不得落库，实际 ${JSON.stringify(active)}`).toBe(null);

    // 门五：重复合同号撞判重（business 构造，脱敏），先建一条真约占号
    const dupNo = genCode('E2E-LC-DUP');
    const first = await apiCallRaw<Row>(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerFor(3), { contract_no: dupNo })
    );
    const dup = await apiCallExpectFail(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerFor(4), { contract_no: dupNo })
    );
    expect(dup.status, `重复合同号应恰 400，实际 ${dup.status}`).toBe(400);
    expect(failureCode(dup)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(dup.message ?? '')).toBe('业务处理失败');
    const dupWorkerActive = await apiCallRaw<Row>(
      page,
      'GET',
      `/labor-contracts/active-by-worker/${workerFor(4)}`
    );
    expect(dupWorkerActive, `重复号建约被拒后不得给新工人落行`).toBe(null);

    // 门六：终止原因空串走 bad_request（空原因校验先于任何写），原合同状态不动
    const emptyReason = await apiCallExpectFail(
      page,
      'POST',
      `/labor-contracts/${first.id}/terminate`,
      { termination_date: todayYmd(), termination_reason: '   ' }
    );
    expect(emptyReason.status, `空终止原因应恰 400，实际 ${emptyReason.status}`).toBe(400);
    expect(failureCode(emptyReason)).toBe(APP_ERROR_CODES.BAD_REQUEST);
    expect(String(emptyReason.message ?? ''), `bad_request 出参必须脱敏`).toBe('请求参数错误');
    const stillActive = await apiCallRaw<Row>(page, 'GET', `/labor-contracts/${first.id}`);
    expect(str(stillActive.status), `被拒终止后状态应保持 active`).toBe('active');
    expect(stillActive.termination_reason, `被拒终止不得写终止原因`).toBe(null);

    // 未知主键三枚腿钉 404（not_found 脱敏）
    const absent = { probeId: 9_299_999 };
    const failGet = await apiCallExpectFail(page, 'GET', `/labor-contracts/${absent.probeId}`);
    expect(failGet.status).toBe(404);
    expect(failureCode(failGet)).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(String(failGet.message ?? ''), `not_found 出参必须脱敏`).toBe('资源未找到');
    const failPut = await apiCallExpectFail(page, 'PUT', `/labor-contracts/${absent.probeId}`, {
      position: '幽灵岗位',
    });
    expect(failPut.status).toBe(404);
    expect(failureCode(failPut)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const failTerm = await apiCallExpectFail(
      page,
      'POST',
      `/labor-contracts/${absent.probeId}/terminate`,
      {
        termination_date: todayYmd(),
        termination_reason: '幽灵终止',
      }
    );
    expect(failTerm.status).toBe(404);
    expect(failureCode(failTerm)).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 收口：把占号真约推到 terminated
    await apiCall(page, 'POST', `/labor-contracts/${first.id}/terminate`, {
      termination_date: todayYmd(),
      termination_reason: 'E2E 收口',
    });
  });

  test('LC-03 试用期法定门：长度超限与工资比例超限钉 business 并证零落库', async ({ page }) => {
    // 2 年期合同试用期上限 60 天，提交 120 天撞《劳动合同法》第19条（business 脱敏）
    const longProbation = await apiCallExpectFail(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerFor(5), {
        probation_end_date: new Date(Date.now() + 120 * 86_400_000).toISOString().slice(0, 10),
      })
    );
    expect(longProbation.status, `试用期超限应恰 400，实际 ${longProbation.status}`).toBe(400);
    expect(failureCode(longProbation), `法定门机器码归 business 族`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(String(longProbation.message ?? ''), `business 出参必须脱敏`).toBe('业务处理失败');

    // 试用期工资低于转正 80% 撞第20条
    const lowProbSalary = await apiCallExpectFail(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerFor(6), { probation_salary: 7000 })
    );
    expect(lowProbSalary.status).toBe(400);
    expect(failureCode(lowProbSalary)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(lowProbSalary.message ?? '')).toBe('业务处理失败');

    // 回读证零落库：两个 worker 都查无在有效合同
    for (const slot of [5, 6]) {
      const active = await apiCallRaw<Row>(
        page,
        'GET',
        `/labor-contracts/active-by-worker/${workerFor(slot)}`
      );
      expect(
        active,
        `法定门被拒建约不得落库（worker ${workerFor(slot)}），实际 ${JSON.stringify(active)}`
      ).toBe(null);
    }
  });

  test('LC-04 到期预警扫描与打印：30 天内到期命中自建行并按剩余天数升序', async ({ page }) => {
    const soon = inNDays(20);
    const created = await apiCallRaw<Row>(
      page,
      'POST',
      '/labor-contracts',
      contractPayload(workerFor(7), {
        end_date: soon.ymd,
        probation_end_date: null,
        sign_date: todayYmd(),
      })
    );
    const id = num(created.id);
    expect(
      Number.isInteger(id) && id > 0,
      `近到期建约应成功，实际 ${JSON.stringify(created)}`
    ).toBe(true);

    const warnings = await apiCallRaw<Row[]>(page, 'POST', '/labor-contracts/scan-expiry-warnings');
    expect(Array.isArray(warnings), `扫描应出数组，实际 ${JSON.stringify(warnings)}`).toBe(true);
    const mine = warnings.find(w => num((w.contract as Row)?.id) === id);
    expect(
      mine,
      `扫描应命中自建近到期合同 ${id}，实际命中集 ${JSON.stringify(
        warnings.map(w => (w.contract as Row)?.id)
      )}`
    ).toBeTruthy();
    expect(str(mine?.level), `20 天内到期级别应为 Warning30Days`).toBe('Warning30Days');
    const days = num(mine?.days_until_expiry);
    expect(
      Number.isInteger(days) && days >= soon.minDays && days <= soon.maxDays,
      `剩余天数应为整数且贴近提交差值（${soon.minDays} 到 ${soon.maxDays}），实际 ${mine?.days_until_expiry}`
    ).toBe(true);
    expect(str((mine?.contract as Row)?.contract_no), `命中行合同号应与自建同源`).toBe(
      str(created.contract_no)
    );
    // 扫描返回按剩余天数升序（最紧急在前），逐对钉序
    for (let i = 1; i < warnings.length; i++) {
      expect(
        num(warnings[i].days_until_expiry) >= num(warnings[i - 1].days_until_expiry),
        `预警数组应按 days_until_expiry 升序，实际第 ${i} 项 ${warnings[i].days_until_expiry} 小于前项 ${warnings[i - 1].days_until_expiry}`
      ).toBe(true);
    }

    // 打印腿：docx 文件流（属可达性与响应头契约证据，不是数据行契约）
    const printResp = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/labor-contracts/${created.id}/print`
    );
    expect(printResp.status(), `打印应 200，实际 ${printResp.status()}`).toBe(200);
    expect(
      printResp.headers()['content-type'],
      `打印 content-type 应为 docx MIME，实际 ${printResp.headers()['content-type']}`
    ).toContain('wordprocessingml.document');
    const docx = await printResp.body();
    expect(docx.byteLength, `docx 字节应非空（zip 容器至少含 PK 魔数）`).toBeGreaterThan(0);
    expect(docx[0], `docx 魔数字节 1`).toBe(0x50);
    expect(docx[1], `docx 魔数字节 2`).toBe(0x4b);

    // 未知主键打印腿钉 404 JSON 信封（not_found 脱敏），与文件流形态分离
    const absent = { probeId: 9_399_999 };
    const failPrint = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/labor-contracts/${absent.probeId}/print`
    );
    expect(failPrint.status(), `未知合同打印应恰 404，实际 ${failPrint.status()}`).toBe(404);
    const failBody = await failPrint.json();
    expect(failureCode(failBody)).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(String(failBody.message ?? ''), `not_found 出参必须脱敏`).toBe('资源未找到');

    // 收口：终止自建行，避免污染后续扫描
    await apiCall(page, 'POST', `/labor-contracts/${created.id}/terminate`, {
      termination_date: todayYmd(),
      termination_reason: 'E2E 收口',
    });
  });
});
