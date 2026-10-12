/**
 * dialog-dismissal.test.ts — ElMessageBox 拒绝值判据的单源检测锁。
 *
 * 功能：锁定 utils/monitor.ts 的 isDialogDismissal / rethrowNonDismissal 行为契约：
 * Element Plus MessageBox（confirm/prompt/alert）在用户放弃时只以字符串 'cancel'
 * （取消按钮、无 distinguishCancelAndClose 时的 X/Esc/遮罩）或 'close'
 * （distinguishCancelAndClose=true 时 X/Esc/遮罩）reject（见 element-plus
 * es/components/message-box/src/messageBox.mjs 的 currentMsg.reject 分支），
 * 二者都必须被判为「用户主动关闭」；真实 Error 或其它 rejection 形态不得被误判，
 * 否则确认框异常会被冒充成用户取消而静默吞掉。
 * 调用方：vitest run（tests/unit）。
 * 入参：无（自包含用例）。
 * 传给谁：expect 断言。
 * 存什么：判据集合 { 'cancel', 'close' } 与 rethrow 的留痕+外显+原样上抛语义。
 * 存哪里：本文件；判据实现单源在 src/utils/monitor.ts，禁止站点内另写私有副本。
 */
import { describe, it, expect, vi, beforeEach } from 'vitest';

// vi.mock 工厂会被提升到文件顶部，工厂内引用的桩必须是 hoisted 绑定，否则取到未初始化的常量。
const { loggerError, operationFail } = vi.hoisted(() => ({
  loggerError: vi.fn(),
  operationFail: vi.fn(),
}));

vi.mock('@/utils/logger', () => ({
  logger: { error: loggerError, debug: vi.fn(), info: vi.fn(), warn: vi.fn() },
}));
vi.mock('@/utils/message', () => ({
  msg: { operationFail: operationFail, success: vi.fn(), error: vi.fn() },
}));

import { isDialogDismissal, rethrowNonDismissal } from '@/utils/monitor';

describe('isDialogDismissal（MessageBox 拒绝值判据，单源）', () => {
  it('识别取消按钮拒绝值 cancel', () => {
    expect(isDialogDismissal('cancel')).toBe(true);
  });

  it('识别 X/Esc/遮罩关闭拒绝值 close（distinguishCancelAndClose 模式）', () => {
    expect(isDialogDismissal('close')).toBe(true);
  });

  it('其它值不是用户放弃：confirm 动作、Error、对象、undefined、大小写变体', () => {
    expect(isDialogDismissal('confirm')).toBe(false);
    expect(isDialogDismissal(new Error('boom'))).toBe(false);
    expect(isDialogDismissal({ message: 'cancel' })).toBe(false);
    expect(isDialogDismissal(undefined)).toBe(false);
    expect(isDialogDismissal(null)).toBe(false);
    expect(isDialogDismissal('Cancel')).toBe(false);
    expect(isDialogDismissal('canceled')).toBe(false);
  });
});

describe('rethrowNonDismissal（非放弃异常统一收口）', () => {
  beforeEach(() => {
    loggerError.mockClear();
    operationFail.mockClear();
  });

  it('留痕（logger.error 带站点名）+ 外显（msg.operationFail）+ 原样上抛同一对象', () => {
    const err = new Error('dialog render failed');
    expect(() => rethrowNonDismissal('test.site', err)).toThrow(err);
    expect(loggerError).toHaveBeenCalledTimes(1);
    expect(loggerError.mock.calls[0][0]).toContain('test.site');
    expect(operationFail).toHaveBeenCalledTimes(1);
  });

  it('字符串 rejection（非 cancel/close）同样上抛，不冒充取消', () => {
    expect(() => rethrowNonDismissal('test.site', 'closeAll')).toThrow('closeAll');
  });
});
