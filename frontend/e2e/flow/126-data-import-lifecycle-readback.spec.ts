// [法#3 范围锁定] 目标：glob='frontend/e2e/flow/122-data-import-lifecycle-readback.spec.ts'
// 数据导入域任务生命周期与模板契约回读 E2E — 122
//
// 功能：把 data-import 域十二枚零覆盖端点钉成"写后同作用域回读"真值链，
// 覆盖任务创建、列表、详情、错误日志、取消、重试与模板列表、详情、更新、
// 删除拒绝、模板文件下载（GET 与 POST 两条腿）。
// 调用方：路由注册在 backend/src/routes（data-import 分组），前端调用面在
// frontend/src/api/data-import.ts；本文件用 flow/helpers 的 apiCall 系列直连契约面。
// 入参：任务创建走 multipart（template_id 与 file 两个字段），template_id 取
// 内存模板存储的内置行（products、customers、inventory 三条）；文件字节仅登记
// 文件名不做解析（handler 为最小实现，真实解析走 import/excel 另一入口）。
// 传给谁：ImportExportService 落 import_tasks 表；模板存储为进程内 OnceLock
// Mutex（import_export_handler 的 import_template_store），重启即重建，不落库。
// 存什么：任务行落 import_type、status、total_rows、imported_rows、failed_rows、
// user_id、file_name、template_id；缺值列写 NULL，不写 0。
// 存哪里：import_tasks 表；模板记录仅存于内存存储，内置 id 1 到 3。
//
// 本文件覆盖的端点（十二枚全量）：
//   GET data-import/tasks、GET data-import/tasks/{id}、
//   GET data-import/tasks/{id}/error-log、POST data-import/tasks、
//   POST data-import/tasks/{id}/cancel、POST data-import/tasks/{id}/retry、
//   GET data-import/templates、GET data-import/templates/{id}、
//   PUT data-import/templates/{id}、DELETE data-import/templates/{id}、
//   GET 与 POST data-import/templates/{id}/download。
// 清理与隔离事实：任务无删除端点，用例把每条自建任务推到 cancelled 终态收口；
// 文件与合同号均带本轮唯一标记，断言只按自建 id 或唯一标记回读，
// 不依赖库是干净的。模板存储是共享内置三条：PUT 采用改后还原（当场写回原值），
// DELETE 只钉不存在 id 的拒绝腿并回读证零误删——内置模板无自建入口，
// 删除正腿不可恢复，属残留登记。
// 脱敏事实：错误日志端点返回 JSON 信封（非文件流），前端以 blob 接收后可另存；
// 下载端点返回 xlsx 文件流，属可达性与响应头契约证据，不是数据行契约。
// 失败信封口径：AppError 默认脱敏（NOT_FOUND 出参恒为资源未找到，
// BAD_REQUEST 恒为请求参数错误，BUSINESS_ERROR 恒为业务处理失败），
// 仅 validator 与 validation_displayable 构造的 VALIDATION_ERROR 外显真实文案。
import { test, expect, type Page } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  API_BASE,
  API_PREFIX,
  apiCallExpectFail,
  apiCallRaw,
  failureCode,
  genCode,
  loginViaUI,
} from './helpers';

type Row = Record<string, unknown>;
const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

/** 从会话 cookie 取当前存活 csrf token（multipart 与 POST 下载腿需自带请求头） */
async function csrfFromCookie(page: Page): Promise<string> {
  const cookies = await page.context().cookies();
  const csrf = cookies.find(c => c.name === 'csrf_token');
  expect(csrf, '122 会话缺少 csrf_token cookie，应已登录').toBeTruthy();
  return csrf?.value ?? '';
}

test.describe.serial('122 数据导入任务生命周期与模板契约', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('122-01 任务全链：上传建任务 running 到 cancel 到 retry 到 cancel 逐步回读', async ({
    page,
  }) => {
    const templates = await apiCallRaw<Row[]>(page, 'GET', '/data-import/templates');
    expect(Array.isArray(templates), `模板列表应出数组，实际 ${JSON.stringify(templates)}`).toBe(
      true
    );
    expect(
      templates.length,
      `内存模板存储应预置内置模板行，实际 ${templates.length} 条`
    ).toBeGreaterThanOrEqual(1);
    const productsTpl = templates.find(t => str(t.import_type) === 'products');
    expect(productsTpl, `应存在内置 products 模板，实际 ${JSON.stringify(templates)}`).toBeTruthy();
    const productsTplId = num(productsTpl?.id);
    expect(
      Number.isInteger(productsTplId) && productsTplId > 0,
      `内置模板行应带正整数主键，实际 ${JSON.stringify(productsTpl)}`
    ).toBe(true);
    const taskFileName = `${genCode('E2E-DI')}.xlsx`;

    const csrf = await csrfFromCookie(page);
    const createResp = await page.request.fetch(`${API_BASE}${API_PREFIX}/data-import/tasks`, {
      method: 'POST',
      headers: {
        'X-Requested-With': 'XMLHttpRequest',
        'X-CSRF-Token': csrf,
      },
      multipart: {
        template_id: String(productsTplId),
        file: {
          name: taskFileName,
          mimeType: 'application/octet-stream',
          // multipart 的 buffer 成员只接受 Buffer，传 Uint8Array 运行期报
          // Unexpected buffer type of 'data.file'，as unknown as never 只会把这条契约错
          // 从类型面挪到运行期。e2e 类型面本就在使用 Buffer 全局。
          buffer: Buffer.from('E2E minimal upload', 'utf-8'),
        },
      },
    });
    expect(createResp.status(), `建任务应 200，实际 ${createResp.status()}`).toBe(200);
    const created = await createResp.json();
    expect(created.code, `建任务信封 code 应为 200，实际 ${JSON.stringify(created)}`).toBe(200);
    expect(
      String(created.message ?? ''),
      `建任务成功文案应为固定 message，实际 ${created.message}`
    ).toBe('导入任务已创建');
    const taskId = num(created.data.id);
    expect(
      Number.isInteger(taskId) && taskId > 0,
      `建任务应回自增主键，实际 ${JSON.stringify(created)}`
    ).toBe(true);
    expect(
      String(created.data.status),
      `新建任务初始状态应为 running，实际 ${created.data.status}`
    ).toBe('running');
    expect(
      String(created.data.task_code),
      `task_code 应为 IMP-六位补零编号，实际 ${created.data.task_code}`
    ).toBe(`IMP-${String(taskId).padStart(6, '0')}`);
    expect(str(created.data.file_name), `详情应回显上传文件名`).toBe(taskFileName);
    expect(num(created.data.template_id), `template_id 应落内置 products 模板 id`).toBe(
      productsTplId
    );

    // 列表腿：按自建 id 找回自己（不做计数）
    const tasks = await apiCallRaw<Row[]>(page, 'GET', '/data-import/tasks');
    expect(Array.isArray(tasks), `任务列表应出数组，实际 ${JSON.stringify(tasks)}`).toBe(true);
    const mine = tasks.find(t => num(t.id) === taskId);
    expect(
      mine,
      `列表应含自建任务 ${taskId}，实际 ${JSON.stringify(tasks.slice(0, 5))}`
    ).toBeTruthy();
    expect(str(mine?.status), `列表行状态应为 running`).toBe('running');
    expect(str(mine?.import_type), `列表行 import_type 应取模板类型`).toBe('products');
    expect(
      Number.isNaN(Date.parse(str(mine?.created_at))),
      `列表行 created_at 应可解析为时间，实际 ${mine?.created_at}`
    ).toBe(false);

    // 详情腿：全键回读（DTO 形状与聚合恒等式）
    const detail = await apiCallRaw<Row>(page, 'GET', `/data-import/tasks/${created.data.id}`);
    expect(num(detail.id), `详情 id 应为自建任务`).toBe(taskId);
    expect(str(detail.file_name), `详情应逐字落库上传文件名`).toBe(taskFileName);
    expect(str(detail.status), `详情初始状态 running`).toBe('running');
    expect(num(detail.processed_rows), `processed_rows 应等于 success_rows 加 failed_rows`).toBe(
      num(detail.success_rows) + num(detail.failed_rows)
    );
    expect(num(detail.total_rows), `上传建任务不解析行数，total_rows 应落 0`).toBe(0);
    expect(str(detail.template_name), `template_name 取任务 import_type`).toBe('products');

    // 错误日志腿：JSON 信封，failed_rows 为 0 时 content 为空串
    const errLog = await apiCallRaw<Row>(
      page,
      'GET',
      `/data-import/tasks/${created.data.id}/error-log`
    );
    expect(num(errLog.task_id), `错误日志应回显任务 id`).toBe(taskId);
    expect(str(errLog.task_code), `错误日志 task_code 与详情同源`).toBe(
      `IMP-${String(taskId).padStart(6, '0')}`
    );
    expect(str(errLog.filename), `错误日志文件名应含任务 id`).toBe(
      `import_task_${taskId}_errors.log`
    );
    expect(str(errLog.content), `零失败行时错误日志 content 应为空串，实际 ${errLog.content}`).toBe(
      ''
    );

    // 状态门负例：running 任务重试应被拒（BUSINESS_ERROR 脱敏文案）
    const retryWhileRunning = await apiCallExpectFail(
      page,
      'POST',
      `/data-import/tasks/${created.data.id}/retry`
    );
    expect(retryWhileRunning.status).toBe(400);
    expect(failureCode(retryWhileRunning), `running 重试拒绝码`).toBe(
      APP_ERROR_CODES.BUSINESS_ERROR
    );
    expect(
      String(retryWhileRunning.message ?? ''),
      `business 出参必须脱敏，实际 ${retryWhileRunning.message}`
    ).toBe('业务处理失败');
    const stillRunning = await apiCallRaw<Row>(
      page,
      'GET',
      `/data-import/tasks/${created.data.id}`
    );
    expect(str(stillRunning.status), `非法重试被拒后状态应保持 running`).toBe('running');

    // 取消正腿 + 回读
    const cancelled = await apiCallRaw<Row>(
      page,
      'POST',
      `/data-import/tasks/${created.data.id}/cancel`
    );
    expect(str(cancelled.status), `取消后返回体状态应为 cancelled`).toBe('cancelled');
    const afterCancel = await apiCallRaw<Row>(page, 'GET', `/data-import/tasks/${created.data.id}`);
    expect(str(afterCancel.status), `取消后详情回读应为 cancelled`).toBe('cancelled');

    // 重复取消负例 + 零变更回读
    const cancelAgain = await apiCallExpectFail(
      page,
      'POST',
      `/data-import/tasks/${created.data.id}/cancel`
    );
    expect(cancelAgain.status).toBe(400);
    expect(failureCode(cancelAgain)).toBe(APP_ERROR_CODES.BUSINESS_ERROR);
    expect(String(cancelAgain.message ?? '')).toBe('业务处理失败');
    const stillCancelled = await apiCallRaw<Row>(
      page,
      'GET',
      `/data-import/tasks/${created.data.id}`
    );
    expect(str(stillCancelled.status), `重复取消被拒后应保持 cancelled`).toBe('cancelled');

    // 重试正腿：cancelled 可重试，状态重置 running
    const retried = await apiCallRaw<Row>(
      page,
      'POST',
      `/data-import/tasks/${created.data.id}/retry`
    );
    expect(str(retried.status), `重试后返回体状态应为 running`).toBe('running');
    const afterRetry = await apiCallRaw<Row>(page, 'GET', `/data-import/tasks/${created.data.id}`);
    expect(str(afterRetry.status), `重试后详情回读应为 running`).toBe('running');

    // 收口：把自建任务推到 cancelled 终态（域内无删除端点）
    const finalCancel = await apiCallRaw<Row>(
      page,
      'POST',
      `/data-import/tasks/${created.data.id}/cancel`
    );
    expect(str(finalCancel.status), `收口取消后应为 cancelled`).toBe('cancelled');
  });

  test('122-02 任务面负例：缺模板与未知模板与未知任务，回读证零落库', async ({ page }) => {
    const marker = genCode('E2E-DI-NEG');

    // 拒绝前记录在册最大主键：被拒的插入若泄漏成行，其主键必然大于该水位
    const before = await apiCallRaw<Row[]>(page, 'GET', '/data-import/tasks');
    expect(Array.isArray(before), `拒绝前任务列表应出数组`).toBe(true);
    const maxIdBefore = before.reduce((acc, t) => Math.max(acc, num(t.id)), 0);

    // 缺 template_id 字段：bad_request 构造，出参脱敏为请求参数错误
    const csrfMiss = await csrfFromCookie(page);
    const missResp = await page.request.fetch(`${API_BASE}${API_PREFIX}/data-import/tasks`, {
      method: 'POST',
      headers: { 'X-Requested-With': 'XMLHttpRequest', 'X-CSRF-Token': csrfMiss },
      multipart: {
        file: {
          name: `${marker}.xlsx`,
          mimeType: 'application/octet-stream',
          buffer: Buffer.from('E2E negative probe', 'utf-8'),
        },
      },
    });
    expect(missResp.status(), `缺 template_id 应恰 400，实际 ${missResp.status()}`).toBe(400);
    const missBody = await missResp.json();
    expect(failureCode(missBody), `缺模板字段机器码`).toBe(APP_ERROR_CODES.BAD_REQUEST);
    expect(String(missBody.message ?? ''), `bad_request 出参必须脱敏`).toBe('请求参数错误');

    // 未知模板 id：not_found 构造，出参脱敏为资源未找到
    const csrfUnknown = await csrfFromCookie(page);
    const unknownResp = await page.request.fetch(`${API_BASE}${API_PREFIX}/data-import/tasks`, {
      method: 'POST',
      headers: { 'X-Requested-With': 'XMLHttpRequest', 'X-CSRF-Token': csrfUnknown },
      multipart: {
        template_id: '999999',
        file: {
          name: `${marker}.xlsx`,
          mimeType: 'application/octet-stream',
          buffer: Buffer.from('E2E negative probe', 'utf-8'),
        },
      },
    });
    expect(unknownResp.status(), `未知模板应恰 404，实际 ${unknownResp.status()}`).toBe(404);
    const unknownBody = await unknownResp.json();
    expect(failureCode(unknownBody)).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(String(unknownBody.message ?? ''), `not_found 出参必须脱敏`).toBe('资源未找到');

    // 回读证零落库：拒绝后新出现的行（主键大于水位）不得携带本轮唯一标记，
    // 两条被拒上传都在 service 建行之前的参数门处返回
    const after = await apiCallRaw<Row[]>(page, 'GET', '/data-import/tasks');
    const newerIds = after.filter(t => num(t.id) > maxIdBefore).map(t => num(t.id));
    for (const id of newerIds) {
      const leaked = await apiCallRaw<Row>(page, 'GET', `/data-import/tasks/${id}`);
      expect(
        str(leaked.file_name),
        `被拒上传（标记 ${marker}）不得落库，实际发现新行 id ${id} 文件名 ${leaked.file_name}`
      ).not.toBe(`${marker}.xlsx`);
    }

    // 未知任务三枚读腿与两枚动作腿各自 404
    const absent = { probeId: 99_001_999 };
    const failDetail = await apiCallExpectFail(page, 'GET', `/data-import/tasks/${absent.probeId}`);
    expect(failDetail.status).toBe(404);
    expect(failureCode(failDetail)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const failLog = await apiCallExpectFail(
      page,
      'GET',
      `/data-import/tasks/${absent.probeId}/error-log`
    );
    expect(failLog.status).toBe(404);
    expect(failureCode(failLog)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const failCancel = await apiCallExpectFail(
      page,
      'POST',
      `/data-import/tasks/${absent.probeId}/cancel`
    );
    expect(failCancel.status).toBe(404);
    expect(failureCode(failCancel)).toBe(APP_ERROR_CODES.NOT_FOUND);
    const failRetry = await apiCallExpectFail(
      page,
      'POST',
      `/data-import/tasks/${absent.probeId}/retry`
    );
    expect(failRetry.status).toBe(404);
    expect(failureCode(failRetry)).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('122-03 模板读面与下载双腿：列表 详情 xlsx 文件流 未知模板 404', async ({ page }) => {
    const templates = await apiCallRaw<Row[]>(page, 'GET', '/data-import/templates');
    expect(Array.isArray(templates), `模板列表应出数组`).toBe(true);
    const first = templates[0];
    expect(first, `模板列表应含内置模板行，实际 ${JSON.stringify(templates)}`).toBeTruthy();
    // 模板 DTO 形状：逐键点名，缺键即形漂判红
    for (const key of [
      'id',
      'import_type',
      'template_code',
      'template_name',
      'module',
      'file_format',
      'columns',
      'sample_data',
      'status',
      'created_at',
      'updated_at',
    ]) {
      expect(
        Object.prototype.hasOwnProperty.call(first, key),
        `模板行应含键 ${key}，实际键集 ${JSON.stringify(Object.keys(first))}`
      ).toBe(true);
    }
    expect(str(first.status), `内置模板状态应为 active，实际 ${first.status}`).toBe('active');
    expect(
      Array.isArray(first.columns) && (first.columns as Row[]).length,
      `模板列定义应非空`
    ).toBeTruthy();
    const col0 = (first.columns as Row[])[0];
    for (const key of ['key', 'label', 'type', 'required']) {
      expect(
        Object.prototype.hasOwnProperty.call(col0, key),
        `模板列定义应含键 ${key}，实际 ${JSON.stringify(col0)}`
      ).toBe(true);
    }

    const detail = await apiCallRaw<Row>(page, 'GET', `/data-import/templates/${first.id}`);
    expect(num(detail.id), `详情 id 应与列表行一致`).toBe(num(first.id));
    expect(str(detail.file_format), `内置模板文件格式 xlsx`).toBe('xlsx');
    expect(
      str(detail.template_code),
      `template_code 应为 IMP_ 加大写类型，实际 ${detail.template_code}`
    ).toBe(`IMP_${str(detail.import_type).toUpperCase()}`);

    // 下载双腿（GET 与 POST）：xlsx 文件流，属可达性与响应头契约证据
    const dlGet = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/data-import/templates/${first.id}/download`
    );
    expect(dlGet.status(), `GET 下载应 200，实际 ${dlGet.status()}`).toBe(200);
    expect(
      dlGet.headers()['content-type'],
      `GET 下载 content-type 应为 xlsx MIME，实际 ${dlGet.headers()['content-type']}`
    ).toContain('spreadsheetml.sheet');
    expect(
      dlGet.headers()['content-disposition'],
      `GET 下载应带 attachment 文件名，实际 ${dlGet.headers()['content-disposition']}`
    ).toContain(`${str(detail.import_type)}_template.xlsx`);
    const getBody = await dlGet.body();
    expect(getBody.byteLength, `GET 下载字节应非空（xlsx 至少含 zip 魔数）`).toBeGreaterThan(0);
    // xlsx 实为 zip 容器，前两字节必须是 PK
    expect(getBody[0], `xlsx 魔数字节 1`).toBe(0x50);
    expect(getBody[1], `xlsx 魔数字节 2`).toBe(0x4b);

    const csrfPost = await csrfFromCookie(page);
    const dlPost = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/data-import/templates/${first.id}/download`,
      {
        method: 'POST',
        headers: { 'X-Requested-With': 'XMLHttpRequest', 'X-CSRF-Token': csrfPost },
      }
    );
    expect(dlPost.status(), `POST 下载应 200，实际 ${dlPost.status()}`).toBe(200);
    expect(dlPost.headers()['content-type']).toContain('spreadsheetml.sheet');
    const postBody = await dlPost.body();
    expect(
      postBody.byteLength,
      `POST 下载字节数应与 GET 同模板同源，实际 GET ${getBody.byteLength} POST ${postBody.byteLength}`
    ).toBe(getBody.byteLength);

    // 未知模板三条读腿各钉 404（not_found 脱敏）
    const absent = { probeId: 99_002_999 };
    const failTpl = await apiCallExpectFail(
      page,
      'GET',
      `/data-import/templates/${absent.probeId}`
    );
    expect(failTpl.status).toBe(404);
    expect(failureCode(failTpl)).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(String(failTpl.message ?? '')).toBe('资源未找到');
    const failDlGet = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/data-import/templates/${absent.probeId}/download`
    );
    expect(failDlGet.status(), `未知模板下载 GET 应 404`).toBe(404);
    const failDlPost = await page.request.fetch(
      `${API_BASE}${API_PREFIX}/data-import/templates/${absent.probeId}/download`,
      {
        method: 'POST',
        headers: {
          'X-Requested-With': 'XMLHttpRequest',
          'X-CSRF-Token': await csrfFromCookie(page),
        },
      }
    );
    expect(failDlPost.status(), `未知模板下载 POST 应 404`).toBe(404);
  });

  test('122-04 模板更新：局部提交未提交列保原值，校验负例零落库，改后还原', async ({ page }) => {
    const templates = await apiCallRaw<Row[]>(page, 'GET', '/data-import/templates');
    const target = templates.find(t => str(t.import_type) === 'products');
    expect(target, `应存在内置 products 模板`).toBeTruthy();
    const originalName = str(target?.template_name);
    const originalDesc = str(target?.description);
    const marker = genCode('E2E-TPL');

    // 局部更新只提交 template_name：description 等未提交列必须保持原值
    const updated = await apiCallRaw<Row>(page, 'PUT', `/data-import/templates/${target?.id}`, {
      template_name: `E2E 改名 ${marker}`,
    });
    expect(str(updated.template_name), `更新后模板名应为提交值`).toBe(`E2E 改名 ${marker}`);
    expect(str(updated.description), `未提交列 description 应保持原值`).toBe(originalDesc);
    expect(str(updated.import_type), `更新不应触碰 import_type`).toBe('products');
    const readback = await apiCallRaw<Row>(page, 'GET', `/data-import/templates/${target?.id}`);
    expect(str(readback.template_name), `详情回读应见新名`).toBe(`E2E 改名 ${marker}`);
    expect(
      str(readback.updated_at),
      `更新后 updated_at 应刷新且可解析，实际 ${readback.updated_at}`
    ).not.toBe(str(target?.updated_at));

    // 校验负例：名称超长走 validator（displayable 外显真实文案），回读证零写入
    const tooLong = '长'.repeat(101);
    const failLong = await apiCallExpectFail(page, 'PUT', `/data-import/templates/${target?.id}`, {
      template_name: tooLong,
    });
    expect(failLong.status, `超长模板名应恰 400，实际 ${failLong.status}`).toBe(400);
    expect(failureCode(failLong), `validator 拒绝机器码`).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(
      String(failLong.message ?? ''),
      `validation_displayable 出参必须外显真实校验原因，实际 ${failLong.message}`
    ).toContain('模板名称长度须为 1-100 字符');
    const afterFail = await apiCallRaw<Row>(page, 'GET', `/data-import/templates/${target?.id}`);
    expect(str(afterFail.template_name), `被拒更新后名称应保持本轮标记值`).toBe(
      `E2E 改名 ${marker}`
    );

    // 未知模板更新 404（not_found 脱敏），内存存储零变化回读
    const absent = { probeId: 99_003_999 };
    const failAbsent = await apiCallExpectFail(
      page,
      'PUT',
      `/data-import/templates/${absent.probeId}`,
      {
        template_name: `E2E 幽灵 ${marker}`,
      }
    );
    expect(failAbsent.status).toBe(404);
    expect(failureCode(failAbsent)).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(String(failAbsent.message ?? '')).toBe('资源未找到');

    // 还原：内置模板是共享内存存储，改后当场写回原值
    const restored = await apiCallRaw<Row>(page, 'PUT', `/data-import/templates/${target?.id}`, {
      template_name: originalName,
    });
    expect(str(restored.template_name), `还原后名称应为原值`).toBe(originalName);
    const finalCheck = await apiCallRaw<Row>(page, 'GET', `/data-import/templates/${target?.id}`);
    expect(str(finalCheck.description), `全程未提交 description 恒为原值`).toBe(originalDesc);
  });

  test('122-05 模板删除拒绝腿：未知模板 404 且回读证零误删', async ({ page }) => {
    const before = await apiCallRaw<Row[]>(page, 'GET', '/data-import/templates');
    expect(Array.isArray(before), `删除前列表应出数组`).toBe(true);

    const absent = { probeId: 99_004_999 };
    const failDel = await apiCallExpectFail(
      page,
      'DELETE',
      `/data-import/templates/${absent.probeId}`
    );
    expect(failDel.status, `删除未知模板应恰 404，实际 ${failDel.status}`).toBe(404);
    expect(failureCode(failDel), `删除拒绝机器码`).toBe(APP_ERROR_CODES.NOT_FOUND);
    expect(String(failDel.message ?? ''), `not_found 出参必须脱敏`).toBe('资源未找到');

    // 回读证零落库：被拒的删除不得动任何在册行（按 id 集合逐条比对，非计数依赖）
    const after = await apiCallRaw<Row[]>(page, 'GET', '/data-import/templates');
    expect(after.length, `被拒删除前后在册行数应相等`).toBe(before.length);
    for (const row of before) {
      const survivor = after.find(t => num(t.id) === num(row.id));
      expect(
        survivor,
        `在册模板 ${row.id} 不应被被拒删除误删，实际删除后列表 ${JSON.stringify(after.map(t => t.id))}`
      ).toBeTruthy();
      expect(str(survivor?.status), `幸存行状态保持`).toBe(str(row.status));
    }
    // 正腿残留登记：内置模板无自建入口，真删不可恢复，本域删除成功腿不覆盖
  });
});
