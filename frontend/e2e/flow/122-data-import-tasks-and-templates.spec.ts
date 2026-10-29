import { test, expect, type Page } from '../diagnose-fixture';
import { type APIResponse } from '@playwright/test';
import {
  API_BASE,
  API_PREFIX,
  APP_ERROR_CODES,
  loginViaUI,
  apiCallRaw,
  apiCallExpectFail,
  type ApiResponse,
} from './helpers';

/**
 * 122 data-import 任务面与模板面真实契约回读（data-import 族，12 条在册端点）
 *
 * 守的是什么：`/data-import/*` 面把提交值真正落库/落内存存储并能逐键读回，
 * 任务状态机（running → cancelled → running → cancelled）按 handler 显式门控拒绝非法迁移，
 * 而不是只证可达。全部判据来自 backend 源码实核：
 * - GET /data-import/templates（import_export_handler::list_import_templates）：
 *   出参为内存预置的内置模板数组（builtin_import_template_records，
 *   import_type 取 products/customers/inventory，template_code 形如 IMP_PRODUCTS）。
 * - GET /data-import/templates/{id}：id 不存在返回 404，机器码 NOT_FOUND
 *   （AppError::not_found 脱敏族，出参文案为固定常量，不含 id 明细）。
 * - PUT /data-import/templates/{id}：仅覆盖请求体出现的列，未出现的保持原值；
 *   更新后 GET 回读必须见新值。本用例只改 description 并在用例尾部还原原文，
 *   不删不建——模板存储为进程级内存（OnceLock<Mutex<Vec>>，），内置条目删后无法重建。
 * - DELETE /data-import/templates/{id}：删除不存在的 id 返回 404 NOT_FOUND。
 *   成功腿不测：面内无 POST 创建腿（路由注册 routes/mod 只有 GET/PUT/DELETE），
 *   删除内置模板会破坏同进程其他并发用例对模板列表的读取，属"只能测负例"的真实残量。
 * - GET|POST /data-import/templates/{id}/download：响应是 xlsx 文件体
 *   （Content-Type 为 spreadsheetml.sheet，Content-Disposition 带 {import_type}_template.xlsx），
 *   魔数按 ZIP 容器判前两点 0x50 0x4B（xlsx 是 zip 包）。
 * - POST /data-import/tasks：multipart 提取 template_id 与 file 名；
 *   缺 template_id → 400 BAD_REQUEST（AppError::bad_request 脱敏族）；
 *   template_id 指向不存在模板 → 404 NOT_FOUND；
 *   成功创建 import_tasks 表行，初始状态 running（services/import_export_ops/task），
 *   出参含 task_code=IMP-{id 补零 6 位}、file_name 回显、total_rows=0（最小实现登记文件名）。
 * - GET /data-import/tasks：DTO 列 id/import_type/status/total_rows/imported_rows/
 *   failed_rows/created_at（ImportTaskItem），新建任务必出现在列表。
 * - GET /data-import/tasks/{id}：详情回显 template_id、file_name、状态。
 * - POST /data-import/tasks/{id}/cancel：仅 running 可取消，否则 400 BUSINESS_ERROR；
 *   取消后状态 cancelled。
 * - POST /data-import/tasks/{id}/retry：running/success 拒绝（400 BUSINESS_ERROR），
 *   cancelled 放行并重置为 running。
 * - GET /data-import/tasks/{id}/error-log：出参 task_id/task_code/status/filename/content，
 *   failed_rows=0 时 content 为空串（build_import_task_error_log）。
 *
 * 隔离与自清：任务行没有删除端点（面内无 DELETE /tasks/{id}），用例把任务推到 cancelled
 * 终态后保留行；所有任务 id 由本用例自建自读，不依赖其他 spec 先验状态。
 * 模板 description 先读原文、改写、回读、还原，改后值与原文都不写死。
 * multipart 走 page.request 的 form（Playwright 自动组 multipart），
 * JSON apiCall 打该端点只会撞 415，测不到创建契约，故不用它。
 */

type TemplateRow = {
  id: number;
  import_type: string;
  template_code: string;
  template_name: string;
  description: string;
  module: string;
  file_format: string;
  columns: Array<{ key: string; label: string; required: boolean }>;
};

type TaskListItem = {
  id: number;
  import_type: string;
  status: string;
  total_rows: number;
  imported_rows: number;
  failed_rows: number;
  created_at: string;
};

type TaskDetail = {
  id: number;
  task_code: string;
  template_id: number;
  template_name: string;
  file_name: string;
  status: string;
  total_rows: number;
  processed_rows: number;
  success_rows: number;
  failed_rows: number;
  error_log: string;
  created_at: string;
  completed_at: string;
};

type ErrorLogBody = {
  task_id: number;
  task_code: string;
  status: string;
  filename: string;
  content: string;
};

// multipart 创建任务的响应体形状（与 import_task_to_frontend_json 同列）。
type CreateTaskEnvelope = ApiResponse<TaskDetail>;

// 从上下文 Cookie 取 csrf 双提交令牌（helpers 的 getCsrfToken 未导出，此处同构自取）。
async function csrfTokenOf(page: Page): Promise<string> {
  const cookies = await page.context().cookies();
  const csrf = cookies.find(c => c.name === 'csrf_token');
  if (!csrf) {
    throw new Error('csrf_token cookie not found — loginViaUI 之后应存在');
  }
  return csrf.value;
}

// 组 multipart 打 POST /data-import/tasks；headers 双提交 CSRF，与 apiCall 同参。
async function postMultipartCreateTask(
  page: Page,
  fields: Record<string, string>,
  fileName: string
): Promise<APIResponse> {
  const token = await csrfTokenOf(page);
  // Playwright 的 multipart buffer 成员只接受 Buffer，传 Uint8Array 运行期报
  // Unexpected buffer type of 'data.file'。
  const csvBytes = Buffer.from('sku,qty\nE2E-ZERO-COV,2\n', 'utf-8');
  return page.request.post(`${API_BASE}${API_PREFIX}/data-import/tasks`, {
    headers: {
      'X-Requested-With': 'XMLHttpRequest',
      'X-CSRF-Token': token,
    },
    multipart: {
      ...fields,
      file: { name: fileName, mimeType: 'text/csv', buffer: csvBytes },
    },
    timeout: 60_000,
  });
}

// 下载腿共判据：xlsx 是 zip 容器（魔数 PK 前两字节），头带表格 MIME 与附件文件名。
async function assertXlsxDownload(res: APIResponse): Promise<void> {
  expect(res.status()).toBe(200);
  const headers = res.headers();
  expect(headers['content-type']).toContain(
    'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
  );
  expect(headers['content-disposition']).toContain('attachment');
  expect(headers['content-disposition']).toContain('products_template.xlsx');
  const bytes = new Uint8Array(await res.body());
  // 空文件不可能出现该两点；模板行内容由 ImportExportService::generate_xlsx 真实生成
  expect(bytes.length).toBeGreaterThan(2);
  expect(bytes[0]).toBe(0x50);
  expect(bytes[1]).toBe(0x4b);
}

test.describe.serial('122 data-import 任务与模板真实契约回读', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('模板列表与详情字段为内存预置真值，未知 id 报 404', async ({ page }) => {
    const list = await apiCallRaw<TemplateRow[]>(page, 'GET', '/data-import/templates');
    expect(Array.isArray(list)).toBe(true);
    expect(list.length).toBeGreaterThan(0);
    const products = list.find(r => r.import_type === 'products');
    expect(products).toBeDefined();
    const tpl = products as unknown as TemplateRow;
    expect(tpl.template_code).toBe('IMP_PRODUCTS');
    expect(tpl.file_format).toBe('xlsx');
    expect(Array.isArray(tpl.columns)).toBe(true);
    expect(tpl.columns.length).toBeGreaterThan(0);
    expect(typeof tpl.columns[0].key).toBe('string');

    const detail = await apiCallRaw<TemplateRow>(page, 'GET', `/data-import/templates/${tpl.id}`);
    expect(detail.id).toBe(tpl.id);
    expect(detail.import_type).toBe('products');

    // 负例：不存在的模板 id → 404 + 机器码 NOT_FOUND（脱敏文案不含 id 明细）
    const missing = await apiCallExpectFail(page, 'GET', '/data-import/templates/987654');
    expect(missing.status).toBe(404);
    expect(missing.code).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('PUT 模板 description 改写后 GET 回读新值并还原原文', async ({ page }) => {
    const before = await apiCallRaw<TemplateRow>(page, 'GET', '/data-import/templates/1');
    const originalDescription = before.description;
    const updatedDescription = `${originalDescription}|e2e-zero-cov-${Date.now().toString(36)}`;

    const updated = await apiCallRaw<TemplateRow>(page, 'PUT', '/data-import/templates/1', {
      description: updatedDescription,
    });
    expect(updated.description).toBe(updatedDescription);
    // 未提交列保持原值（update_import_template 起逐列 Some 才覆盖）
    expect(updated.import_type).toBe(before.import_type);
    expect(updated.template_code).toBe(before.template_code);

    const readBack = await apiCallRaw<TemplateRow>(page, 'GET', '/data-import/templates/1');
    expect(readBack.description).toBe(updatedDescription);

    // 还原原文，避免污染同进程内其他消费者对模板列表的读取
    const restored = await apiCallRaw<TemplateRow>(page, 'PUT', '/data-import/templates/1', {
      description: originalDescription,
    });
    expect(restored.description).toBe(originalDescription);
  });

  test('DELETE 未知模板 404；模板下载 GET 与 POST 返回 xlsx 文件体', async ({ page }) => {
    // 删除成功腿不测：面内无创建腿，内置模板删后无法重建（详见头注释），只钉 404 契约
    const delMissing = await apiCallExpectFail(page, 'DELETE', '/data-import/templates/987654');
    expect(delMissing.status).toBe(404);
    expect(delMissing.code).toBe(APP_ERROR_CODES.NOT_FOUND);

    const csrf = await csrfTokenOf(page);
    const downloadHeaders = {
      'X-Requested-With': 'XMLHttpRequest',
      'X-CSRF-Token': csrf,
    };

    // GET 与 POST 同挂一个 handler（routes/mod 下载路由双方法）；
    // 两条腿各写死字面调用——方法经循环变量传 fetch 时，覆盖判定器按字面量取不到 POST 位点
    const getRes = await page.request.get(
      `${API_BASE}${API_PREFIX}/data-import/templates/1/download`,
      {
        headers: downloadHeaders,
        timeout: 60_000,
      }
    );
    await assertXlsxDownload(getRes);

    const postRes = await page.request.post(
      `${API_BASE}${API_PREFIX}/data-import/templates/1/download`,
      {
        headers: downloadHeaders,
        data: '{}',
        timeout: 60_000,
      }
    );
    await assertXlsxDownload(postRes);
  });

  test('multipart 创建任务落库并按列表、详情、错误日志三处回读', async ({ page }) => {
    const fileName = `e2e-zero-cov-${Date.now().toString(36)}.csv`;
    const res = await postMultipartCreateTask(page, { template_id: '1' }, fileName);
    expect(res.status()).toBe(200);
    const body = (await res.json()) as CreateTaskEnvelope;
    expect(body.code).toBe(200);
    expect(body.message).toBe('导入任务已创建');
    const task = body.data;
    expect(typeof task.id).toBe('number');
    expect(task.id).toBeGreaterThan(0);
    expect(task.task_code).toBe(`IMP-${String(task.id).padStart(6, '0')}`);
    expect(task.status).toBe('running');
    expect(task.template_id).toBe(1);
    expect(task.file_name).toBe(fileName);
    expect(task.total_rows).toBe(0);

    const list = await apiCallRaw<TaskListItem[]>(page, 'GET', '/data-import/tasks');
    expect(Array.isArray(list)).toBe(true);
    const mine = list.find(t => t.id === task.id);
    expect(mine).toBeDefined();
    const listed = mine as unknown as TaskListItem;
    expect(listed.import_type).toBe('products');
    expect(listed.status).toBe('running');

    const detail = await apiCallRaw<TaskDetail>(page, 'GET', `/data-import/tasks/${task.id}`);
    expect(detail.id).toBe(task.id);
    expect(detail.file_name).toBe(fileName);
    expect(detail.status).toBe('running');

    const errLog = await apiCallRaw<ErrorLogBody>(
      page,
      'GET',
      `/data-import/tasks/${task.id}/error-log`
    );
    expect(errLog.task_id).toBe(task.id);
    expect(errLog.task_code).toBe(task.task_code);
    expect(errLog.filename).toBe(`import_task_${task.id}_errors.log`);
    // failed_rows=0 → 错误日志文本为空串（build_import_task_error_log 的显式分支）
    expect(errLog.content).toBe('');

    // 未知任务详情 → 404 NOT_FOUND
    const missing = await apiCallExpectFail(page, 'GET', '/data-import/tasks/987654');
    expect(missing.status).toBe(404);
    expect(missing.code).toBe(APP_ERROR_CODES.NOT_FOUND);

    // 把任务留给终态 cancelled，不让 running 悬挂（cancel 合法迁移）
    const cancelled = await apiCallRaw<TaskDetail>(
      page,
      'POST',
      `/data-import/tasks/${task.id}/cancel`
    );
    expect(cancelled.status).toBe('cancelled');
  });

  test('任务状态机门控：非 running 不可取消、running 不可重试，取消后可重试', async ({ page }) => {
    const res = await postMultipartCreateTask(page, { template_id: '2' }, 'e2e-state-machine.csv');
    expect(res.status()).toBe(200);
    const created = ((await res.json()) as CreateTaskEnvelope).data;
    const id = created.id;

    const cancelled = await apiCallRaw<TaskDetail>(page, 'POST', `/data-import/tasks/${id}/cancel`);
    expect(cancelled.status).toBe('cancelled');
    expect(cancelled.template_id).toBe(2);

    // 已取消再取消 → 400 BUSINESS_ERROR（仅 running 可取消）
    const cancelAgain = await apiCallExpectFail(page, 'POST', `/data-import/tasks/${id}/cancel`);
    expect(cancelAgain.status).toBe(400);
    expect(cancelAgain.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // cancelled 可重试 → 状态重置 running
    const retried = await apiCallRaw<TaskDetail>(page, 'POST', `/data-import/tasks/${id}/retry`);
    expect(retried.status).toBe('running');

    // running 再重试 → 400 BUSINESS_ERROR（仅失败/部分成功/已取消可重试）
    const retryAgain = await apiCallExpectFail(page, 'POST', `/data-import/tasks/${id}/retry`);
    expect(retryAgain.status).toBe(400);
    expect(retryAgain.code).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 收尾推到 cancelled 终态
    const finalCancel = await apiCallRaw<TaskDetail>(
      page,
      'POST',
      `/data-import/tasks/${id}/cancel`
    );
    expect(finalCancel.status).toBe('cancelled');

    // 未知任务的动作腿 → 404 而非 400（存在性检查先于状态门）
    const missingCancel = await apiCallExpectFail(page, 'POST', '/data-import/tasks/987654/cancel');
    expect(missingCancel.status).toBe(404);
    expect(missingCancel.code).toBe(APP_ERROR_CODES.NOT_FOUND);
  });

  test('multipart 缺 template_id 报 400，未知 template_id 报 404', async ({ page }) => {
    const noTemplate = await postMultipartCreateTask(page, {}, 'e2e-no-template.csv');
    expect(noTemplate.status()).toBe(400);
    const noTemplateBody = (await noTemplate.json()) as { code?: string };
    expect(noTemplateBody.code).toBe(APP_ERROR_CODES.BAD_REQUEST);

    const unknownTemplate = await postMultipartCreateTask(
      page,
      { template_id: '987654' },
      'e2e-unknown-template.csv'
    );
    expect(unknownTemplate.status()).toBe(404);
    const unknownBody = (await unknownTemplate.json()) as { code?: string };
    expect(unknownBody.code).toBe(APP_ERROR_CODES.NOT_FOUND);
  });
});
