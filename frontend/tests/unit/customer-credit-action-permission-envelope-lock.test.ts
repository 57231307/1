/**
 * 客户信用列表页的动作权限门与列表信封静态锁。
 * 功能，钉三件事。一，该页每个会打后端端点的动作按钮都必须带 v-permission，
 * 所用键必须是权限常量表已声明的 customer-credits 族键，且键的动作段取值
 * 由后端权限中间件的键派生规则在本锁内现场算出，不写死字面量，
 * 反向钉为出现没有权限指令的动作按钮或未登记的按钮动作入口即判红，
 * 并钉每个入口声明调用的 api 函数确在其请求载体里，防钉死清单与页面代码脱钩。
 * 二，列表函数的返回声明必须是 ApiResponse 包 PaginatedResponse 的 items total
 * page page_size 信封形态，不得再声明裸数组，列表页按 items 键取数，
 * 且不得用空数组兜底把契约漂移伪装成无数据。
 * 三，出参三个金额键的声明必须与后端 rust_decimal 的线形态同词，
 * 未启浮点化特性即 string，不得写成 number 也不得写成两者联合，
 * 消费侧的归一只允许发生在真正要算的地方。
 * 调用方，vitest，tests unit 目录，CI 的 vitest job。
 * 入参，仅 fs 读取仓库内源码文本，不执行后端代码，不发网络请求。
 * 传给谁，纯断言，无下游。存什么，不写库。存哪里，无。
 * 检测力自证，见末尾夹具段，把无权限指令的裸按钮与裸数组声明两种缺陷形态
 * 喂给同一判据，断言判据确实抓到，防判据静默失效。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';
import { PERMISSIONS } from '@/constants/permissions';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');
const REPO_ROOT = path.resolve(FRONTEND_ROOT, '..');

/** 权限常量表按名取值时的只读视图 */
const PERMISSION_KEYS: Readonly<Record<string, string>> = PERMISSIONS;

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出在 core.autocrlf 为真时是 CRLF、
 * CI 检出是 LF，本锁的模板切分与夹具替换对行尾形态敏感，不归一会让
 * 检测力自证被误判成判据失效。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

function feSrc(rel: string): string {
  return readRepoSource(path.join(FRONTEND_ROOT, rel));
}
function beSrc(rel: string): string {
  return readRepoSource(path.join(REPO_ROOT, 'backend', 'src', rel));
}

const PAGE_INDEX_SRC = feSrc('src/views/customer-credit/index.vue');
const RATING_TAB_SRC = feSrc('src/views/customer-credit/tabs/RatingDialogTab.vue');
const ADJUST_TAB_SRC = feSrc('src/views/customer-credit/tabs/AdjustDialogTab.vue');
const AMOUNT_TAB_SRC = feSrc('src/views/customer-credit/tabs/AmountDialogTab.vue');
const CREDIT_API_SRC = feSrc('src/api/customer-credit.ts');
const TYPES_API_SRC = feSrc('src/types/api-response.ts');
const CREDIT_HANDLER_RS = beSrc('handlers/customer_credit_handler.rs');
const RESPONSE_RS = beSrc('utils/response.rs');
const INIT_SERVICE_RS = beSrc('services/init_service.rs');
const PERMISSION_MIDDLEWARE_RS = beSrc('middleware/permission.rs');
const CREDIT_MODEL_RS = readRepoSource(
  path.join(REPO_ROOT, 'backend', 'src', 'models', 'customer_credit.rs')
);
const CARGO_TOML = readRepoSource(path.join(REPO_ROOT, 'backend', 'Cargo.toml'));

/** 截取 Rust 常量数组字面量体内的字符串项，与既有权限常量注册表锁同判据 */
function extractRustStringArray(source: string, constName: string): string[] {
  const start = source.indexOf(constName);
  if (start === -1) {
    throw new Error(`后端源码中找不到常量 ${constName}，后端改名必须同步本锁`);
  }
  const end = source.indexOf('];', start);
  if (end === -1) {
    throw new Error(`常量 ${constName} 的数组字面量未闭合`);
  }
  return [...source.slice(start, end).matchAll(/"([a-z0-9_-]+)"/g)].map(match => match[1]);
}

/**
 * 键派生要素全部现场取自后端源码，本锁不另写一份动作字面量。
 * 资源侧取 init_service 的 PERMISSION_RESOURCES 注册表，
 * 动作侧取权限中间件的方法派生常量与路径末段关键字表和查询动作白名单。
 */
const registry = new Set(extractRustStringArray(INIT_SERVICE_RS, 'PERMISSION_RESOURCES'));
const pathActionKeywords = new Set(
  extractRustStringArray(PERMISSION_MIDDLEWARE_RS, 'PATH_ACTION_KEYWORDS')
);
const queryActionKeywords = new Set(
  extractRustStringArray(PERMISSION_MIDDLEWARE_RS, 'QUERY_ACTION_KEYWORDS')
);

/** 从后端 method_to_action 的 match 分支解析 HTTP 方法到派生动作的映射 */
function parseMethodActionMap(src: string): Map<string, string> {
  const actionConsts = new Map<string, string>();
  for (const match of src.matchAll(/pub\(crate\) const ACTION_([A-Z]+): &str = "([a-z_]+)"/g)) {
    actionConsts.set(match[1], match[2]);
  }
  const map = new Map<string, string>();
  for (const match of src.matchAll(/Method::([A-Z]+)\s*=>\s*ACTION_([A-Z]+)/g)) {
    const action = actionConsts.get(match[2]);
    if (action) map.set(match[1], action);
  }
  if (map.size < 4) {
    throw new Error(`后端方法派生动作只解析到 ${map.size} 条，少于 4 条，判据已失效`);
  }
  return map;
}

const methodActionMap = parseMethodActionMap(PERMISSION_MIDDLEWARE_RS);
const assignableActions = new Set<string>([
  ...methodActionMap.values(),
  ...pathActionKeywords,
  ...queryActionKeywords,
]);

/**
 * 按后端 extract_route_info 的同序判据派生一条权限键。
 * 入参是 HTTP 方法与请求路径字面量，资源段取路径里第一个在注册表在册的段，
 * 动作段优先取路径末段关键字，否则按方法派生；两处认不出都显式抛错，不静默放行。
 */
function derivePermissionKey(method: string, urlPath: string): string {
  const segments = urlPath.split('/').filter(segment => segment !== '');
  const resource = segments.find(segment => registry.has(segment));
  if (!resource) {
    throw new Error(`路径 ${urlPath} 的段里找不到在册资源，注册表漂移或本锁解析失效`);
  }
  const lastSegment = segments[segments.length - 1] ?? '';
  const action = pathActionKeywords.has(lastSegment)
    ? lastSegment
    : methodActionMap.get(method.toUpperCase());
  if (!action) {
    throw new Error(`方法 ${method} 没有派生动作，后端 method_to_action 已漂移`);
  }
  return `${resource}:${action}`;
}

/** 从 api 源文件里逐个导出函数解析其请求方法小写名与路径字面量 */
function extractApiEndpoints(apiSrc: string): Map<string, { method: string; path: string }> {
  const endpoints = new Map<string, { method: string; path: string }>();
  for (const chunk of apiSrc.split(/\nexport const /).slice(1)) {
    const name = chunk.match(/^([A-Za-z0-9_]+)/)?.[1];
    const call = chunk.match(/request\.(get|post|put|delete)\(\s*([`'])((?:\\.|(?!\2)[^\\])*)\2/);
    if (name && call) {
      endpoints.set(name, { method: call[1].toUpperCase(), path: call[3] });
    }
  }
  if (endpoints.size < 10) {
    throw new Error(
      `api 源文件只解析到 ${endpoints.size} 条端点，少于 10 条，解析判据已退化，本锁不可信`
    );
  }
  return endpoints;
}

const creditEndpoints = extractApiEndpoints(CREDIT_API_SRC);

/** 被钉的一个动作按钮及其门禁来源 */
interface PinnedButton {
  /** 所属文件标签 */
  file: string;
  /** 按钮 @click 绑定的处理函数名 */
  handler: string;
  /**
   * 真正发起请求的载体，同文件的另一个处理函数名或 tabs 下的组件文件名。
   * 入口只负责打开对话框时用它点名调用发生在哪，缺省即处理函数自身。
   */
  via?: string;
  /** 处理函数实际调用的 api 函数与期望权限常量名的配对 */
  grants: Array<{ apiFn: string; perm: string }>;
}

const PINNED_BUTTONS: readonly PinnedButton[] = [
  {
    file: 'index.vue',
    handler: 'openRatingDialog',
    via: 'RatingDialogTab.vue',
    grants: [{ apiFn: 'setCreditRating', perm: 'CUSTOMER_CREDIT_CREATE' }],
  },
  {
    file: 'index.vue',
    handler: 'handleViewDetail',
    grants: [{ apiFn: 'getCustomerCredit', perm: 'CUSTOMER_CREDIT_READ' }],
  },
  {
    file: 'index.vue',
    handler: 'handleEdit',
    via: 'submitEdit',
    grants: [{ apiFn: 'updateCustomerCredit', perm: 'CUSTOMER_CREDIT_UPDATE' }],
  },
  {
    file: 'index.vue',
    handler: 'submitEdit',
    grants: [{ apiFn: 'updateCustomerCredit', perm: 'CUSTOMER_CREDIT_UPDATE' }],
  },
  {
    file: 'index.vue',
    handler: 'openAdjustDialog',
    via: 'AdjustDialogTab.vue',
    grants: [{ apiFn: 'adjustCreditLimit', perm: 'CUSTOMER_CREDIT_CREATE' }],
  },
  {
    file: 'index.vue',
    handler: 'openOccupyDialog',
    via: 'AmountDialogTab.vue',
    grants: [{ apiFn: 'occupyCredit', perm: 'CUSTOMER_CREDIT_CREATE' }],
  },
  {
    file: 'index.vue',
    handler: 'openReleaseDialog',
    via: 'AmountDialogTab.vue',
    grants: [{ apiFn: 'releaseCredit', perm: 'CUSTOMER_CREDIT_RELEASE' }],
  },
  {
    file: 'index.vue',
    handler: 'handleEvaluate',
    grants: [{ apiFn: 'evaluateCustomerCredit', perm: 'CUSTOMER_CREDIT_CREATE' }],
  },
  {
    file: 'index.vue',
    handler: 'handleDeactivate',
    grants: [{ apiFn: 'deactivateCredit', perm: 'CUSTOMER_CREDIT_CREATE' }],
  },
  {
    file: 'index.vue',
    handler: 'handleDelete',
    grants: [{ apiFn: 'deleteCustomerCredit', perm: 'CUSTOMER_CREDIT_DELETE' }],
  },
  {
    file: 'RatingDialogTab.vue',
    handler: 'handleSubmit',
    grants: [{ apiFn: 'setCreditRating', perm: 'CUSTOMER_CREDIT_CREATE' }],
  },
  {
    file: 'AdjustDialogTab.vue',
    handler: 'handleSubmit',
    grants: [{ apiFn: 'adjustCreditLimit', perm: 'CUSTOMER_CREDIT_CREATE' }],
  },
  {
    file: 'AmountDialogTab.vue',
    // 该对话框按 operationType 复用于占用与释放，指令只在挂载时求值，
    // 取两枚动作键的并集做二次校验，单动作的精确门禁在列表页的占用与释放入口。
    handler: 'handleSubmit',
    grants: [
      { apiFn: 'occupyCredit', perm: 'CUSTOMER_CREDIT_CREATE' },
      { apiFn: 'releaseCredit', perm: 'CUSTOMER_CREDIT_RELEASE' },
    ],
  },
];

interface ButtonTag {
  /** @click 原文，用于识别只改可见性的关窗形态 */
  click: string;
  /** @click 绑定的处理函数名，非函数调用形态时为空串 */
  handler: string;
  /** v-permission 表达式原文，缺指令时为空串 */
  permission: string;
}

/** 切出一个 vue 模板段里的全部 el-button 开标签及其 @click 与 v-permission */
function extractButtons(source: string): ButtonTag[] {
  const scriptAt = source.indexOf('<script');
  const template = scriptAt === -1 ? source : source.slice(0, scriptAt);
  const buttons: ButtonTag[] = [];
  for (const match of template.matchAll(/<el-button\b[\s\S]*?>/g)) {
    const tag = match[0];
    const click = tag.match(/@click="([^"]*)"/)?.[1] ?? '';
    const permission = tag.match(/v-permission="([^"]*)"/)?.[1] ?? '';
    const handler = click.match(/^([A-Za-z_$][\w$]*)\s*(?:\(|$)/)?.[1] ?? '';
    buttons.push({ click, handler, permission });
  }
  if (buttons.length === 0) {
    throw new Error('模板里一个 el-button 都没切到，切分判据已失效，本锁不可信');
  }
  return buttons;
}

/** v-permission 表达式与钉死期望的比对，多枚键时按声明顺序写成数组形态 */
function expectedPermissionExpr(grants: readonly { perm: string }[]): string {
  const items = grants.map(grant => `PERMISSIONS.${grant.perm}`);
  return items.length === 1 ? items[0] : `[${items.join(', ')}]`;
}

/**
 * 动作按钮权限门的判据主体，输入一段源文本与所属文件标签，输出违规清单。
 * 正向钉，钉死清单里每个动作入口都必须带期望的那枚权限指令。
 * 反向钉，出现无权限指令的动作按钮、清单外的按钮动作入口、
 * 关窗形态以外的赋值形态点击，都判红；钉死清单里的入口在页面消失同样判红。
 */
function judgeActionButtons(source: string, fileLabel: string): string[] {
  const errors: string[] = [];
  const buttons = extractButtons(source);
  const pinnedForFile = PINNED_BUTTONS.filter(pinned => pinned.file === fileLabel);
  for (const button of buttons) {
    if (button.click === '') continue;
    if (button.handler === '') {
      if (/^[A-Za-z_$][\w$]*\s*=\s*false$/.test(button.click.replace(/\s+/g, ' '))) continue;
      errors.push(
        `${fileLabel} 按钮的 @click 形态未登记，须是动作函数调用或纯关窗赋值，实际 ${button.click}`
      );
      continue;
    }
    const pinned = pinnedForFile.find(item => item.handler === button.handler);
    if (!pinned) {
      errors.push(`${fileLabel} 动作入口 ${button.handler} 未在钉死清单登记，无法判定其权限门禁`);
      continue;
    }
    if (button.permission === '') {
      errors.push(`${fileLabel} 动作入口 ${button.handler} 缺 v-permission 权限指令`);
      continue;
    }
    const expectedExpr = expectedPermissionExpr(pinned.grants);
    const actualExpr = button.permission.replace(/\s+/g, ' ').trim();
    if (actualExpr !== expectedExpr) {
      errors.push(
        `${fileLabel} 动作入口 ${button.handler} 权限指令期望 ${expectedExpr}，实际 ${actualExpr}`
      );
      continue;
    }
    for (const grant of pinned.grants) {
      if (!(grant.perm in PERMISSIONS)) {
        errors.push(`${fileLabel} 动作入口 ${button.handler} 用到的 ${grant.perm} 不在权限常量表`);
      }
    }
  }
  for (const pinned of pinnedForFile) {
    if (!buttons.some(button => button.handler === pinned.handler)) {
      errors.push(`${fileLabel} 钉死清单里的动作入口 ${pinned.handler} 在页面里消失，锁失去覆盖`);
    }
  }
  return errors;
}

/** 取一个导出 const 的函数体文本，函数改名或被删除时显式抛错而不是空转 */
function exportedConstBody(source: string, name: string, fileLabel: string): string {
  const start = source.indexOf(`export const ${name}`);
  if (start === -1) {
    throw new Error(`${fileLabel} 里找不到导出函数 ${name}，被钉对象已漂移`);
  }
  const next = source.indexOf('\nexport const ', start + 1);
  return source.slice(start, next === -1 ? source.length : next);
}

/**
 * 列表声明的形态判据，正向钉必须是 ApiResponse 包 PaginatedResponse 的信封，
 * 反向钉出现 ApiResponse 包模型数组的裸数组形态与空数组兜底即判红。
 */
function judgeListDeclaration(
  apiSource: string,
  fnName: string,
  model: string,
  fileLabel: string
): string[] {
  const errors: string[] = [];
  const flat = exportedConstBody(apiSource, fnName, fileLabel).replace(/\s+/g, '');
  const envelope = `Promise<ApiResponse<PaginatedResponse<${model}>>>`;
  if (!flat.includes(envelope)) {
    errors.push(`${fnName} 的返回声明不是列表信封 ${envelope}`);
  }
  if (flat.includes(`ApiResponse<${model}[]>`)) {
    errors.push(`${fnName} 仍在声明裸数组载荷 ApiResponse<${model}[]>`);
  }
  if (/data\?\?\[\]/.test(flat)) {
    errors.push(`${fnName} 用空数组兜底掩盖信封漂移`);
  }
  return errors;
}

/**
 * 取一个处理函数的函数体文本，函数改名或被删除时显式抛错而不是空转。
 */
function handlerBody(source: string, handler: string, fileLabel: string): string {
  const start = source.indexOf(`const ${handler} =`);
  if (start === -1) {
    throw new Error(`${fileLabel} 里找不到处理函数 ${handler}，被钉对象已漂移`);
  }
  const next = source.indexOf('\nconst ', start + 1);
  return source.slice(start, next === -1 ? source.length : next);
}

/**
 * 动作权限门与请求载体两段的公共文件表，键是钉死清单里的文件标签。
 */
const PAGE_SOURCES: Readonly<Record<string, string>> = {
  'index.vue': PAGE_INDEX_SRC,
  'RatingDialogTab.vue': RATING_TAB_SRC,
  'AdjustDialogTab.vue': ADJUST_TAB_SRC,
  'AmountDialogTab.vue': AMOUNT_TAB_SRC,
};

/**
 * 解析一个钉死条目的请求载体，via 指向 tabs 组件文件时取该文件全文，
 * 指向同文件的另一个处理函数时取该函数体，缺省取条目自身处理函数体。
 */
function resolveCarrierSource(pinned: PinnedButton): string {
  const via = pinned.via;
  if (!via) return handlerBody(PAGE_SOURCES[pinned.file], pinned.handler, pinned.file);
  if (via.endsWith('.vue')) {
    const target = PAGE_SOURCES[via];
    if (!target) throw new Error(`钉死清单的 via 指向未知文件 ${via}`);
    return target;
  }
  return handlerBody(PAGE_SOURCES[pinned.file], via, pinned.file);
}

/**
 * 请求载体判据，正向钉每个条目的 api 函数都要在其载体里真的被调用，
 * 反向钉入口改打别的端点或 via 指向虚处即判红，防钉死清单与页面代码脱钩。
 */
function judgeGrantCarriers(pinnedList: readonly PinnedButton[]): string[] {
  const errors: string[] = [];
  for (const pinned of pinnedList) {
    const carrier = resolveCarrierSource(pinned);
    for (const grant of pinned.grants) {
      if (!carrier.includes(`${grant.apiFn}(`)) {
        errors.push(
          `${pinned.file} 的 ${pinned.handler} 声明调用 ${grant.apiFn}，载体里没有该调用`
        );
      }
    }
  }
  return errors;
}

/**
 * 后端 Decimal 出参的三个金额键，线形态由 Cargo.toml 的特性集决定。
 */
const MONEY_KEYS = ['credit_limit', 'used_credit', 'available_credit'] as const;

/**
 * 会把 Decimal 出参变成 JSON 数字的 rust_decimal 特性清单，
 * 与后端同名锁的取值逐字同源，两侧任一枚改动都必须同步另一枚。
 */
const FLOAT_FORMING_FEATURES = [
  'serde-float',
  'serde-with-float',
  'serde-arbitrary-precision',
  'serde-with-arbitrary-precision',
];

/** 取一段 TS 源里指定 interface 的块文本，块缺失或没闭合时显式抛错 */
function tsInterfaceBlock(apiSrc: string, name: string, fileLabel: string): string {
  const start = apiSrc.indexOf(`export interface ${name} {`);
  if (start === -1) {
    throw new Error(`${fileLabel} 里找不到 interface ${name}，被钉对象已漂移`);
  }
  const end = apiSrc.indexOf('\n}', start);
  if (end === -1) {
    throw new Error(`${fileLabel} 里 interface ${name} 块未闭合，无法解析`);
  }
  return apiSrc.slice(start, end);
}

/** 取接口块里某个键声明的类型词，可选标记与分号剥掉，键不存在时返回空串 */
function declaredTypeOfKey(block: string, key: string): string {
  const line = block
    .split('\n')
    .map(text => text.trim())
    .find(text => text.startsWith(`${key}?`) || text.startsWith(`${key}:`));
  if (!line) return '';
  const colonAt = line.indexOf(':');
  if (colonAt === -1) return '';
  return line
    .slice(colonAt + 1)
    .trim()
    .replace(/;$/, '')
    .trim();
}

/** 取 Cargo.toml 的 rust_decimal 依赖声明行，只认赋值行以避开 sea-orm 的特性名 */
function rustDecimalDepLine(cargo: string): string {
  const line = cargo
    .split('\n')
    .map(text => text.trim())
    .find(text => text.startsWith('rust_decimal =') || text.startsWith('rust_decimal='));
  if (!line) {
    throw new Error('后端 Cargo.toml 里找不到 rust_decimal 依赖声明行，判据失去依据');
  }
  return line;
}

/** 由依赖声明行推出 Decimal 线形态是否为字符串，未启浮点化特性即字符串 */
function decimalWireIsString(cargo: string): boolean {
  const line = rustDecimalDepLine(cargo);
  return !FLOAT_FORMING_FEATURES.some(feature => line.includes(feature));
}

/**
 * 金额键声明的判据主体，输入 api 文本、Cargo 文本与实体文本，输出违规清单。
 * 正向钉三键的声明类型词与后端线形态同词，未启浮点化特性时必须是 string，
 * 且实体侧这三列确是 Decimal；反向钉回到 number、写成字符串与数值的联合形态、
 * 实体列改成非 Decimal 都判红，判据不靠猜也不允许兜底联合。
 */
function judgeMoneyKeyDeclarations(
  apiSrc: string,
  cargo: string,
  modelRs: string,
  fileLabel: string
): string[] {
  const errors: string[] = [];
  const block = tsInterfaceBlock(apiSrc, 'CustomerCredit', fileLabel);
  const expected = decimalWireIsString(cargo) ? 'string' : 'number';
  for (const key of MONEY_KEYS) {
    const declared = declaredTypeOfKey(block, key);
    if (declared === '') {
      errors.push(`${key} 在 CustomerCredit 接口里不存在，出参声明被删或改名`);
      continue;
    }
    if (declared !== expected) {
      const reason =
        expected === 'string'
          ? '后端 Decimal 线形态是字符串，声明回 number 会让按数值取数的消费方拿到字符串却不自知'
          : '后端 Cargo.toml 已启浮点化特性，Decimal 出参变成 JSON 数字，声明仍写 string 会让消费方拿到数字却按字符串处理';
      errors.push(`${key} 声明成 ${declared}，期望 ${expected}，${reason}`);
    }
    const column = modelRs.match(new RegExp(`pub ${key}:\\s*([^,\\n]+)`));
    const columnType = column ? column[1].trim() : '';
    if (!/^(rust_decimal::)?Decimal$/.test(columnType)) {
      errors.push(`${key} 的后端列类型是 ${columnType || '未取到'}，不是 Decimal，两侧需同步裁定`);
    }
  }
  return errors;
}

/**
 * 金额键消费侧的判据，归一只允许发生在真正要算的地方。
 * 正向钉列表页的比较走归一函数、编辑回填在控件边界归一、提交前转回写线形态；
 * 反向钉裸比较、把线形态直接喂给数值控件、整个表单直发与在类型层把表单声明回
 * 实体部分形态都判红，NaN 一律不许静默进渲染。
 */
function judgeMoneyKeyConsumers(pageSrc: string): string[] {
  const errors: string[] = [];
  if (!/decimalWireToNumber\(/.test(pageSrc)) {
    errors.push('列表页对金额键做比较却没走归一函数，字符串比较会静默失真');
  }
  if (!/numberToDecimalWire\(/.test(pageSrc)) {
    errors.push('编辑提交没把数值控件的取值转回后端线形态');
  }
  if (
    /\b(?:row|detailRecord)\.(?:credit_limit|used_credit|available_credit)\s*[<>]/.test(pageSrc)
  ) {
    errors.push('金额键被裸比较，后端线形态是字符串，必须先归一再比');
  }
  if (/credit_limit:\s*row\.credit_limit/.test(pageSrc)) {
    errors.push('金额键的线形态被直接回填进数值控件，缺值与脏串无法区分');
  }
  if (/updateCustomerCredit\([^)]*editForm\.value\)/.test(pageSrc)) {
    errors.push('编辑提交把整个数值控件表单直发，金额键没做写线转换');
  }
  if (/const editForm = ref<Partial<CustomerCredit>>/.test(pageSrc)) {
    errors.push('编辑表单在类型层把金额键声明回实体形态，等于假装它是 number');
  }
  if (
    /\bNumber\((?:row|detailRecord|value)\.(?:credit_limit|used_credit|available_credit)\)/.test(
      pageSrc
    )
  ) {
    errors.push('金额键被就地 Number 转换，缺值与不可解析没显式判');
  }
  const callAt = pageSrc.indexOf('decimalWireToNumber(');
  if (callAt !== -1) {
    const start = pageSrc.lastIndexOf('\nconst ', callAt);
    const end = pageSrc.indexOf('\n};', callAt);
    if (start === -1 || end === -1) {
      errors.push('取不到归一调用所在的函数体，判据对该调用失去覆盖');
    } else {
      const body = pageSrc.slice(start, end);
      if (!/===\s*(undefined|null)/.test(body) || !/===\s*''/.test(body)) {
        errors.push('归一调用点没把未填与空串显式判掉，缺值会静默算成 0 进渲染');
      }
    }
  }
  return errors;
}

describe('客户信用列表页动作权限门与列表信封静态锁', () => {
  it('该页四个源文件里每个动作按钮都带钉死的权限指令，反向钉裸按钮与未登记入口', () => {
    const files: Array<{ label: string; src: string }> = [
      { label: 'index.vue', src: PAGE_INDEX_SRC },
      { label: 'RatingDialogTab.vue', src: RATING_TAB_SRC },
      { label: 'AdjustDialogTab.vue', src: ADJUST_TAB_SRC },
      { label: 'AmountDialogTab.vue', src: AMOUNT_TAB_SRC },
    ];
    // 按钮数量地板，防切分判据退化成切不到也算过。该页现有 11 个按钮
    expect(extractButtons(PAGE_INDEX_SRC).length, '列表页按钮数量地板').toBeGreaterThanOrEqual(11);
    const errors = files.flatMap(entry => judgeActionButtons(entry.src, entry.label));
    expect(errors, `动作按钮权限门违规\n${errors.join('\n')}`).toEqual([]);
  });

  it('用到的每枚 customer-credits 键都在权限常量表在册且由后端派生规则算出', () => {
    for (const pinned of PINNED_BUTTONS) {
      for (const grant of pinned.grants) {
        const declared = PERMISSION_KEYS[grant.perm];
        expect(
          declared,
          `${pinned.file} 的 ${pinned.handler} 用到未声明的常量 ${grant.perm}`
        ).toBeDefined();
        const endpoint = creditEndpoints.get(grant.apiFn);
        expect(endpoint, `api 层找不到函数 ${grant.apiFn}`).toBeDefined();
        const derived = derivePermissionKey(endpoint!.method, endpoint!.path);
        expect(
          derived,
          `${pinned.file} 的 ${pinned.handler} 声明 ${declared}，后端按 ${grant.apiFn} 的 ` +
            `${endpoint!.method} ${endpoint!.path} 派生成 ${derived}，两侧不同词`
        ).toBe(declared);
        expect(declared.split(':')[0], `${declared} 的资源段不是 customer-credits`).toBe(
          'customer-credits'
        );
        expect(
          assignableActions.has(declared.split(':')[1]),
          `${declared} 的动作段后端不可派生`
        ).toBe(true);
      }
    }
  });

  it('钉死清单声明的请求载体与页面代码同源，入口改打别的端点即判红', () => {
    expect(
      judgeGrantCarriers(PINNED_BUTTONS),
      '动作入口与其声明调用的 api 函数脱钩\n' + judgeGrantCarriers(PINNED_BUTTONS).join('\n')
    ).toEqual([]);
  });

  it('api 层全部端点派生出的键都在权限常量表在册，缺键即判红', () => {
    const declaredValues = new Set<string>(Object.values(PERMISSION_KEYS));
    for (const [fnName, endpoint] of creditEndpoints) {
      const derived = derivePermissionKey(endpoint.method, endpoint.path);
      expect(
        declaredValues.has(derived),
        `${fnName} 打的 ${endpoint.method} ${endpoint.path} 派生成 ${derived}，权限常量表未声明该键`
      ).toBe(true);
    }
  });

  it('列表函数声明是 items total 信封，前端类型与后端载荷同形', () => {
    expect(
      judgeListDeclaration(CREDIT_API_SRC, 'getCustomerCreditList', 'CustomerCredit', 'api 层'),
      '客户信用列表出参声明不是信封形态'
    ).toEqual([]);
    expect(
      TYPES_API_SRC,
      '前端 PaginatedResponse 类型不再是 items total page page_size 四键，信封声明失去载体'
    ).toMatch(
      /export interface PaginatedResponse<T = unknown> \{\s*items: T\[\];\s*total: number;\s*page: number;\s*page_size: number;\s*\}/
    );
    expect(
      RESPONSE_RS,
      '后端 PaginatedResponse 结构不再是 items total page page_size 四字段，与前端信封声明脱钩'
    ).toMatch(
      /pub struct PaginatedResponse<T> \{\s*pub items: Vec<T>,\s*pub total: u64,\s*pub page: u64,\s*pub page_size: u64,/
    );
    expect(
      CREDIT_HANDLER_RS,
      '后端 list_credits 出参不再是 ApiResponse 包 PaginatedResponse，前端信封声明随之失效'
    ).toMatch(
      /pub async fn list_credits\([\s\S]*?\) -> Result<Json<ApiResponse<PaginatedResponse<customer_credit::Model>>>, AppError>/
    );
  });

  it('列表页按 items 键取数且没有空数组兜底', () => {
    expect(PAGE_INDEX_SRC, '列表页未按 items 键点明取数字段，与 api 层信封声明脱钩').toMatch(
      /listKey:\s*'items'/
    );
    expect(PAGE_INDEX_SRC, '列表页不得用空数组兜底掩盖信封漂移').not.toMatch(/data\s*\?\?\s*\[\]/);
    expect(CREDIT_API_SRC, 'api 层不得用空数组兜底掩盖信封漂移').not.toMatch(/data\s*\?\?\s*\[\]/);
    const declared = creditEndpoints.get('getCustomerCreditList');
    expect(declared?.method).toBe('GET');
    expect(declared?.path).toBe('/crm/customer-credits');
  });

  it('出参三个金额键的声明与后端 Decimal 线形态同词，回到 number 即判红', () => {
    expect(
      decimalWireIsString(CARGO_TOML),
      '后端 rust_decimal 已启浮点化特性，本锁的期望要重裁'
    ).toBe(true);
    expect(
      judgeMoneyKeyDeclarations(CREDIT_API_SRC, CARGO_TOML, CREDIT_MODEL_RS, 'api 层'),
      '金额键声明与后端 Decimal 线形态不同词'
    ).toEqual([]);
  });

  it('金额键的归一发生在真正要算的地方，裸比较与整表直发即判红', () => {
    expect(
      judgeMoneyKeyConsumers(PAGE_INDEX_SRC),
      `金额键消费侧违规\n${judgeMoneyKeyConsumers(PAGE_INDEX_SRC).join('\n')}`
    ).toEqual([]);
  });

  it('判据自身的检测力，裸按钮与裸数组与金额声明漂移三类缺陷形态必须都被抓到', () => {
    // 夹具一，缺陷形态的裸按钮，动作入口没有权限指令
    const bareButtonSrc = [
      '<template>',
      '  <el-button type="primary" @click="openRatingDialog">设置评级</el-button>',
      '  <el-button link type="danger" @click="handleDelete(row)">删除</el-button>',
      '</template>',
      '',
    ].join('\n');
    const bareButtonErrors = judgeActionButtons(bareButtonSrc, 'index.vue');
    expect(
      bareButtonErrors.filter(message => message.includes('缺 v-permission 权限指令')),
      '判据没抓到无权限指令的动作按钮'
    ).toHaveLength(2);

    // 夹具二，缺陷形态的错键与未登记入口
    const wrongKeySrc = [
      '<template>',
      '  <el-button v-permission="PERMISSIONS.CUSTOMER_CREDIT_DELETE" @click="openRatingDialog">评级</el-button>',
      '  <el-button @click="handleExport(row)">导出</el-button>',
      '</template>',
      '',
    ].join('\n');
    const wrongKeyErrors = judgeActionButtons(wrongKeySrc, 'index.vue');
    expect(
      wrongKeyErrors.some(message => message.includes('openRatingDialog 权限指令期望')),
      '判据没抓到用错键的动作入口'
    ).toBe(true);
    expect(
      wrongKeyErrors.some(message => message.includes('handleExport 未在钉死清单登记')),
      '判据没抓到未登记的按钮动作入口'
    ).toBe(true);

    // 夹具三，缺陷形态的裸数组列表声明
    const bareArrayApiSrc = [
      'export const getCustomerCreditList = (',
      '  params?: CustomerCreditQueryParams',
      '): Promise<ApiResponse<CustomerCredit[]>> => request.get(`/crm/customer-credits`, { params });',
      '',
    ].join('\n');
    const bareArrayErrors = judgeListDeclaration(
      bareArrayApiSrc,
      'getCustomerCreditList',
      'CustomerCredit',
      '夹具'
    );
    expect(
      bareArrayErrors.filter(message => message.includes('不是列表信封')),
      '判据没把裸数组声明判成非信封形态'
    ).toHaveLength(1);
    expect(
      bareArrayErrors.filter(message => message.includes('仍在声明裸数组')),
      '判据没抓到裸数组载荷'
    ).toHaveLength(1);

    // 夹具四，缺陷形态的空数组兜底
    const fallbackApiSrc = [
      'export const getCustomerCreditList = (params?: CustomerCreditQueryParams) => {',
      '  return request.get(`/crm/customer-credits`, { params }).then(res => res.data ?? []);',
      '};',
      '',
    ].join('\n');
    expect(
      judgeListDeclaration(fallbackApiSrc, 'getCustomerCreditList', 'CustomerCredit', '夹具'),
      '判据没抓到空数组兜底'
    ).toContain('getCustomerCreditList 用空数组兜底掩盖信封漂移');

    // 夹具五，请求载体判据本身要能区分，声明的调用不在载体里必须判红
    const bogusCarrier: PinnedButton[] = [
      {
        file: 'index.vue',
        handler: 'handleViewDetail',
        grants: [{ apiFn: 'deleteCustomerCredit', perm: 'CUSTOMER_CREDIT_DELETE' }],
      },
    ];
    expect(judgeGrantCarriers(bogusCarrier), '判据没抓到入口与其声明调用的 api 函数脱钩').toContain(
      'index.vue 的 handleViewDetail 声明调用 deleteCustomerCredit，载体里没有该调用'
    );
    expect(judgeGrantCarriers(PINNED_BUTTONS)).toEqual([]);

    // 夹具六，缺陷形态的金额声明，三键回到 number
    const numberMoneyApiSrc = [
      'export interface CustomerCredit {',
      '  id?: number;',
      '  credit_limit?: number;',
      '  used_credit?: number;',
      '  available_credit?: number;',
      '}',
      '',
    ].join('\n');
    const numberMoneyErrors = judgeMoneyKeyDeclarations(
      numberMoneyApiSrc,
      CARGO_TOML,
      CREDIT_MODEL_RS,
      '夹具'
    );
    expect(numberMoneyErrors).toHaveLength(3);
    for (const message of numberMoneyErrors) {
      expect(message).toContain(
        '后端 Decimal 线形态是字符串，声明回 number 会让按数值取数的消费方拿到字符串却不自知'
      );
    }

    // 夹具七，缺陷形态的金额声明，写成字符串与数值的联合形态
    const unionMoneyApiSrc = [
      'export interface CustomerCredit {',
      '  credit_limit?: string | number;',
      '  used_credit?: string;',
      '  available_credit?: string;',
      '}',
      '',
    ].join('\n');
    expect(
      judgeMoneyKeyDeclarations(unionMoneyApiSrc, CARGO_TOML, CREDIT_MODEL_RS, '夹具'),
      '判据没抓到金额键的联合形态声明'
    ).toEqual([
      'credit_limit 声明成 string | number，期望 string，后端 Decimal 线形态是字符串，声明回 number 会让按数值取数的消费方拿到字符串却不自知',
    ]);

    // 夹具八，缺陷形态的消费侧，裸比较加线形态直接回填控件加整表直发
    const rawConsumerPageSrc = [
      '<template>',
      '  <span :style="{ color: row.available_credit > 0 ? 1 : 2 }">{{ row.available_credit }}</span>',
      '</template>',
      '<script setup lang="ts">',
      'const editForm = ref<Partial<CustomerCredit>>({});',
      'const handleEdit = (row: CustomerCredit) => {',
      '  editForm.value = { credit_limit: row.credit_limit };',
      '};',
      'const submitEdit = async () => {',
      '  await updateCustomerCredit(1, editForm.value);',
      '};',
      '</script>',
      '',
    ].join('\n');
    const rawConsumerErrors = judgeMoneyKeyConsumers(rawConsumerPageSrc);
    expect(rawConsumerErrors.filter(message => message.includes('金额键被裸比较'))).toHaveLength(1);
    expect(rawConsumerErrors.filter(message => message.includes('没走归一函数'))).toHaveLength(1);
    expect(
      rawConsumerErrors.filter(message => message.includes('直接回填进数值控件'))
    ).toHaveLength(1);
    expect(
      rawConsumerErrors.filter(message => message.includes('整个数值控件表单直发'))
    ).toHaveLength(1);
    expect(rawConsumerErrors.filter(message => message.includes('假装它是 number'))).toHaveLength(
      1
    );
    expect(
      rawConsumerErrors.filter(message => message.includes('没把数值控件的取值转回'))
    ).toHaveLength(1);

    // 夹具八之二，归一调用点没有显式判未填与空串，缺值会被静默算成 0
    const noGuardConsumerSrc = [
      '<script setup lang="ts">',
      'const isPositive = (value: string | undefined): boolean => {',
      '  return decimalWireToNumber(value) > 0;',
      '};',
      'const submitEdit = async () => {',
      '  await updateCustomerCredit(1, { credit_limit: numberToDecimalWire(1) });',
      '};',
      '</script>',
      '',
    ].join('\n');
    expect(judgeMoneyKeyConsumers(noGuardConsumerSrc), '判据没抓到归一调用点缺显式判').toContain(
      '归一调用点没把未填与空串显式判掉，缺值会静默算成 0 进渲染'
    );

    // 夹具八之三，就地 Number 转换金额键，缺值与不可解析没显式判
    const inlineNumberConsumerSrc = [
      '<script setup lang="ts">',
      'const color = (row: CustomerCredit): string =>',
      '  Number(row.available_credit) > 0 ? green : red;',
      'const submitEdit = async () => {',
      '  await updateCustomerCredit(1, { credit_limit: numberToDecimalWire(1) });',
      '};',
      '</script>',
      '',
    ].join('\n');
    expect(
      judgeMoneyKeyConsumers(inlineNumberConsumerSrc),
      '判据没抓到金额键的就地 Number 转换'
    ).toContain('金额键被就地 Number 转换，缺值与不可解析没显式判');

    // 夹具八之四，归一调用点所在函数体切不出来时判据必须点名失去覆盖而不是静默放过
    const unlocatableConsumerSrc = [
      '<script setup lang="ts">',
      'const flag = decimalWireToNumber(value) > 0;',
      '</script>',
      '',
    ].join('\n');
    expect(
      judgeMoneyKeyConsumers(unlocatableConsumerSrc),
      '判据切不出归一调用所在函数体时必须点名'
    ).toContain('取不到归一调用所在的函数体，判据对该调用失去覆盖');

    // 夹具九，期望形态由后端特性集推出而非写死，启了浮点化特性后 string 反而判红
    const floatFormingCargo =
      'rust_decimal = { version = "1.0", features = ["serde", "serde-float"] }';
    expect(decimalWireIsString(floatFormingCargo)).toBe(false);
    expect(
      judgeMoneyKeyDeclarations(CREDIT_API_SRC, floatFormingCargo, CREDIT_MODEL_RS, '夹具'),
      '特性集翻转时期望没跟着翻，说明判据写死了 string'
    ).toHaveLength(3);

    // 夹具十，猜出来的键必须派生不出来，派生判据本身必须显式失败
    const bogusKey = 'customer-credits:transfer';
    expect(assignableActions.has(bogusKey.split(':')[1]), '未在册动作段竟被判成可派生').toBe(false);
    expect(Object.values(PERMISSION_KEYS)).not.toContain(bogusKey);
    expect(derivePermissionKey('POST', '/crm/customer-credits/{id}/release')).toBe(
      'customer-credits:release'
    );
    expect(derivePermissionKey('POST', '/crm/customer-credits/{id}/rating')).toBe(
      'customer-credits:create'
    );
    expect(derivePermissionKey('GET', '/crm/customer-credits/{id}')).toBe('customer-credits:read');
    expect(derivePermissionKey('DELETE', '/crm/customer-credits/{id}')).toBe(
      'customer-credits:delete'
    );
    expect(() => derivePermissionKey('GET', '/crm/nonexistent-resource')).toThrow('找不到在册资源');
    expect(() => exportedConstBody(CREDIT_API_SRC, 'notExportedAtAll', 'api 层')).toThrow(
      '被钉对象已漂移'
    );
  });
});
