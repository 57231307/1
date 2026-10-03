/**
 * useTableApi 契约错误处理单元测试（响应形状漂移的取数行为）
 *
 * 覆盖调查结论 B/C 对应的行为：
 *  - detectList/detectTotal 命中「行数即总数」/正常分页分支；
 *  - 分页响应缺 total、无列表字段 → 抛 SchemaMismatchError；
 *  - 契约错误**不重试**（请求次数恒为 1），网络/5xx 瞬态错误**保持重试**；
 *  - 失败经 logger.error 显式留痕；契约错误无 onError 时经 msg 弹一次，网络错误不重复弹。
 *
 * 本地按任务允许 `npx vitest run`；仓库既有约定「视图级覆盖由 CI e2e 承担」，此处仅测非视图 TS。
 */
import { describe, expect, it, vi, beforeEach, afterEach } from 'vitest';
import { mount, flushPromises } from '@vue/test-utils';
import { defineComponent } from 'vue';

// 模块级 mock：request.get 由每个用例按需 stub
const mockRequestGet = vi.fn();
vi.mock('@/api/request', () => ({
  request: { get: (...args: unknown[]) => mockRequestGet(...args) },
}));

// 观测 logger / msg 两条上报通道
const mockLoggerError = vi.fn();
vi.mock('@/utils/logger', () => ({
  logger: {
    error: (...args: unknown[]) => mockLoggerError(...args),
    warn: vi.fn(),
    info: vi.fn(),
    debug: vi.fn(),
  },
  logAuxLoadFailure: vi.fn(),
  default: { error: (...args: unknown[]) => mockLoggerError(...args) },
}));

const mockMsgError = vi.fn();
vi.mock('@/utils/message', () => ({
  msg: { error: (...args: unknown[]) => mockMsgError(...args) },
  default: { error: (...args: unknown[]) => mockMsgError(...args) },
}));

import { useTableApi, SchemaMismatchError, isSchemaMismatchError } from '@/composables/useTableApi';
import type { UseTableApiOptions } from '@/composables/useTableApi';

/**
 * 在一个最小宿主组件里实例化 useTableApi（提供 onBeforeUnmount 生命周期），
 * 返回暴露给断言的 api 句柄。
 */
function mountTable(options: UseTableApiOptions | string) {
  let api: ReturnType<typeof useTableApi> | undefined;
  const wrapper = mount(
    defineComponent({
      setup() {
        api = useTableApi(options);
        return () => null;
      },
    })
  );
  return { wrapper, api: api! };
}

beforeEach(() => {
  mockRequestGet.mockReset();
  mockLoggerError.mockReset();
  mockMsgError.mockReset();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('useTableApi · 取数分支（正常/行数即总数）', () => {
  it('① 裸数组端点：total = 行数（86954da7 行为，不回归 0）', async () => {
    // 后端 ApiResponse::success(Vec<T>) → {code, data:[...]}
    mockRequestGet.mockResolvedValue({ code: 200, data: [{ id: 1 }, { id: 2 }, { id: 3 }] });
    const { api } = mountTable({ url: '/any-bare-array' });
    await flushPromises();
    expect(api.data.value).toHaveLength(3);
    expect(api.total.value).toBe(3);
  });

  it('② {code,data:[...],total:N} 顶层 total 优先：total = N（非行数）', async () => {
    mockRequestGet.mockResolvedValue({ code: 200, data: [{ id: 1 }, { id: 2 }], total: 50 });
    const { api } = mountTable({ url: '/api-gateway-like' });
    await flushPromises();
    expect(api.data.value).toHaveLength(2);
    expect(api.total.value).toBe(50);
  });

  it('③ {items,total} 分页端点：列表取 items，total 取 total', async () => {
    mockRequestGet.mockResolvedValue({
      code: 200,
      data: { items: [{ id: 1 }, { id: 2 }, { id: 3 }], total: 7, page: 1, page_size: 3 },
    });
    const { api } = mountTable({ url: '/paginated-items', listKey: 'items' });
    await flushPromises();
    expect(api.data.value).toHaveLength(3);
    expect(api.total.value).toBe(7);
  });
});

describe('useTableApi · 契约错误抛错且不重试', () => {
  it('④ 分页响应缺 total（{items} 无 total）→ 抛 SchemaMismatchError 且请求只发 1 次', async () => {
    // retryCount 默认 2；契约错误必须绕过重试
    mockRequestGet.mockResolvedValue({ code: 200, data: { items: [{ id: 1 }] } });
    const { api } = mountTable({ url: '/missing-total', listKey: 'items' });
    // mount 里的初始悬浮加载先跑完（它同样命中契约错误），之后只统计 refresh 这一条链的请求次数
    await flushPromises();
    mockRequestGet.mockClear();
    // refresh() 以 throwOnFail=true 调用，能把抛错透出
    await expect(api.refresh()).rejects.toBeInstanceOf(SchemaMismatchError);
    expect(isSchemaMismatchError(new SchemaMismatchError('total', '/x', 'e'))).toBe(true);
    expect(mockRequestGet).toHaveBeenCalledTimes(1); // 未重试
    // 失败可见：logger.error 落痕；无 onError ⇒ 契约错误经 msg 弹一次
    expect(mockLoggerError).toHaveBeenCalled();
    expect(mockMsgError).toHaveBeenCalledWith('loadFailed');
  });

  it('④b 同一表格的重复契约错误在去重窗口内只弹一条 toast（不刷屏）', async () => {
    mockRequestGet.mockResolvedValue({ code: 200, data: { items: [{ id: 1 }] } });
    const { api } = mountTable({ url: '/dedup', listKey: 'items' });
    await flushPromises(); // 初始加载弹第 1 条
    await api.refresh().catch(() => undefined); // 同文案，窗口内应被去重
    await api.refresh().catch(() => undefined);
    expect(mockMsgError).toHaveBeenCalledTimes(1);
    // 但留痕不丢：每次失败都进 logger
    expect(mockLoggerError).toHaveBeenCalledTimes(3);
  });

  it('⑤ 响应无可识别列表字段 → 抛 SchemaMismatchError(kind=list) 且不重试', async () => {
    mockRequestGet.mockResolvedValue({ code: 200, data: { foo: [{ id: 1 }] } });
    const { api } = mountTable({ url: '/unknown-shape' });
    await flushPromises();
    mockRequestGet.mockClear();
    const err = await api.refresh().catch(e => e);
    expect(err).toBeInstanceOf(SchemaMismatchError);
    expect((err as SchemaMismatchError).kind).toBe('list');
    expect(mockRequestGet).toHaveBeenCalledTimes(1);
  });

  it('初始悬浮加载：契约错误不向上抛成未处理 rejection，但已在通道留痕', async () => {
    mockRequestGet.mockResolvedValue({ code: 200, data: { items: [{ id: 1 }] } });
    // 不 await refresh()：仅触发 setup 里的悬浮 fetchData(0,false)。
    // 若实现错误地对悬浮调用 rethrow，vitest 会以未处理 rejection 判失败。
    mountTable({ url: '/floating', listKey: 'items' });
    await flushPromises();
    await flushPromises();
    // 悬浮加载只发 1 次（契约错误不重试）
    expect(mockRequestGet).toHaveBeenCalledTimes(1);
    expect(mockLoggerError).toHaveBeenCalled();
    expect(mockMsgError).toHaveBeenCalledWith('loadFailed');
  });
});

describe('useTableApi · 网络/5xx 瞬态错误仍保持重试', () => {
  it('request.get 抛非契约错误 → 按 retryCount 重试后终端抛错，且不重复弹 msg（交给 axios）', async () => {
    vi.useFakeTimers();
    mockRequestGet.mockRejectedValue(new Error('Network Error'));
    const { api } = mountTable({ url: '/net-flaky', retryCount: 2, retryDelay: 1000 });
    // 先让 mount 里的初始悬浮链跑完（1 首次 + 2 重试），再单独计量 refresh 链
    await vi.runAllTimersAsync();
    mockRequestGet.mockClear();
    mockLoggerError.mockClear();
    // 先把断言挂上再推进定时器，避免 refresh 链的 rejection 短暂无人处理
    const assertion = expect(api.refresh()).rejects.toThrow('Network Error');
    await vi.runAllTimersAsync();
    await assertion;
    // 首次 + 2 次重试 = 3 次请求
    expect(mockRequestGet).toHaveBeenCalledTimes(3);
    // 终端留痕走 logger，但网络错误不在本 composable 重复弹 msg（axios 拦截器已负责）
    expect(mockLoggerError).toHaveBeenCalled();
    expect(mockMsgError).not.toHaveBeenCalled();
  });
});
