import { test, expect, type Page } from '../diagnose-fixture';
import {
  APP_ERROR_CODES,
  apiCallExpectFail,
  apiCallRaw,
  expectDenied,
  failureCode,
  genCode,
  loginViaUI,
  tryCleanup,
} from './helpers';

/**
 * 117 报表增强模板生命周期真值链（字段字典 建模板 读模板 改模板 预览 版本列表 回滚 执行）
 *
 * 与既有覆盖的分工。既有只把报表模板域做到"列表能读、建单能删"的浅覆盖
 * （列表健康探针与删除矩阵两条），判定器口径下 GET fields 按模板类型解析、
 * GET 模板详情、PUT 改模板、GET preview、GET versions、POST rollback、
 * POST execute 七枚端点均为零覆盖。本 spec 把这七枚钉成真实落库回读链，
 * 并把必填门与安全门拒绝钉到机器码与零漂移两个口径。
 *
 * 契约真相（逐段读后端源，非推测）。
 * 路由装配在 backend/src/routes/analytics.rs 的 reports_enhanced()，
 * 以 nest 前缀 reports enhanced 挂进分析域，最终路径为
 * /api/v1/erp/reports/enhanced 下的各子路径。前端调用方是
 * frontend/src/api/report-enhanced.ts 与 frontend/src/api/report-templates.ts，
 * 消费页面是 frontend/src/views/report-templates/index.vue。
 * handler 在 backend/src/handlers/report_enhanced_handler.rs，
 * 业务实现与落库在 backend/src/services/report_template_service.rs。
 *
 * 字段字典 GET fields 入参是路径段 template_type，出参 data 两键
 * template_type 与 fields，每枚字段恰三键 field title data_type。
 * 词表按小写匹配且含中文别名，sales 与 sales_daily 与中文销售同档返回
 * 订单编号 客户名称 订单日期 订单金额 状态五枚；purchase 与 purchase_summary
 * 与中文采购返回采购单号 供应商 下单日期 采购金额 交期五枚；
 * inventory 与 inventory_status 与中文库存返回产品编码 产品名称 可用库存
 * 预留库存 仓库五枚；financial 与 finance 与中文财务返回付款单号 金额
 * 付款方式 状态 创建时间五枚；custom 与中文自定义返回 ID 名称 创建时间三枚；
 * 其余任何类型一律返回单枚通配字段，field 为星号且 title 为全部字段。
 * 该档字段元数据由服务层静态常量出参，不查库，故同一类型两次读取必须逐字节相同。
 *
 * 建模板 POST templates 入参 CreateReportTemplateRequest，必填 name code
 * report_type columns 四键（columns 是非 Option 的 JSON，缺键即被请求解码拒），
 * name 长度限 1 到 100、code 限 1 到 50，越界走参数校验后以脱敏 business 出参，
 * 即 HTTP 400 加 BUSINESS_ERROR。同码还有两处入口：code 撞库时以
 * "报表模板编码已存在"的同族 business 拒绝；提交 data_source_sql 任一模板
 * 一律 403（自定义 SQL 报表面被安全策略整口关闭，create 与 update 双门同判）。
 * 建行落 report_templates 表，服务端写死初值 status 为 ACTIVE、version 为 1、
 * sort_order 缺省补 asc，并按 report_type 自动派生 required_permission
 * （sales 档派生 report:sales:view），created_by 取会话用户。出参为整行 Model。
 *
 * 读模板 GET templates 对象端点按主键取行并做可见性判定：私有且非本人创建即 403，
 * 主键不存在即 404 加 NOT_FOUND。版本列表 GET versions 复用同一可见性判定，
 * 但对"主键不存在"只取返回值的错误通道而丢弃 None 分支，故对不存在的主键
 * 返回 200 加空数组，与详情端的 404 构成同一片实现里的两种真实差异，
 * 本 spec 如实钉出当前行为，不据它改判后端。
 *
 * 改模板 PUT templates 门为"只有创建者可改"，改前把当前行整行写入
 * report_template_versions 表作为版本号为旧值的历史快照，再让主表 version 自增 1。
 * 快照存 name code report_type category data_source columns filters parameters
 * supported_formats sort_by sort_order data_source_sql description is_public
 * required_permission 与快照人 snapshot_by 与快照时间 snapshot_at，
 * 因此历史版本列表可按版本号倒序读回改动前的真值。
 *
 * 回滚 POST rollback 入参是模板主键与目标版本号两段路径，门为"仅创建者可回滚"，
 * 目标版本号在该模板的历史快照里不存在即 404。回滚先把当前行以当前版本号再存一份
 * 快照（保证回滚本身可被再次回滚），取快照表最大版本号与当前版本号的较大者加一
 * 作为新版本号，再用目标快照的业务字段整行覆盖主表。所以一次"建 v1 改 v2 回滚 v1"
 * 的确定结果是主表 version 由 2 变 3、业务列回到 v1 值、版本表出现 1 与 2 两行。
 *
 * 预览 GET preview 与执行 POST execute 都先按主键做可见性读取再走取数口
 * execute_custom_report。该取数口在当前实现里对任何模板都只可能返回错误：
 * 带 data_source_sql 的行 403，不带的行 400 加 BUSINESS_ERROR 且出参文案脱敏，
 * 而经 API 建出的模板永远不带 data_source_sql（create 与 update 双门拒绝），
 * 所以这两枚端点对自建模板的可达契约就是"400 加 BUSINESS_ERROR"。
 * 本 spec 钉死这条可达契约并同时钉死主键不存在时前置的 404，
 * 不去伪造一个取不到数据的正向断言。
 *
 * 假绿防线。每条写操作都紧跟一次对象读回逐列对表，版本表回读的是改动前的旧值
 * 而不是新值，回滚断的是主表回到旧值且版本号继续前进；必填缺失 与 判重 与
 * 安全门三类拒绝一律钉 HTTP 与机器码，并在拒绝后回读列表证明库里零推进。
 * 全程不用 ?? 或 || 兜底，后端必出的键缺失即形漂判红；造数失败当场红并点名步骤。
 */

type Row = Record<string, unknown>;

interface FieldDef {
  field: string;
  title: string;
  data_type: string;
}

const str = (v: unknown): string => String(v);
const num = (v: unknown): number => Number(String(v));

/** 字段字典各档的期望全形（与后端静态常量逐键逐序对齐，不是抽样） */
const SALES_FIELDS: FieldDef[] = [
  { field: 'order_no', title: '订单编号', data_type: 'string' },
  { field: 'customer_name', title: '客户名称', data_type: 'string' },
  { field: 'order_date', title: '订单日期', data_type: 'date' },
  { field: 'total_amount', title: '订单金额', data_type: 'decimal' },
  { field: 'status', title: '状态', data_type: 'string' },
];
const PURCHASE_FIELDS: FieldDef[] = [
  { field: 'order_no', title: '采购单号', data_type: 'string' },
  { field: 'supplier_name', title: '供应商', data_type: 'string' },
  { field: 'order_date', title: '下单日期', data_type: 'date' },
  { field: 'total_amount', title: '采购金额', data_type: 'decimal' },
  { field: 'delivery_date', title: '交期', data_type: 'date' },
];
const INVENTORY_FIELDS: FieldDef[] = [
  { field: 'product_code', title: '产品编码', data_type: 'string' },
  { field: 'product_name', title: '产品名称', data_type: 'string' },
  { field: 'quantity_available', title: '可用库存', data_type: 'decimal' },
  { field: 'quantity_reserved', title: '预留库存', data_type: 'decimal' },
  { field: 'warehouse', title: '仓库', data_type: 'string' },
];
const FINANCE_FIELDS: FieldDef[] = [
  { field: 'payment_no', title: '付款单号', data_type: 'string' },
  { field: 'amount', title: '金额', data_type: 'decimal' },
  { field: 'payment_method', title: '付款方式', data_type: 'string' },
  { field: 'status', title: '状态', data_type: 'string' },
  { field: 'created_at', title: '创建时间', data_type: 'datetime' },
];
const CUSTOM_FIELDS: FieldDef[] = [
  { field: 'id', title: 'ID', data_type: 'string' },
  { field: 'name', title: '名称', data_type: 'string' },
  { field: 'created_at', title: '创建时间', data_type: 'datetime' },
];

/** 本 worker 专属模板建行，返回整行。造数失败当场红并点名是建模板这一步 */
async function createTemplate(page: Page, tag: string, overrides: Row = {}): Promise<Row> {
  const created = await apiCallRaw<Row>(page, 'POST', '/reports/enhanced/templates', {
    name: `117模板${tag}`,
    code: genCode(`RT117${tag}`),
    report_type: 'sales',
    columns: [{ field: 'order_no', title: '订单编号' }],
    description: `117建模板${tag}`,
    is_public: false,
    ...overrides,
  });
  const templateId = num(created.id);
  expect(
    Number.isInteger(templateId) && templateId > 0,
    `117 建模板造数失败，实际 ${JSON.stringify(created)}`
  ).toBe(true);
  return created;
}

test.describe.serial('117 报表增强模板生命周期真值链', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('T1 字段字典逐档全形解析 中文别名同档 未知档回落通配 两次读取逐字节相同', async ({
    page,
  }) => {
    const cases: Array<{ type: string; expectFields: FieldDef[] }> = [
      { type: 'sales', expectFields: SALES_FIELDS },
      { type: 'sales_daily', expectFields: SALES_FIELDS },
      { type: '销售', expectFields: SALES_FIELDS },
      { type: 'purchase', expectFields: PURCHASE_FIELDS },
      { type: 'purchase_summary', expectFields: PURCHASE_FIELDS },
      { type: '采购', expectFields: PURCHASE_FIELDS },
      { type: 'inventory', expectFields: INVENTORY_FIELDS },
      { type: 'inventory_status', expectFields: INVENTORY_FIELDS },
      { type: '库存', expectFields: INVENTORY_FIELDS },
      { type: 'financial', expectFields: FINANCE_FIELDS },
      { type: 'finance', expectFields: FINANCE_FIELDS },
      { type: '财务', expectFields: FINANCE_FIELDS },
      { type: 'custom', expectFields: CUSTOM_FIELDS },
      { type: '自定义', expectFields: CUSTOM_FIELDS },
    ];

    for (const c of cases) {
      const res = await apiCallRaw<Row>(
        page,
        'GET',
        `/reports/enhanced/fields/${encodeURIComponent(c.type)}`
      );
      expect(
        str(res.template_type),
        `字段字典出参 template_type 必须回显入参 ${c.type}，实际 ${JSON.stringify(res)}`
      ).toBe(c.type);
      expect(
        Array.isArray(res.fields),
        `字段字典 ${c.type} 档必须出数组，实际 ${JSON.stringify(res.fields)}`
      ).toBe(true);
      expect(
        res.fields as FieldDef[],
        `字段字典 ${c.type} 档的字段全形必须逐键逐序等于后端常量`
      ).toEqual(c.expectFields);
    }

    // 未知档：不报 404，而是单枚通配字段（后端 catch-all 分支的真实契约）
    const wild = await apiCallRaw<Row>(page, 'GET', '/reports/enhanced/fields/unknown_type_117');
    expect(wild.fields as FieldDef[], '未知档必须只出一枚通配字段').toEqual([
      { field: '*', title: '全部字段', data_type: 'string' },
    ]);

    // 静态常量档：同档两次读取逐字节相同，锁的是该端点不查库、不受并发影响的确定性。
    // 比对基准取首读原文，不取用例侧常量声明序——后端出参经 serde_json Value 构键，
    // 对象键恒按字母序序列化，与 TS 常量的书写序不同源，拿书写序做逐字节比必假红。
    const firstSales = await apiCallRaw<Row>(page, 'GET', '/reports/enhanced/fields/sales');
    const again = await apiCallRaw<Row>(page, 'GET', '/reports/enhanced/fields/sales');
    expect(JSON.stringify(again.fields), '字段字典同档二次读取必须与首次逐字节相同').toBe(
      JSON.stringify(firstSales.fields)
    );
  });

  test('T2 建模板服务端初值逐列回显 详情逐列互证 主键不存在 404', async ({ page }) => {
    const tag = genCode('T2');
    const created = await createTemplate(page, tag, {
      sort_by: 'order_date',
      sort_order: 'desc',
      refresh_strategy: 'DAILY',
      cache_ttl_seconds: 3600,
      supported_formats: ['pdf', 'xlsx'],
    });
    const templateId = num(created.id);
    expect(
      Number.isInteger(templateId) && templateId > 0,
      `建模板必须回主键，实际 ${JSON.stringify(created)}`
    ).toBe(true);

    // 服务端写死的初值：状态词表 ACTIVE、版本 1、派生权限码 report:sales:view
    expect(str(created.status), '建行 status 必须为 ACTIVE').toBe('ACTIVE');
    expect(num(created.version), '建行 version 必须为 1').toBe(1);
    expect(str(created.required_permission), 'sales 档必须自动派生权限码').toBe(
      'report:sales:view'
    );
    expect(str(created.sort_order), '提交 desc 时 sort_order 必须回显提交值').toBe('desc');
    expect(created.is_public, '私有位必须回显 false').toBe(false);
    expect(created.data_source_sql, '建行不得带数据源SQL').toBeNull();
    expect(num(created.cache_ttl_seconds), '缓存 TTL 必须回显提交值').toBe(3600);
    expect(str(created.refresh_strategy), '刷新策略必须回显提交值').toBe('DAILY');

    // 详情读口逐列互证（写响应与独立读口同源，防"响应回显但没落库"）
    const detail = await apiCallRaw<Row>(page, 'GET', `/reports/enhanced/templates/${templateId}`);
    expect(num(detail.id), '详情主键必须等于建行主键').toBe(templateId);
    expect(str(detail.name), '详情 name 必须等于提交名').toBe(`117模板${tag}`);
    expect(str(detail.code), '详情 code 必须等于建行回显 code').toBe(str(created.code));
    expect(str(detail.report_type), '详情 report_type 必须等于 sales').toBe('sales');
    expect(str(detail.description), '详情 description 必须等于提交备注').toBe(`117建模板${tag}`);
    expect(num(detail.version), '详情 version 必须为 1').toBe(1);
    expect(str(detail.status), '详情 status 必须为 ACTIVE').toBe('ACTIVE');
    expect(str(detail.required_permission), '详情权限码必须与建行派生值一致').toBe(
      str(created.required_permission)
    );
    expect(JSON.stringify(detail.columns), '详情 columns 必须原样回读提交的列定义').toBe(
      JSON.stringify([{ field: 'order_no', title: '订单编号' }])
    );
    expect(
      Array.isArray(detail.supported_formats),
      `supported_formats 必须回数组，实际 ${JSON.stringify(detail.supported_formats)}`
    ).toBe(true);
    expect(
      (detail.supported_formats as string[]).slice().sort().join(','),
      'supported_formats 必须回读提交的两枚格式'
    ).toBe('pdf,xlsx');

    // 主键不存在：详情 404 + NOT_FOUND 机器码
    const missing = await apiCallExpectFail(page, 'GET', '/reports/enhanced/templates/99999999');
    expect(missing.status, `不存在的主键必须 404，实际 ${missing.status}`).toBe(404);
    expect(
      failureCode(missing),
      `不存在的主键必须归因 NOT_FOUND，实际 ${JSON.stringify(missing.code)}`
    ).toBe(APP_ERROR_CODES.NOT_FOUND);

    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/templates/${templateId}`,
      `[117] T2 ${templateId}`
    );
  });

  test('T3 改模板写历史快照 版本列表读回旧值 回滚回到旧值且版本前进 目标版本不存在 404', async ({
    page,
  }) => {
    const tag = genCode('T3');
    const created = await createTemplate(page, tag);
    const templateId = num(created.id);
    const code = str(created.code);
    const v1Name = str(created.name);
    const v1Desc = str(created.description);
    const v1Columns = [{ field: 'order_no', title: '订单编号' }];

    // 改模板：主表版本自增，旧值进版本表
    const v2Name = `117改名${tag}`;
    const v2Columns = [
      { field: 'order_no', title: '订单编号' },
      { field: 'total_amount', title: '订单金额' },
    ];
    const updated = await apiCallRaw<Row>(
      page,
      'PUT',
      `/reports/enhanced/templates/${templateId}`,
      {
        name: v2Name,
        columns: v2Columns,
        description: `117改后${tag}`,
      }
    );
    expect(num(updated.version), '改后 version 必须自增到 2').toBe(2);
    expect(str(updated.name), '改后回显 name 必须等于提交新名').toBe(v2Name);
    const readBack = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}`
    );
    expect(num(readBack.version), '详情回读 version 必须为 2').toBe(2);
    expect(str(readBack.name), '详情回读 name 必须已是新名').toBe(v2Name);

    // 版本列表：读回的是改动前的旧值，且按版本号倒序
    const versions = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}/versions`
    );
    expect(num(versions.template_id), '版本列表必须回显所查模板主键').toBe(templateId);
    expect(Array.isArray(versions.versions), '版本列表必须出数组').toBe(true);
    const list = versions.versions as Row[];
    expect(list.length, '一次改动应恰留一行版本 1 快照').toBe(1);
    expect(num(versions.total), '版本列表 total 必须等于行数').toBe(1);
    expect(num(list[0].version), '首行快照版本号必须为 1').toBe(1);
    expect(str(list[0].name), '快照 name 必须是改动前的旧名而不是新名').toBe(v1Name);
    expect(str(list[0].description), '快照 description 必须是改动前的旧备注').toBe(v1Desc);
    expect(str(list[0].code), '快照 code 必须等于模板编码').toBe(code);
    expect(JSON.stringify(list[0].columns), '快照 columns 必须原样存改动前的列定义').toBe(
      JSON.stringify(v1Columns)
    );
    expect(
      Number.isInteger(num(list[0].snapshot_by)) && num(list[0].snapshot_by) > 0,
      `快照人必须是会话用户主键，实际 ${JSON.stringify(list[0].snapshot_by)}`
    ).toBe(true);

    // 回滚到版本 1：主表业务列回到旧值，版本号按快照表最大值继续前进
    const rolled = await apiCallRaw<Row>(
      page,
      'POST',
      `/reports/enhanced/templates/${templateId}/rollback/1`,
      {}
    );
    expect(str(rolled.name), '回滚后 name 必须回到版本 1 的旧名').toBe(v1Name);
    expect(str(rolled.description), '回滚后备注必须回到版本 1 的旧值').toBe(v1Desc);
    expect(num(rolled.version), '回滚后 version 必须前进到 3 而不是退回 1').toBe(3);
    expect(JSON.stringify(rolled.columns), '回滚后 columns 必须回到版本 1 的旧列定义').toBe(
      JSON.stringify(v1Columns)
    );

    const afterRollback = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}`
    );
    expect(str(afterRollback.name), '回滚必须真落库（详情回读到旧名）').toBe(v1Name);
    expect(num(afterRollback.version), '详情回读 version 必须为 3').toBe(3);

    // 回滚自身也要留快照：版本表出现 1 与 2 两行，倒序首行是回滚前的 v2 真值
    const versionsAfter = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}/versions`
    );
    const listAfter = versionsAfter.versions as Row[];
    expect(listAfter.length, '回滚后版本表必须有两行快照').toBe(2);
    expect(num(listAfter[0].version), '倒序首行必须是回滚前保存的版本 2 快照').toBe(2);
    expect(str(listAfter[0].name), '版本 2 快照必须存回滚前的新名').toBe(v2Name);
    expect(num(listAfter[1].version), '次行必须是版本 1 快照').toBe(1);

    // 回滚到从未快照过的版本号：404，且主表零漂移
    const badRollback = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/templates/${templateId}/rollback/77`,
      {}
    );
    expect(badRollback.status, `回滚到不存在的版本号必须 404，实际 ${badRollback.status}`).toBe(
      404
    );
    expect(
      failureCode(badRollback),
      `回滚不存在的版本号必须归因 NOT_FOUND，实际 ${JSON.stringify(badRollback.code)}`
    ).toBe(APP_ERROR_CODES.NOT_FOUND);
    const afterBad = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}`
    );
    expect(num(afterBad.version), '被拒的回滚不许推进版本号').toBe(3);
    expect(str(afterBad.name), '被拒的回滚不许改动名称').toBe(v1Name);
    const versionsFinal = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}/versions`
    );
    expect((versionsFinal.versions as Row[]).length, '被拒的回滚不许新增历史快照行').toBe(2);

    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/templates/${templateId}`,
      `[117] T3 ${templateId}`
    );
  });

  test('T4 预览与执行端点的可达契约是取数门拒绝 主键不存在先行 404 拒绝零副作用', async ({
    page,
  }) => {
    const tag = genCode('T4');
    const created = await createTemplate(page, tag);
    const templateId = num(created.id);

    // 预览与执行都走同一取数口：对经 API 建出的模板恒为 400 加 BUSINESS_ERROR
    const preview = await apiCallExpectFail(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}/preview`
    );
    expect(
      preview.status,
      `自建模板的预览必须被取数门拒为 400，实际 ${preview.status} code=${JSON.stringify(preview.code)}`
    ).toBe(400);
    expect(
      failureCode(preview),
      `预览拒绝必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(preview.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    const exec = await apiCallExpectFail(
      page,
      'POST',
      `/reports/enhanced/templates/${templateId}/execute`,
      {}
    );
    expect(
      exec.status,
      `自建模板的执行必须被取数门拒为 400，实际 ${exec.status} code=${JSON.stringify(exec.code)}`
    ).toBe(400);
    expect(
      failureCode(exec),
      `执行拒绝必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(exec.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 拒绝零副作用：预览与执行都不写主表、不写版本表
    const afterExec = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}`
    );
    expect(num(afterExec.version), '被拒的执行不许推进版本号').toBe(1);
    expect(str(afterExec.name), '被拒的执行不许改动名称').toBe(str(created.name));
    expect(str(afterExec.status), '被拒的执行不许改动状态位').toBe('ACTIVE');
    const versionsAfterExec = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}/versions`
    );
    expect((versionsAfterExec.versions as Row[]).length, '被拒的预览与执行不许留版本快照').toBe(0);

    // 主键不存在时，两枚端点都在取数之前先 404
    const previewMissing = await apiCallExpectFail(
      page,
      'GET',
      '/reports/enhanced/templates/99999999/preview'
    );
    expect(previewMissing.status, `不存在主键的预览必须 404，实际 ${previewMissing.status}`).toBe(
      404
    );
    expect(failureCode(previewMissing), '不存在主键的预览必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );
    const execMissing = await apiCallExpectFail(
      page,
      'POST',
      '/reports/enhanced/templates/99999999/execute',
      {}
    );
    expect(execMissing.status, `不存在主键的执行必须 404，实际 ${execMissing.status}`).toBe(404);
    expect(failureCode(execMissing), '不存在主键的执行必须归因 NOT_FOUND').toBe(
      APP_ERROR_CODES.NOT_FOUND
    );

    // 版本列表对不存在主键的真实差异：前置可见性只吃错误通道，None 分支放行成空列表
    const versionsMissing = await apiCallRaw<Row>(
      page,
      'GET',
      '/reports/enhanced/templates/99999999/versions'
    );
    expect((versionsMissing.versions as Row[]).length, '不存在主键的版本列表当前实现出空数组').toBe(
      0
    );
    expect(num(versionsMissing.total), '空版本列表 total 必须为 0').toBe(0);

    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/templates/${templateId}`,
      `[117] T4 ${templateId}`
    );
  });

  test('T5 必填缺失撞解码门 名称越界撞校验门 编码判重 数据源SQL安全门 四类拒绝全部零推进', async ({
    page,
  }) => {
    const tag = genCode('T5');
    const code = genCode(`RT117${tag}`);

    // 一 必填缺失：columns 是非 Option JSON，缺键由请求解码拒（不进业务层）
    const missingRequired = await apiCallExpectFail(page, 'POST', '/reports/enhanced/templates', {
      name: `117缺列${tag}`,
      code: `${code}-A`,
      report_type: 'sales',
    });
    expect(
      missingRequired.status,
      `缺 columns 必须被解码门拒为 400，实际 ${missingRequired.status}`
    ).toBe(400);
    expect(
      failureCode(missingRequired),
      `缺必填键必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(missingRequired.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 二 空请求体：同样撞解码门
    const emptyBody = await apiCallExpectFail(page, 'POST', '/reports/enhanced/templates', {});
    expect(emptyBody.status, '空体建模板必须 400').toBe(400);
    expect(
      failureCode(emptyBody),
      `空体必须归因 VALIDATION_ERROR，实际 ${JSON.stringify(emptyBody.code)}`
    ).toBe(APP_ERROR_CODES.VALIDATION_ERROR);

    // 三 名称越界：走参数校验后以脱敏 business 出参（与解码门同 HTTP 不同机器码）
    const tooLong = await apiCallExpectFail(page, 'POST', '/reports/enhanced/templates', {
      name: `117超长名${tag}`.padEnd(101, '长'),
      code: `${code}-B`,
      report_type: 'sales',
      columns: [],
    });
    expect(tooLong.status, `名称超 100 字符必须 400，实际 ${tooLong.status}`).toBe(400);
    expect(
      failureCode(tooLong),
      `名称越界走参数校验后由 business 出参，实际 ${JSON.stringify(tooLong.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 四 编码判重：先建一行占位，再同码重复提交必须被 business 拒
    const created = await createTemplate(page, tag, { code });
    const templateId = num(created.id);
    const dup = await apiCallExpectFail(page, 'POST', '/reports/enhanced/templates', {
      name: `117重复码${tag}`,
      code,
      report_type: 'purchase',
      columns: [],
    });
    expect(dup.status, `重复编码必须 400，实际 ${dup.status}`).toBe(400);
    expect(
      failureCode(dup),
      `重复编码必须归因 BUSINESS_ERROR，实际 ${JSON.stringify(dup.code)}`
    ).toBe(APP_ERROR_CODES.BUSINESS_ERROR);

    // 五 安全门：提交 data_source_sql 一律 403（自定义 SQL 报表面整口关闭）
    const sqlBody = await apiCallExpectFail(page, 'POST', '/reports/enhanced/templates', {
      name: `117带SQL${tag}`,
      code: `${code}-C`,
      report_type: 'custom',
      columns: [],
      data_source_sql: 'SELECT 1',
    });
    expectDenied(sqlBody, '自定义 SQL 报表必须被安全门 403 拒绝');

    // 六 改模板同样吃安全门，且被拒后主表零漂移
    const updateSql = await apiCallExpectFail(
      page,
      'PUT',
      `/reports/enhanced/templates/${templateId}`,
      { data_source_sql: 'SELECT 2' }
    );
    expectDenied(updateSql, '改模板提交 SQL 必须被安全门 403 拒绝');
    const afterSqlUpdate = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates/${templateId}`
    );
    expect(num(afterSqlUpdate.version), '被拒的改模板不许推进版本号').toBe(1);
    expect(String(afterSqlUpdate.data_source_sql), '被拒的改模板不许写入数据源SQL').toBe('null');

    // 零推进总证：按本 worker 专属编码前缀查列表，只允许存在占位那一行
    const list = await apiCallRaw<Row>(
      page,
      'GET',
      `/reports/enhanced/templates?keyword=${encodeURIComponent(code)}&page=1&page_size=20`
    );
    const items = list.items as Row[];
    expect(Array.isArray(items), '模板列表必须出 items 数组').toBe(true);
    expect(items.length, `被拒的五次提交都不许建行，实际 ${JSON.stringify(items)}`).toBe(1);
    expect(str(items[0].code), '列表唯一行必须是占位模板自己的编码').toBe(code);
    expect(num(list.total), '列表 total 必须与唯一占位行一致').toBe(1);

    await tryCleanup(
      page,
      'DELETE',
      `/reports/enhanced/templates/${templateId}`,
      `[117] T5 ${templateId}`
    );
  });
});
