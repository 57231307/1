import { test, expect, type Page } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  apiCall,
  apiCallExpectFail,
  apiCallRaw,
  failureCode,
  genCode,
  loginViaUI,
  tryCleanup,
} from './helpers';

/**
 * 118 报表增强导出端点与订阅动作真值链（模板导出 两枚直出导出 手动触发 立即发送 启停位）
 *
 * 与既有覆盖的分工。既有把报表订阅只做到标准增删改接与列表可打开，判定器口径下
 * POST templates 对象导出、POST export excel 与 POST export pdf 两枚直出导出、
 * POST subscriptions 手动触发、POST subscriptions 立即发送五枚端点均为零覆盖。
 * 本 spec 把这五枚钉成真实回读链，并把订阅创建的校验门与动作的拒绝钉到机器码
 * 与零漂移两个口径。
 *
 * 契约真相（逐段读后端源，非推测）。
 * 路由装配在 backend/src/routes/analytics.rs 的 reports_enhanced()，最终路径为
 * /api/v1/erp/reports/enhanced 下的各子路径。前端调用方是
 * frontend/src/api/report-enhanced.ts，消费页面是
 * frontend/src/views/report-templates/index.vue 与报表订阅面。
 * handler 在 backend/src/handlers/report_enhanced_handler.rs，
 * 订阅业务与落库在 backend/src/services/report_subscription_service.rs，
 * 模板取数口在 backend/src/services/report_template_service.rs。
 *
 * 三枚导出口的共同前提是先取数再编码。模板对象端点 POST templates 对象 export
 * 入参 TemplateExportRequest 两键 format 与 title 皆可缺省，format 只认 pdf，
 * 其余值一律走 xlsx 档；两枚直出端点 POST export pdf 与 POST export excel
 * 入参 ExportRequest，其中 template_id 是非 Option 的字符串键（传数字即请求解码拒），
 * title 可缺省。取数走 execute_custom_report，而该口在当前实现里对任何模板都只可能
 * 返回错误：带数据源SQL 的行 403，不带的行 400 加 BUSINESS_ERROR 且文案脱敏，
 * 而经 API 建出的模板永远不带数据源SQL（create 与 update 双门一律 403 拒绝）。
 * 因此三枚导出口对自建模板的可达契约就是"400 加 BUSINESS_ERROR"，
 * 且拒绝发生在取数阶段，出参里根本没有 filename size content 这些成功键。
 * 成功分支的 base64 字节流与 zip 魔数与 PDF 头在本车道无法从 e2e 触达，
 * 该面只能由后端对 export_service 与 xlsx_export 与 pdf 构造的单元或集成测试覆盖，
 * 本 spec 不伪造正向断言，只把取数门与前置的模板不存在 404 钉实。
 *
 * 直出端点还多一条前置门：template_id 字符串先 parse 为整数，parse 失败走
 * validation_displayable，即 HTTP 400 加 VALIDATION_ERROR 且出参文案原文
 * "无效的模板ID"。parse 通过但模板不存在则 404 加 NOT_FOUND。
 * 三档拒绝码互不相同，逐档钉住即可分辨是入参形错、对象不存在还是取数门关。
 *
 * 订阅端点。POST subscriptions 入参 CreateSubscriptionRequest，
 * name 与 template_id 与 frequency 与 recipients 四键必填；先按 template_id
 * 查 report_templates 行，缺行即 404，私有且非本人创建即 403；再校收件人非空
 * 与邮箱格式，两者都是 validation_displayable（400 加 VALIDATION_ERROR 加原文）；
 * 最后按 frequency 派生下次执行时间，DAILY 加一天、WEEKLY 加一周、MONTHLY 加三十天，
 * 其余值 400 加"无效的订阅频率"。建行落 report_subscriptions 表，服务端写死
 * status 为 ACTIVE、run_count 与 retry_count 为 0、max_retries 为 3、
 * next_retry_at 与 last_run 三列皆为空，export_format 缺省补 pdf，
 * recipients 以 JSON 数组落库。
 * 对象读口 GET subscriptions 按主键取整行，不做可见性过滤，故可用它做真值回读。
 * POST trigger 只做一件事：把下次执行时间置为当前并刷新更新时间后落库，
 * 不写 run_count 也不写上次执行状态；出参 data 为空对象载荷，成功文案是固定常量。
 * POST send 先按主键取行（缺行 404 加"订阅不存在"语义的 NOT_FOUND），
 * 再调同一 trigger，出参回显 subscription_id 与 name 与 recipients 与 export_format
 * 四键，全部读自库中该行，因此可用它与建行提交值逐键对表。
 * 启停 toggle 入参 ToggleSubscriptionDto 的 enabled 为必填布尔，缺键即请求解码拒；
 * 置为启用时按该实体频率重算下次执行时间，停用与触发只动排期与开关列族。
 * 下次执行时间属可被其它分支按实体频率重算口径调整的派生值，
 * 本 spec 对其只断非空且可解析，不断具体时刻，状态判据钉在启用位翻转本身。
 *
 * 假绿防线。五枚端点每次都真发动作并紧跟一次按主键的独立读回，
 * 订阅的身份列逐键对表建行提交值，导出拒绝后回读模板证明版本与列定义零漂移；
 * 校验门一律钉 HTTP 与机器码，可外显族再钉文案原文，被拒后一律回读列表或对象
 * 证明库里零推进。全程不用 ?? 或 || 兜底，后端必出的键缺失即形漂判红；
 * 造数失败当场红并点名步骤。
 */

type Row = Record<string, unknown>;

const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

/** 时间列只断 非空且可解析，不钉具体时刻（派生口径可被其它分支调整） */
function assertParseableTime(value: unknown, label: string): void {
  expect(
    typeof value === 'string' && value.length > 0,
    `${label} 必须是非空字符串，实际 ${JSON.stringify(value)}`
  ).toBe(true);
  const parsed = Date.parse(String(value));
  expect(Number.isNaN(parsed), `${label} 必须能被解析为时间，实际 ${JSON.stringify(value)}`).toBe(
    false
  );
} /** 自建本 worker 专属模板（订阅外键与导出对象的共同前置），失败即红 */
async function createTemplate(page: Page, tag: string): Promise<Row> {
  const created = await apiCallRaw<Row>(page, 'POST', '/reports/enhanced/templates', {
    name: `118模板${tag}`,
    code: genCode(`RT118${tag}`),
    report_type: 'custom',
    columns: [{ field: 'id', title: 'ID' }],
    description: `118建模板${tag}`,
    is_public: false,
  });
  const templateId = num(created.id);
  expect(
    Number.isInteger(templateId) && templateId > 0,
    `118 模板自建失败，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  return created;
}

/** 自建本 worker 专属订阅，返回整行 */
async function createSubscription(page: Page, templateId: number, tag: string): Promise<Row> {
  const created = await apiCallRaw<Row>(page, 'POST', '/reports/enhanced/subscriptions', {
    name: `118订阅${tag}`,
    template_id: templateId,
    frequency: 'WEEKLY',
    recipients: [`rpt${tag}@example.com`, `rpt${tag}-cc@example.com`],
    export_format: 'xlsx',
    is_enabled: false,
  });
  const subscriptionId = num(created.id);
  expect(
    Number.isInteger(subscriptionId) && subscriptionId > 0,
    `118 订阅自建失败，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  return created;
}

test.describe.serial('118 报表增强导出端点与订阅动作真值链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('T1 模板对象导出与两枚直出导出 三档拒绝码各自归因 拒绝后模板零漂移', async ({ page }) => {
    const tag = genCode('T1');
    const created = await createTemplate(page, tag);
    const templateId = num(created.id);
    const name = str(created.name);
    const columnsJson = JSON.stringify(created.columns);

    // 一 模板对象导出端点：pdf 档与缺省档都先撞取数门，出参不含任何文件键
    const tplPdf = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/templates/${templateId}/export`,
      { format: 'pdf', title: `118对象导出${tag}` }
    );
    expect(
      tplPdf.status,
      `模板对象导出 pdf 档必须被取数门拒为 400，实际 ${tplPdf.status} code=${JSON.stringify(tplPdf.code)}`
    ).toBe(400);
    expect(
      failureCode(tplPdf),
      `模板对象导出 pdf 档必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(tplPdf.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const tplDefault = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/templates/${templateId}/export`,
      {}
    );
    expect(tplDefault.status, `模板对象导出缺省档必须 400，实际 ${tplDefault.status}`).toBe(400);
    expect(
      failureCode(tplDefault),
      `模板对象导出缺省档必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(tplDefault.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 二 两枚直出端点：合法 template_id 同样撞取数门
    const directPdf = await apiCallExpectFail(page, 'POST', '/reports/enhanced/export/pdf', {
      template_id: String(templateId),
      title: `118直出PDF${tag}`,
    });
    expect(
      directPdf.status,
      `直出 PDF 必须被取数门拒为 400，实际 ${directPdf.status} code=${JSON.stringify(directPdf.code)}`
    ).toBe(400);
    expect(
      failureCode(directPdf),
      `直出 PDF 必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(directPdf.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const directExcel = await apiCallExpectFail(page, 'POST', '/reports/enhanced/export/excel', {
      template_id: String(templateId),
    });
    expect(
      directExcel.status,
      `直出 Excel 必须被取数门拒为 400，实际 ${directExcel.status} code=${JSON.stringify(directExcel.code)}`
    ).toBe(400);
    expect(
      failureCode(directExcel),
      `直出 Excel 必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(directExcel.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 三 直出端点的入参形门：template_id 是非 Option 字符串键，数字即解码拒
    const numericShape = await apiCallExpectFail(page, 'POST', '/reports/enhanced/export/pdf', {
      template_id: templateId,
    });
    expect(
      numericShape.status,
      `template_id 传数字必须被解码门拒为 400，实际 ${numericShape.status}`
    ).toBe(400);
    expect(
      failureCode(numericShape),
      `template_id 形错必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(numericShape.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 四 直出端点的 parse 门：字符串不可转整数，走可外显校验并原文点名
    const badId = await apiCallExpectFail(page, 'POST', '/reports/enhanced/export/excel', {
      template_id: 'not-a-number',
    });
    expect(badId.status, `template_id 不可解析必须 400，实际 ${badId.status}`).toBe(400);
    expect(
      failureCode(badId),
      `template_id 不可解析必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(badId.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(str(badId.message), 'template_id 不可解析的出参文案必须是后端原文').toBe('无效的模板ID');

    // 五 前置对象门：三枚端点对不存在的主键都先 404，与取数门 400 可分辨
    const objectMissing = await apiCallExpectFail(
      page,
      'POST',
      '/reports/enhanced/templates/99999999/export',
      {}
    );
    expect(objectMissing.status, `不存在主键的对象导出必须 404，实际 ${objectMissing.status}`).toBe(
      404
    );
    expect(failureCode(objectMissing), '不存在主键的对象导出必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const pdfMissing = await apiCallExpectFail(page, 'POST', '/reports/enhanced/export/pdf', {
      template_id: '99999999',
    });
    expect(pdfMissing.status, `不存在主键的直出 PDF 必须 404，实际 ${pdfMissing.status}`).toBe(404);
    expect(failureCode(pdfMissing), '不存在主键的直出 PDF 必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const excelMissing = await apiCallExpectFail(page, 'POST', '/reports/enhanced/export/excel', {
      template_id: '99999999',
    });
    expect(
      excelMissing.status,
      `不存在主键的直出 Excel 必须 404，实际 ${excelMissing.status}`
    ).toBe(404);

    // 拒绝零副作用回读：六次导出尝试都不许改动模板本体
    const after = await apiCallRaw<Row>(page, 'GET', `/reports/enhanced/templates/${templateId}`);
    expect(num(after.version), '被拒的导出不许推进模板版本').toBe(1);
    expect(str(after.name), '被拒的导出不许改动模板名称').toBe(name);
    expect(str(after.status), '被拒的导出不许改动模板状态位').toBe('ACTIVE');
    expect(JSON.stringify(after.columns), '被拒的导出不许改动模板列定义').toBe(columnsJson);
    expect(after.data_source_sql, '被拒的导出不许写入数据源SQL').toBeNull();

    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/templates/${templateId}`,
      `[118] T1 ${templateId}`
    );
  });

  test('T2 订阅整行建读 启停状态位翻转 手动触发与立即发送逐键回读', async ({ page }) => {
    const tag = genCode('T2');
    const createdTpl = await createTemplate(page, tag);
    const templateId = num(createdTpl.id);
    const sub = await createSubscription(page, templateId, tag);
    const subscriptionId = num(sub.id);
    const recipients = [`rpt${tag}@example.com`, `rpt${tag}-cc@example.com`];

    // 建行出参整行的服务端派生列逐键对表
    expect(str(sub.name), '订阅回显 name 必须等于提交名').toBe(`118订阅${tag}`);
    expect(num(sub.template_id), '订阅回显外键必须等于自建模板主键').toBe(templateId);
    expect(str(sub.frequency), '订阅回显频率必须等于 WEEKLY').toBe('WEEKLY');
    expect(
      Array.isArray(sub.recipients),
      `订阅回显收件人必须是 JSON 数组，实际 ${JSON.stringify(sub.recipients)}`
    ).toBe(true);
    expect(sub.recipients as string[], '收件人数组必须逐枚等于提交值').toEqual(recipients);
    expect(str(sub.export_format), '导出格式必须回显提交值 xlsx').toBe('xlsx');
    expect(sub.is_enabled, '提交 false 时启用位必须为 false').toBe(false);
    expect(str(sub.status), '建行状态位必须为 ACTIVE').toBe('ACTIVE');
    expect(num(sub.run_count), '建行执行次数必须为 0').toBe(0);
    expect(num(sub.retry_count), '建行重试次数必须为 0').toBe(0);
    expect(num(sub.max_retries), '建行最大重试必须为服务端默认 3').toBe(3);
    expect(sub.next_retry_at, '建行下次重试时间必须为空').toBeNull();
    expect(sub.last_run_status, '建行上次执行状态必须为空').toBeNull();
    expect(sub.last_run_error, '建行上次执行错误必须为空').toBeNull();
    expect(
      Number.isInteger(num(sub.created_by)) && num(sub.created_by) > 0,
      `建行创建人必须是会话用户主键，实际 ${JSON.stringify(sub.created_by)}`
    ).toBe(true);

    // 独立读口互证，防"响应回显但没落库"
    const detail = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions/${subscriptionId}`
    );
    expect(num(detail.id), '对象读口主键必须等于建行主键').toBe(subscriptionId);
    expect(str(detail.name), '对象读口名称必须等于提交名').toBe(`118订阅${tag}`);
    expect(num(detail.template_id), '对象读口外键必须等于自建模板主键').toBe(templateId);
    expect(detail.recipients as string[], '对象读口收件人必须逐枚等于提交值').toEqual(recipients);
    expect(str(detail.export_format), '对象读口导出格式必须为 xlsx').toBe('xlsx');
    assertParseableTime(detail.created_at, '建行创建时间');

    // 列表信封按外键过滤，四键恒在
    const list = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions?template_id=${templateId}&page=1&page_size=20`
    );
    expect(Array.isArray(list.items), '订阅列表必须出 items 数组').toBe(true);
    const rows = list.items as Row[];
    expect(rows.length, `按外键过滤必须恰命中自建那一行，实际 ${JSON.stringify(rows)}`).toBe(1);
    expect(num(rows[0].id), '列表行的主键必须等于建行主键').toBe(subscriptionId);
    expect(num(list.total), '列表 total 必须等于命中行数').toBe(1);
    expect(num(list.page), '列表 page 必须回显生效页码 1').toBe(1);
    expect(num(list.page_size), '列表 page_size 必须回显生效页长 20').toBe(20);

    // 启停状态位翻转（只钉状态位，下次执行时间只断非空可解析）
    const offToOn = await apiCallRaw<Row>(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${subscriptionId}/toggle`,
      { enabled: true }
    );
    expect(offToOn.is_enabled, 'toggle 启用后回显启用位必须为 true').toBe(true);
    const afterOn = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions/${subscriptionId}`
    );
    expect(afterOn.is_enabled, 'toggle 启用必须真落库（对象读回 true）').toBe(true);
    assertParseableTime(afterOn.next_run_at, 'toggle 启用后的下次执行时间');
    expect(str(afterOn.name), 'toggle 不许改动订阅名称').toBe(`118订阅${tag}`);
    expect(afterOn.recipients as string[], 'toggle 不许改动收件人').toEqual(recipients);

    const onToOff = await apiCallRaw<Row>(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${subscriptionId}/toggle`,
      { enabled: false }
    );
    expect(onToOff.is_enabled, 'toggle 禁用后回显启用位必须为 false').toBe(false);
    const afterOff = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions/${subscriptionId}`
    );
    expect(afterOff.is_enabled, 'toggle 禁用必须真落库（对象读回 false）').toBe(false);

    // 手动触发：成功出参是固定文案，动作只置排期不产执行结果
    const triggered = await apiCall<unknown>(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${subscriptionId}/trigger`,
      {}
    );
    expect(str(triggered.message), '手动触发的成功文案必须是后端固定常量').toBe('报表订阅已触发');
    expect(
      triggered.data === null,
      `手动触发不出业务载荷，实际 ${JSON.stringify(triggered.data)}`
    ).toBe(true);
    const afterTrigger = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions/${subscriptionId}`
    );
    expect(num(afterTrigger.id), '触发后对象读回必须还是那一行').toBe(subscriptionId);
    expect(str(afterTrigger.status), '触发不许改动状态位').toBe('ACTIVE');
    expect(afterTrigger.is_enabled, '触发不许改动启用位').toBe(false);
    expect(num(afterTrigger.run_count), '触发只置排期，不产执行结果，次数仍为 0').toBe(0);
    expect(afterTrigger.last_run_status, '触发不写上次执行状态').toBeNull();
    assertParseableTime(afterTrigger.next_run_at, '触发后的下次执行时间');

    // 立即发送：出参四键全部读自库中该行，与建行提交值逐键对表
    const sent = await apiCall<Row>(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${subscriptionId}/send`,
      {}
    );
    expect(str(sent.message), '立即发送的成功文案必须是后端固定常量').toBe('报表订阅已立即发送');
    const sentData = sent.data as Row;
    expect(sentData, `立即发送必须出业务载荷，实际 ${JSON.stringify(sent.data)}`).not.toBeNull();
    expect(num(sentData.subscription_id), '立即发送出参必须回显订阅主键').toBe(subscriptionId);
    expect(str(sentData.name), '立即发送出参名称必须等于建行提交名').toBe(`118订阅${tag}`);
    expect(sentData.recipients as string[], '立即发送出参收件人必须逐枚等于建行提交值').toEqual(
      recipients
    );
    expect(str(sentData.export_format), '立即发送出参导出格式必须等于建行提交值').toBe('xlsx');
    const afterSend = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions/${subscriptionId}`
    );
    expect(str(afterSend.name), '立即发送不许改动订阅名称').toBe(`118订阅${tag}`);
    expect(afterSend.recipients as string[], '立即发送不许改动收件人').toEqual(recipients);
    expect(num(afterSend.run_count), '立即发送同样不产执行结果，次数仍为 0').toBe(0);
    assertParseableTime(afterSend.next_run_at, '立即发送后的下次执行时间');

    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/subscriptions/${subscriptionId}`,
      `[118] T2 sub ${subscriptionId}`
    );
    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/templates/${templateId}`,
      `[118] T2 tpl ${templateId}`
    );
  });

  test('T3 订阅四道校验门与三枚动作的缺前置 全部零推进与零漂移', async ({ page }) => {
    const tag = genCode('T3');
    const createdTpl = await createTemplate(page, tag);
    const templateId = num(createdTpl.id);
    const sub = await createSubscription(page, templateId, tag);
    const subscriptionId = num(sub.id);

    // 一 收件人空集：validation_displayable，出参原文可钉
    const emptyRecipients = await apiCallExpectFail(
      page,
      'POST',
      '/reports/enhanced/subscriptions',
      {
        name: `118空收件人${tag}`,
        template_id: templateId,
        frequency: 'DAILY',
        recipients: [],
      }
    );
    expect(emptyRecipients.status, `空收件人必须 400，实际 ${emptyRecipients.status}`).toBe(400);
    expect(
      failureCode(emptyRecipients),
      `空收件人必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(emptyRecipients.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(str(emptyRecipients.message), '空收件人的出参文案必须是后端原文').toBe(
      '收件人列表不能为空'
    );

    // 二 邮箱形错：原文点名是哪一枚
    const badEmail = `rpt${tag}-bad@example`;
    const badRecipient = await apiCallExpectFail(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `118坏邮箱${tag}`,
      template_id: templateId,
      frequency: 'DAILY',
      recipients: [badEmail],
    });
    expect(badRecipient.status, `非法邮箱必须 400，实际 ${badRecipient.status}`).toBe(400);
    expect(
      failureCode(badRecipient),
      `非法邮箱必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(badRecipient.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(str(badRecipient.message), '非法邮箱的出参文案必须点名被拒的那一枚').toBe(
      `收件人邮箱格式无效: ${badEmail}`
    );

    // 三 频率越界：不在词表即拒，且不回落到默认档
    const badFrequency = await apiCallExpectFail(page, 'POST', '/reports/enhanced/subscriptions', {
      name: `118坏频率${tag}`,
      template_id: templateId,
      frequency: 'HOURLY',
      recipients: [`rpt${tag}@example.com`],
    });
    expect(badFrequency.status, `越界频率必须 400，实际 ${badFrequency.status}`).toBe(400);
    expect(
      failureCode(badFrequency),
      `越界频率必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(badFrequency.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    expect(str(badFrequency.message), '越界频率的出参文案必须是后端原文').toBe('无效的订阅频率');

    // 四 外键缺行：先查模板行，缺行即 404，比收件人校验更靠前
    const missingTemplate = await apiCallExpectFail(
      page,
      'POST',
      '/reports/enhanced/subscriptions',
      {
        name: `118坏外键${tag}`,
        template_id: 99999999,
        frequency: 'DAILY',
        recipients: [`rpt${tag}@example.com`],
      }
    );
    expect(
      missingTemplate.status,
      `缺模板行的订阅创建必须 404，实际 ${missingTemplate.status}`
    ).toBe(404);
    expect(failureCode(missingTemplate), '缺模板行的订阅创建必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    // 四次被拒一律零推进：按本 worker 专属外键过滤，仍只有建行的那一枚
    const list = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions?template_id=${templateId}&page=1&page_size=20`
    );
    const rows = list.items as Row[];
    expect(rows.length, `被拒的四次创建都不许建行，实际 ${JSON.stringify(rows)}`).toBe(1);
    expect(num(rows[0].id), '唯一行必须是自建的那一枚订阅').toBe(subscriptionId);

    // 五 启停必填缺失：ToggleSubscriptionDto 的 enabled 为必填布尔，缺键即解码拒
    const toggleNoBody = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${subscriptionId}/toggle`,
      {}
    );
    expect(toggleNoBody.status, `toggle 缺 enabled 必须 400，实际 ${toggleNoBody.status}`).toBe(
      400
    );
    expect(
      failureCode(toggleNoBody),
      `toggle 缺必填必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(toggleNoBody.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);
    const afterToggleReject = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions/${subscriptionId}`
    );
    expect(afterToggleReject.is_enabled, '被拒的 toggle 不许改动启用位').toBe(false);
    expect(num(afterToggleReject.run_count), '被拒的 toggle 不许改动执行次数').toBe(0);

    // 六 三枚动作对不存在主键一律前置 404，且回读列表证明没有半程数据
    const missingId = 99999999;
    const triggerMissing = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${missingId}/trigger`,
      {}
    );
    expect(
      triggerMissing.status,
      `不存在主键的手动触发必须 404，实际 ${triggerMissing.status}`
    ).toBe(404);
    expect(failureCode(triggerMissing), '不存在主键的手动触发必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const sendMissing = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${missingId}/send`,
      {}
    );
    expect(sendMissing.status, `不存在主键的立即发送必须 404，实际 ${sendMissing.status}`).toBe(
      404
    );
    expect(failureCode(sendMissing), '不存在主键的立即发送必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const toggleMissing = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/subscriptions/${missingId}/toggle`,
      { enabled: true }
    );
    expect(toggleMissing.status, `不存在主键的启停必须 404，实际 ${toggleMissing.status}`).toBe(
      404
    );
    expect(failureCode(toggleMissing), '不存在主键的启停必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const listAfterMissing = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/subscriptions?template_id=${missingId}&page=1&page_size=20`
    );
    expect((listAfterMissing.items as Row[]).length, '不存在主键的三次动作都不许留下订阅行').toBe(
      0
    );
    expect(num(listAfterMissing.total), '不存在主键的订阅列表 total 必须为 0').toBe(0);

    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/subscriptions/${subscriptionId}`,
      `[118] T3 sub ${subscriptionId}`
    );
    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/templates/${templateId}`,
      `[118] T3 tpl ${templateId}`
    );
  });
});
