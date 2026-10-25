/**
 * 凭证新建提交在途锁行为单测（并发第二次提交必须被挡在请求之外）
 *
 * 功能：直接驱动 useVchrLst 的提交入口，验证第一次请求在途时第二次调用不发请求、
 *   返回 false，首次成功后锁释放。与 e2e 的 89-voucher-create-dedup 用例配对：
 *   e2e 证明真实按钮与请求计数，本用例在无浏览器条件下锁住同一并发语义，
 *   使"锁被摘掉"这类回归不依赖分片排队也能立刻红。
 * 调用方：vitest（tests/unit，CI ci-frontend-test job）。
 * 入参：仅 mock 凭证与科目 api、列表取数 composable、提示与日志通道，不发真实请求。
 * 传给谁：纯断言，无下游。
 * 存什么/存哪里：不落盘、不存储。
 */
import { describe, expect, it, vi, beforeEach } from 'vitest';

const mockCreateVoucher = vi.fn();
const mockUpdateVoucher = vi.fn();
const mockGetVoucher = vi.fn();
const mockGetVoucherTypes = vi.fn();
const mockGenerateVoucherNo = vi.fn();
const mockRefresh = vi.fn();

vi.mock('@/api/voucher', () => ({
  createVoucher: (...args: unknown[]) => mockCreateVoucher(...args),
  updateVoucher: (...args: unknown[]) => mockUpdateVoucher(...args),
  getVoucher: (...args: unknown[]) => mockGetVoucher(...args),
  getVoucherTypes: (...args: unknown[]) => mockGetVoucherTypes(...args),
  generateVoucherNo: (...args: unknown[]) => mockGenerateVoucherNo(...args),
}));

vi.mock('@/api/account-subject', () => ({
  getAccountSubjectTree: vi.fn(async () => ({ data: [] })),
}));

// 列表侧只用到 useTableApi 的返回形状，按 composable 的解构键给齐
vi.mock('@/composables/useTableApi', () => ({
  useTableApi: () => ({
    data: { value: [] },
    total: { value: 0 },
    loading: { value: false },
    page: { value: 1 },
    pageSize: { value: 20 },
    queryParams: { value: {} },
    refresh: (...args: unknown[]) => mockRefresh(...args),
  }),
}));

vi.mock('@/utils/message', () => ({
  msg: {
    warning: vi.fn(),
    success: vi.fn(),
    error: vi.fn(),
    translate: (key: string) => key,
  },
}));

vi.mock('@/utils/logger', () => ({
  logger: { error: vi.fn(), warn: vi.fn(), info: vi.fn(), debug: vi.fn() },
  logAuxLoadFailure: vi.fn(),
}));

const { useVchrLst } = await import('@/views/voucher/tabs/composables/useVchrLst');

/** 一张真实平衡的两行分录凭证：借 100 与贷 100，未平衡会在锁内早退 */
function fillBalancedVoucher(vchr: ReturnType<typeof useVchrLst>): void {
  vchr.form.voucher_no = 'JV-E2E-0001';
  vchr.form.voucher_date = '2026-08-19';
  vchr.form.voucher_type = 'journal';
  vchr.form.entries = [
    { account_subject_id: 1, debit_amount: 100, credit_amount: 0, description: '借方' },
    { account_subject_id: 2, debit_amount: 0, credit_amount: 100, description: '贷方' },
  ];
}

describe('凭证新建在途锁', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    // clearAllMocks 会连返回值一起清掉，列表重读必须仍可 await
    mockRefresh.mockResolvedValue(undefined);
  });

  it('首次请求在途时的第二次提交不发出请求且返回 false', async () => {
    let releaseFirst: ((value: unknown) => void) | undefined;
    mockCreateVoucher.mockImplementation(
      () =>
        new Promise(resolve => {
          releaseFirst = resolve;
        })
    );

    const vchr = useVchrLst();
    fillBalancedVoucher(vchr);

    const first = vchr.handleSubmit();
    expect(vchr.submitting, '提交入口应立刻置在途标记').toBe(true);

    const second = await vchr.handleSubmit();
    expect(second, '在途期间的第二次提交必须被拒绝').toBe(false);
    expect(mockCreateVoucher, '在途期间不得发出第二条建单请求').toHaveBeenCalledTimes(1);
    expect(mockUpdateVoucher, '新建分支不得走更新端点').not.toHaveBeenCalled();

    releaseFirst?.({ data: { id: 7 } });
    const ok = await first;
    expect(ok, '首次提交成功应回报 true 供调用方关窗').toBe(true);
    expect(vchr.submitting, '完成后必须释放锁').toBe(false);

    const payload = mockCreateVoucher.mock.calls[0][0] as Record<string, unknown>;
    expect(payload.voucher_type, '凭证类型必须是后端词表键').toBe('journal');
    expect(payload.items, '两行借贷分录必须齐备').toHaveLength(2);
  });

  it('第二次提交在首次完成后仍可正常发起', async () => {
    mockCreateVoucher.mockResolvedValue({ data: { id: 8 } });
    const vchr = useVchrLst();
    fillBalancedVoucher(vchr);

    expect(await vchr.handleSubmit()).toBe(true);
    expect(vchr.submitting).toBe(false);
    expect(await vchr.handleSubmit()).toBe(true);
    expect(mockCreateVoucher).toHaveBeenCalledTimes(2);
  });

  it('请求失败也必须释放锁，否则页面永久点不动', async () => {
    mockCreateVoucher.mockRejectedValue(new Error('后端拒绝'));
    const vchr = useVchrLst();
    fillBalancedVoucher(vchr);

    expect(await vchr.handleSubmit(), '失败应如实回报 false').toBe(false);
    expect(vchr.submitting, '失败后锁必须释放').toBe(false);
  });
});
