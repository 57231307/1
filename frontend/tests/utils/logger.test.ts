/**
 * logAuxLoadFailure 分级单元测试（锁死 403 降 warn、其余保 error 的判据方向）。
 *
 * 断言点为 logger 单例的 warn/error 被调与否，直接反映分级，不经 console，
 * 因而不受 import.meta.env.DEV 对 warn 的门控影响。调用方为 useTableApi 及各视图页
 * 的辅助与主数据加载 catch，用于把后端正确拒绝的权限 403 从应用错误信号里摘出。
 */
import { describe, expect, it, vi, afterEach } from 'vitest';
import { logAuxLoadFailure, logger } from '@/utils/logger';

function axiosErrWithStatus(status: number | undefined): unknown {
  return { response: status === undefined ? undefined : { status } };
}

describe('logAuxLoadFailure · 按 response.status 分级', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('403 属权限下界正常生效，降为 warn 且不产 error', () => {
    const warn = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const error = vi.spyOn(logger, 'error').mockImplementation(() => {});
    logAuxLoadFailure('列表取数失败', axiosErrWithStatus(403));
    expect(warn).toHaveBeenCalledWith(
      '列表取数失败',
      expect.objectContaining({ response: { status: 403 } })
    );
    expect(error).not.toHaveBeenCalled();
  });

  it('5xx 是真实缺陷，保持 error 不降级', () => {
    const warn = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const error = vi.spyOn(logger, 'error').mockImplementation(() => {});
    logAuxLoadFailure('列表取数失败', axiosErrWithStatus(500));
    expect(error).toHaveBeenCalled();
    expect(warn).not.toHaveBeenCalled();
  });

  it('无 response 的裸 Error（契约漂移或网络错误）保持 error', () => {
    const warn = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const error = vi.spyOn(logger, 'error').mockImplementation(() => {});
    logAuxLoadFailure('列表取数失败', new Error('Network Error'));
    expect(error).toHaveBeenCalled();
    expect(warn).not.toHaveBeenCalled();
  });
});
