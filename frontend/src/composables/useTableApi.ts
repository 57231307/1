/**
 * useTableApi - 通用表格数据 composable
 */
import { ref, watch, onBeforeUnmount } from 'vue';
import type { Ref } from 'vue';
import { request } from '@/api/request';
import { logger } from '@/utils/logger';
import { msg } from '@/utils/message';
import type { ApiResponse } from '@/types/api';

/**
 * 契约/形状不符错误：后端响应结构漂移（认不出列表字段，或分页响应缺总数）。
 *
 * 这类错误是**永久性**的——同一请求重试多少次都会得到同样的错误结构，重试只会
 * 放大请求量（叠加 axios 拦截器对幂等 GET 的自动退避重试，放大更严重）。fetchData
 * 的重试判定以「是否为 SchemaMismatchError」区分它与网络/5xx 瞬态错误：瞬态才重试，
 * 契约错误直接失败上抛。用显式类型 + `isSchemaMismatch` 标记位判定，不做 message
 * 字符串匹配（instanceof 在打包/跨 realm 下可能失效，标记位是兜底判据）。
 */
export class SchemaMismatchError extends Error {
  /** 错误发生环节：'list' = 认不出列表字段；'total' = 分页响应缺总数 */
  readonly kind: 'list' | 'total';
  /** 出错请求的 url（供上报通道定位到具体表格） */
  readonly url: string;
  /** 类型标记位：即使 instanceof 因边界失效也能据此判定为契约错误，不重试 */
  readonly isSchemaMismatch = true;

  constructor(kind: 'list' | 'total', url: string, message: string) {
    super(message);
    this.name = 'SchemaMismatchError';
    this.kind = kind;
    this.url = url;
    // 维持原型链，保证 transpile 到 ES5 目标时 instanceof 仍可用
    Object.setPrototypeOf(this, SchemaMismatchError.prototype);
  }
}

/**
 * 判定错误是否为「契约/形状错误」（不可重试）。
 * instanceof 为主判据，`isSchemaMismatch` 标记位为兜底（跨 realm/打包降级时 instanceof
 * 可能失效），二者任一命中即视为契约错误。绝不匹配 message 文案。
 */
export function isSchemaMismatchError(err: unknown): err is SchemaMismatchError {
  return (
    err instanceof SchemaMismatchError ||
    (typeof err === 'object' &&
      err !== null &&
      (err as { isSchemaMismatch?: unknown }).isSchemaMismatch === true)
  );
}

/**
 * 契约错误 toast 去重窗口（毫秒）：同一表格在窗口内重复触发同一契约错误时只弹一条，
 * 避免翻页/刷新抖动或并发多表把同类结构漂移刷屏。网络/5xx 由 axios 拦截器负责提示，
 * 本 composable 不重复弹，故此处只约束契约错误的 toast。
 */
const SCHEMA_TOAST_DEDUP_MS = 3000;

/**
 * 表格接口响应的可识别字段
 * 后端部分接口使用 list/顶层，部分使用 data.items/嵌套
 */
interface ListResponsePayload {
  data?: unknown;
  list?: unknown;
  items?: unknown;
  results?: unknown;
  total?: number;
  count?: number;
}

export interface UseTableApiOptions {
  url: string;
  defaultParams?: Record<string, unknown>;
  defaultPageSize?: number;
  pageKey?: string;
  pageSizeKey?: string;
  totalKey?: string;
  listKey?: string;
  retryCount?: number;
  retryDelay?: number;
  onError?: (err: unknown) => void;
}

export interface UseTableApiReturn<T = unknown> {
  data: Ref<T[]>;
  total: Ref<number>;
  loading: Ref<boolean>;
  page: Ref<number>;
  pageSize: Ref<number>;
  queryParams: Ref<Record<string, unknown>>;
  refresh: () => Promise<void>;
  reset: () => void;
  setQueryParam: (key: string, value: unknown) => void;
}

/**
 * 通用表格数据获取 composable
 * 支持分页 / 筛选 / 排序 / loading / 错误重试
 */
export function useTableApi<T = unknown>(
  optionsOrUrl: UseTableApiOptions | string
): UseTableApiReturn<T> {
  const options: UseTableApiOptions =
    typeof optionsOrUrl === 'string' ? { url: optionsOrUrl } : optionsOrUrl;

  const {
    url,
    defaultParams = {},
    defaultPageSize = 20,
    pageKey = 'page',
    pageSizeKey = 'page_size',
    totalKey = 'total',
    listKey = 'list',
    retryCount = 2,
    retryDelay = 1000,
    onError,
  } = options;

  const data = ref<T[]>([]) as Ref<T[]>;
  const total = ref(0);
  const loading = ref(false);
  const page = ref(1);
  const pageSize = ref(defaultPageSize);
  const queryParams = ref<Record<string, unknown>>({ ...defaultParams });

  /**
   * 从响应中取列表字段
   * - 优先匹配 options.listKey 指定字段名
   * - 后备：list / items / data / results（本仓既有端点确实分用这几个键）
   * 都不匹配时**必须报错**：这里返回 [] 会把"契约漂移"伪装成"这张表没有数据"，
   * 用户看到的是空表而不是失败，缺陷永远暴露不出来。
   */
  const detectList = (payload: ListResponsePayload | unknown[]): T[] => {
    if (Array.isArray(payload)) return payload as T[];
    // 优先按 listKey 指定的字段名取
    const obj = payload as ListResponsePayload;
    if (listKey && Array.isArray(obj?.[listKey as keyof ListResponsePayload])) {
      return obj[listKey as keyof ListResponsePayload] as unknown as T[];
    }
    if (Array.isArray(obj?.list)) return obj.list as unknown as T[];
    if (Array.isArray(obj?.items)) return obj.items as unknown as T[];
    if (Array.isArray(obj?.data)) return obj.data as unknown as T[];
    if (Array.isArray(obj?.results)) return obj.results as unknown as T[];
    throw new SchemaMismatchError(
      'list',
      url,
      `[useTableApi] ${url} 响应里没有可识别的列表字段：` +
        `期望 ${listKey ?? 'list/items/data/results'}，实际键为 ${JSON.stringify(Object.keys(obj))}`
    );
  };

  const detectTotal = (payload: ListResponsePayload, rawWasArray: boolean): number => {
    // 顶层/嵌套 total 优先：`{code, data:[...], total:N}` 这类形态的总数在包装时已并入 payload
    if (typeof payload?.[totalKey as keyof ListResponsePayload] === 'number') {
      return payload[totalKey as keyof ListResponsePayload] as number;
    }
    if (typeof payload?.total === 'number') return payload.total;
    if (typeof payload?.count === 'number') return payload.count;
    // 裸数组端点不分页：服务端给的就是全量，行数即总数
    if (rawWasArray && Array.isArray(payload?.data)) return payload.data.length;
    // 分页端点漏 total 会让"共 N 条/页码"恒显 0 而把数据静默截断，必须显式失败。
    throw new SchemaMismatchError(
      'total',
      url,
      `[useTableApi] ${url} 的分页响应缺少总数字段：` +
        `期望 ${totalKey}/count，实际键为 ${JSON.stringify(Object.keys(payload))}`
    );
  };

  // 契约错误 toast 去重（实例级）：记录上一次弹给用户的错误文案与时间，
  // 同一表格短时间内重复的同类失败只弹一条，避免 refresh/翻页连点刷屏。
  let lastSchemaToastMsg = '';
  let lastSchemaToastAt = 0;

  /**
   * 终端失败上报：契约/形状错误必须「可见」——
   * - logger.error：技术细节（期望键 vs 实际键、url），始终输出（生产环境亦用于监控）；
   * - msg toast：仅契约错误弹（网络/5xx 已由 axios 拦截器统一弹，重复弹会双条）；
   *   若调用方传了 onError，则由调用方自行决定 UI（本处不再弹，避免与既有提示叠加）。
   * 单次去重：同一表格同一契约错误在去重窗口内只弹一次。
   */
  const reportTerminalError = (err: unknown, isSchema: boolean): void => {
    const detail = err instanceof Error ? err.message : String(err);
    logger.error(
      `[useTableApi] ${url} 列表取数失败（${isSchema ? '响应结构不符合契约' : '请求失败'}）`,
      err
    );
    onError?.(err);
    if (!isSchema || onError) {
      // 网络/5xx：交给 axios 提示；有 onError：调用方已处理 UI——均不在此重复弹
      return;
    }
    const now = Date.now();
    if (detail !== lastSchemaToastMsg || now - lastSchemaToastAt >= SCHEMA_TOAST_DEDUP_MS) {
      lastSchemaToastMsg = detail;
      lastSchemaToastAt = now;
      // message.loadFailed 中英双侧已存在；技术细节已进 logger，此处仅给用户「加载失败」级提示
      msg.error('loadFailed');
    }
  };

  /**
   * 核心请求函数：网络/5xx 瞬态错误重试；契约/形状错误永久性，不重试。
   * @param attempt 当前重试次数
   * @param throwOnFail 终端失败是否向上抛（refresh 手动调用需要向调用方传播；
   *                    初始加载 / 分页 watch 的悬浮调用传 false，避免未处理的 Promise rejection
   *                    沦为纯控制台噪音——失败已在 reportTerminalError 里显式上报）。
   */
  const fetchData = async (attempt = 0, throwOnFail = false): Promise<void> => {
    loading.value = true;
    try {
      const params: Record<string, unknown> = {
        ...queryParams.value,
        [pageKey]: page.value,
        [pageSizeKey]: pageSize.value,
      };
      // 表格接口响应：ApiResponse 包装的 list/total 或裸 list/total
      const res = await request.get<
        ApiResponse<ListResponsePayload | T[]> | ListResponsePayload | T[]
      >(url, { params });
      // 兼容三种返回：ApiResponse 包装 / 裸 list / 裸对象
      // 当 res.data 是裸数组（如 `{ data: T[], total: number }`）时，
      // 需要把 res 外层的 total/count 保留到 payload，否则 detectTotal 会丢失总数。
      const resObj = res as ListResponsePayload;
      const raw: ListResponsePayload | T[] =
        (res as { data?: unknown })?.data ?? (res as ListResponsePayload | T[]);
      const rawWasArray = Array.isArray(raw);
      const payload: ListResponsePayload = rawWasArray
        ? { data: raw, total: resObj?.total, count: resObj?.count }
        : (raw ?? {});
      data.value = detectList(payload) as T[];
      total.value = detectTotal(payload, rawWasArray);
    } catch (err) {
      const isSchema = isSchemaMismatchError(err);
      // 契约/形状错误是永久性的，重试只会放大请求量（且 axios 对幂等 GET 还会自动退避重试）——
      // 直接落到终端上报，不进入重试。仅网络/5xx 瞬态错误按 retryCount 重试。
      if (!isSchema && attempt < retryCount) {
        await new Promise(r => setTimeout(r, retryDelay));
        return fetchData(attempt + 1, throwOnFail);
      }
      reportTerminalError(err, isSchema);
      if (throwOnFail) throw err;
    } finally {
      loading.value = false;
    }
  };

  const refresh = async (): Promise<void> => {
    await fetchData(0, true);
  };

  const reset = (): void => {
    queryParams.value = { ...defaultParams };
    page.value = 1;
    pageSize.value = defaultPageSize;
  };

  const setQueryParam = (key: string, value: unknown): void => {
    queryParams.value = { ...queryParams.value, [key]: value };
  };

  // 监听分页变化自动加载（悬浮调用：throwOnFail=false，失败已在 reportTerminalError
  // 里经 logger+msg 显式上报，不再向上抛成未处理的 Promise rejection）
  const stopWatch = watch([page, pageSize], () => {
    void fetchData(0, false);
  });

  // 组件卸载时停止 watcher，避免内存泄漏
  onBeforeUnmount(() => {
    stopWatch();
  });

  // 初始加载（同为悬浮调用，失败上报走统一通道，不产生未处理 rejection）
  void fetchData(0, false);

  return {
    data,
    total,
    loading,
    page,
    pageSize,
    queryParams,
    refresh,
    reset,
    setQueryParam,
  };
}
