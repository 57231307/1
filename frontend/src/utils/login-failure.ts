/**
 * 登录连续失败计数与壳内回弹，单点判定。
 *
 * 用途，两类失败都算一次，一是用户所填地址可达但并不是目标 ERP（例如端口开在另一个服务上），
 * 页面打得开、提交一直拿 401；二是提交时拿不到响应或网络错，服务器此刻连不上。
 * 两类都不该一次就把用户退回配置窗，据此把连续失败累计，达到上限才调壳内命令回弹 gateway
 * 卡页重填地址，卡页按回弹标记渲染空表单不带旧地址；未达上限只在页内计数并保留请求层提示。
 * 调用方 Login 视图的凭证失败与不可达分支，以及登录成功分支的清零。
 * 入参 host 为当前部署站点 origin，计数按 host 归组，换 host 从 1 重计。
 * 传给谁 recordLoginFailure 与 clearLoginFailure 管计数，invokeReopenGateway 经
 * window.__TAURI__ 调壳内 reopen_gateway，壳侧剥 query 存稳定基址并自行追加 reconfigure=1。
 * 存什么 host 与 count 两个字段的记录，登录成功或已回弹即清除。
 * 存哪里 浏览器 localStorage，键 bingxi.login.failCount，与壳内卡页的 bingxi_server_url 同一命名空间。
 */
export const LOGIN_FAIL_COUNT_KEY = 'bingxi.login.failCount';

/** 连续失败（凭证不对或连不上）达到该次数即回弹卡页重填，5 次是"多半配置不对"与"只是记性差"的分界。 */
export const REOPEN_GATEWAY_AFTER_FAILURES = 5;

export interface LoginFailureRecord {
  host: string;
  count: number;
}

/** 可注入的最小存储面，默认取 window.localStorage，单测传内存实现。 */
export interface KeyValueStore {
  getItem(key: string): string | null;
  setItem(key: string, value: string): void;
  removeItem(key: string): void;
}

function defaultStore(): KeyValueStore {
  return window.localStorage;
}

/** 解析存储值，空值，非法 JSON，缺字段或计数非正一律按无记录返回 null。 */
export function parseLoginFailureRecord(raw: string | null): LoginFailureRecord | null {
  if (!raw) return null;
  try {
    const parsed = JSON.parse(raw) as Partial<LoginFailureRecord>;
    if (
      typeof parsed.host === 'string' &&
      typeof parsed.count === 'number' &&
      Number.isFinite(parsed.count) &&
      parsed.count >= 1
    ) {
      return { host: parsed.host, count: Math.floor(parsed.count) };
    }
  } catch {
    // 存储值损坏按无记录处理，下一次从 1 重计，不影响登录主流程
  }
  return null;
}

/** 同 host 累计，换 host（服务搬家或改了地址）从 1 重计，纯函数便于单测。 */
export function nextLoginFailureRecord(
  prev: LoginFailureRecord | null,
  host: string
): LoginFailureRecord {
  if (prev && prev.host === host) return { host, count: prev.count + 1 };
  return { host, count: 1 };
}

/** 记一次凭证失败并返回新记录，存储不可用时按本次读出值判定，不阻塞调用方。 */
export function recordLoginFailure(
  host: string,
  store: KeyValueStore = defaultStore()
): LoginFailureRecord {
  const next = nextLoginFailureRecord(
    parseLoginFailureRecord(store.getItem(LOGIN_FAIL_COUNT_KEY)),
    host
  );
  try {
    store.setItem(LOGIN_FAIL_COUNT_KEY, JSON.stringify(next));
  } catch {
    // 隐私模式写入抛错，计数不落盘，本次返回值仍然有效
  }
  return next;
}

/** 清除连续失败计数，登录成功与已回弹后调用。 */
export function clearLoginFailure(store: KeyValueStore = defaultStore()): void {
  try {
    store.removeItem(LOGIN_FAIL_COUNT_KEY);
  } catch {
    // 存储不可用时无计数可清
  }
}

/** 是否已到回弹阈值。 */
export function shouldReopenGateway(record: LoginFailureRecord): boolean {
  return record.count >= REOPEN_GATEWAY_AFTER_FAILURES;
}

/**
 * 经壳内桥跳回 gateway 卡页重填地址。
 * 调用方 网络不可达分支与连续 401 达到阈值分支，纯 Web 环境无桥时什么都不做。
 * 入参 无，命令由壳侧执行并在稳定基址上追加 reconfigure=1 标记。
 */
export function invokeReopenGateway(): void {
  const tauri = (
    window as unknown as {
      __TAURI__?: { core?: { invoke?: (cmd: string, args?: unknown) => Promise<unknown> } };
    }
  ).__TAURI__;
  if (tauri?.core?.invoke) {
    void tauri.core.invoke('reopen_gateway').catch(() => {
      // 基址未记录等壳侧失败不掩盖在页面上，用户仍可用返回手势，不在登录页再叠一层报错
    });
  }
}
