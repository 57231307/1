/**
 * android-print-bridge.test.ts — 安卓打印前端出口契约。
 *
 * 功能 锁定壳内两条打印出口：蓝牙小票每次先确认权限并把打印机地址下发，
 *      系统打印把整页 HTML 或当前页面交给壳侧命令，非安卓环境一律不触发命令。
 * 调用方 vitest run（tests/unit）。
 * 入参 无（自包含用例）。
 * 传给谁 expect 断言。
 * 存什么 只在内存记录命令名与参数。
 * 存哪里 本文件。
 */
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

const { warnSpy, successSpy, translateSpy, errorSpy } = vi.hoisted(() => ({
  warnSpy: vi.fn(),
  successSpy: vi.fn(),
  translateSpy: vi.fn((key: string) => key),
  errorSpy: vi.fn(),
}));

vi.mock('@/utils/message', () => ({
  msg: { warning: warnSpy, success: successSpy, error: errorSpy, translate: translateSpy },
}));

import {
  documentPrintRoute,
  labelPrintRoute,
  printLabelToBluetooth,
  printReceiptToBluetooth,
  savePrinterPreference,
  savePrintRoute,
} from '@/utils/bluetooth-print';
import { printCurrentPageOnAndroid } from '@/utils/system-print';
import { printTableOnAndroid } from '@/utils/print';
import {
  LABEL_FIELD_KEYS,
  buildPageSizeCss,
  lineWidthForPaperWidth,
  loadDocumentPaper,
  loadLabelPaper,
  saveDocumentPaper,
  saveLabelPaper,
  type LabelFieldKey,
} from '@/utils/print-settings';
import { buildPieceLabelFields, buildPieceLabelHtml } from '@/utils/label-print';

const ANDROID_UA =
  'Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/122.0.0.0 Mobile Safari/537.36';
const DESKTOP_UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36';

interface BridgeCall {
  command: string;
  args?: Record<string, unknown>;
}

function setUserAgent(value: string): void {
  Object.defineProperty(window.navigator, 'userAgent', { configurable: true, value });
}

function stubBridge(
  impl?: (command: string, args?: Record<string, unknown>) => unknown
): BridgeCall[] {
  const calls: BridgeCall[] = [];
  Object.defineProperty(window, '__TAURI__', {
    configurable: true,
    value: {
      core: {
        invoke(command: string, args?: Record<string, unknown>): Promise<unknown> {
          calls.push({ command, args });
          return Promise.resolve(impl ? impl(command, args) : {});
        },
      },
    },
  });
  return calls;
}

beforeEach(() => {
  localStorage.clear();
  setUserAgent(ANDROID_UA);
});

afterEach(() => {
  delete (window as { __TAURI__?: unknown }).__TAURI__;
  vi.clearAllMocks();
});

describe('蓝牙小票打印', () => {
  it('先申请权限，再把打印机地址与排好的小票一起下发', async () => {
    savePrinterPreference({ name: 'e2e 热敏打印机', address: 'AA:BB:CC:DD:EE:FF' });
    const calls = stubBridge(command =>
      command === 'bluetooth_request_permission' ? { granted: true, sdkInt: 35 } : { bytes: 12 }
    );

    const printed = await printReceiptToBluetooth({
      title: '库存台账',
      meta: ['打印人 张三'],
      columns: [{ title: '名称' }],
      rows: [['纱线']],
      footer: [],
    });

    expect(printed).toBe(true);
    expect(calls.map(call => call.command)).toEqual([
      'bluetooth_request_permission',
      'bluetooth_print_text',
    ]);
    expect(calls[1].args?.address).toBe('AA:BB:CC:DD:EE:FF');
    expect(String(calls[1].args?.text)).toContain('纱线');
    expect(successSpy).toHaveBeenCalledWith('printBluetoothSent');
  });

  it('权限被拒时不发打印命令，只给可见提示', async () => {
    savePrinterPreference({ name: 'e2e 热敏打印机', address: 'AA:BB:CC:DD:EE:FF' });
    const calls = stubBridge(() => ({ granted: false, sdkInt: 35 }));

    const printed = await printReceiptToBluetooth({
      title: '库存台账',
      meta: [],
      columns: [{ title: '名称' }],
      rows: [['纱线']],
      footer: [],
    });

    expect(printed).toBe(false);
    expect(calls.map(call => call.command)).toEqual(['bluetooth_request_permission']);
    expect(warnSpy).toHaveBeenCalledWith('printBluetoothPermissionDenied');
  });

  it('未选择打印机时只提示，不触碰壳侧命令', async () => {
    const calls = stubBridge();

    const printed = await printReceiptToBluetooth({
      title: '库存台账',
      meta: [],
      columns: [],
      rows: [],
      footer: [],
    });

    expect(printed).toBe(false);
    expect(calls).toHaveLength(0);
    expect(warnSpy).toHaveBeenCalledWith('printBluetoothPrinterNotSelected');
  });
});

describe('系统打印出口', () => {
  it('非安卓环境不调用系统打印命令', async () => {
    setUserAgent(DESKTOP_UA);
    const calls = stubBridge();

    const handled = await printTableOnAndroid({
      title: '库存台账',
      rows: [{ a: 1 }],
      columns: ['a'],
    });

    expect(handled).toBe(false);
    expect(calls).toHaveLength(0);
  });

  it('安卓环境把表格 HTML 交给系统打印，标题与单元格都做转义', async () => {
    const calls = stubBridge(() => ({ jobName: '库存<台账>' }));

    const handled = await printTableOnAndroid({
      title: '库存<台账>',
      rows: [{ 名称: '<script>alert(1)</script>' }],
      columns: [{ field: '名称', displayName: '名称' }],
    });

    expect(handled).toBe(true);
    expect(calls).toHaveLength(1);
    expect(calls[0].command).toBe('system_print_html');
    const html = String(calls[0].args?.html);
    expect(html).toContain('库存&lt;台账&gt;');
    expect(html).toContain('&lt;script&gt;alert(1)&lt;/script&gt;');
    expect(calls[0].args?.jobName).toBe('库存<台账>');
    expect(successSpy).toHaveBeenCalledWith('printSystemOpened');
  });

  it('无数据时只提示，不打开系统打印', async () => {
    const calls = stubBridge();

    const handled = await printTableOnAndroid({ title: '库存台账', rows: [], columns: ['a'] });

    expect(handled).toBe(true);
    expect(calls).toHaveLength(0);
    expect(warnSpy).toHaveBeenCalledWith('noDataToPrint');
  });

  it('当前页面打印走 system_print_page', async () => {
    const calls = stubBridge(() => ({ jobName: '财务报表' }));

    const handled = await printCurrentPageOnAndroid('财务报表');

    expect(handled).toBe(true);
    expect(calls).toEqual([{ command: 'system_print_page', args: { jobName: '财务报表' } }]);
  });
});

describe('按单据与标签选择打印机', () => {
  it('单据分配了打印机时优先于默认打印机', async () => {
    savePrinterPreference({ name: '默认打印机', address: 'AA:00:00:00:00:01' });
    savePrintRoute(documentPrintRoute('purchase_order'), {
      name: '采购打印机',
      address: 'AA:00:00:00:00:02',
    });
    const calls = stubBridge(command =>
      command === 'bluetooth_request_permission' ? { granted: true, sdkInt: 35 } : { bytes: 8 }
    );

    const handled = await printTableOnAndroid({
      title: '采购订单',
      rows: [{ no: 'PO-1' }],
      columns: [{ field: 'no', displayName: '单号' }],
      documentKey: 'purchase_order',
    });

    expect(handled).toBe(true);
    expect(calls.map(call => call.command)).toEqual([
      'bluetooth_request_permission',
      'bluetooth_print_text',
    ]);
    expect(calls[1].args?.address).toBe('AA:00:00:00:00:02');
    expect(String(calls[1].args?.text)).toContain('PO-1');
  });

  it('未分配打印机的单据跟随默认打印机', async () => {
    savePrinterPreference({ name: '默认打印机', address: 'AA:00:00:00:00:01' });
    const calls = stubBridge(command =>
      command === 'bluetooth_request_permission' ? { granted: true, sdkInt: 35 } : { bytes: 8 }
    );

    const handled = await printTableOnAndroid({
      title: '会计凭证',
      rows: [{ no: 'V-1' }],
      columns: ['no'],
      documentKey: 'voucher',
    });

    expect(handled).toBe(true);
    expect(calls[1].args?.address).toBe('AA:00:00:00:00:01');
  });

  it('标签打印机独立于默认打印机', async () => {
    savePrinterPreference({ name: '默认打印机', address: 'AA:00:00:00:00:01' });
    const target = labelPrintRoute('fabric_piece');
    savePrintRoute(target, { name: '标签打印机', address: 'AA:00:00:00:00:03' });
    const calls = stubBridge(command =>
      command === 'bluetooth_request_permission' ? { granted: true, sdkInt: 35 } : { bytes: 8 }
    );

    const printed = await printLabelToBluetooth('成品布标签', [{ label: '匹号', value: 'P-1' }], {
      target,
    });

    expect(printed).toBe(true);
    expect(calls[1].args?.address).toBe('AA:00:00:00:00:03');
    expect(String(calls[1].args?.text)).toContain('P-1');
    expect(successSpy).toHaveBeenCalledWith('printBluetoothSent');
  });
});

describe('打印纸张与标签字段', () => {
  it('默认单据为 A4，标签为 40x30 且字段全选', () => {
    expect(loadDocumentPaper()).toEqual({ widthMm: 210, heightMm: 297, orientation: 'portrait' });
    const label = loadLabelPaper();
    expect(label.widthMm).toBe(40);
    expect(label.heightMm).toBe(30);
    expect(label.fields).toEqual([...LABEL_FIELD_KEYS]);
  });

  it('保存后按原值读取，标签只保留已知字段', () => {
    saveDocumentPaper({ widthMm: 58, heightMm: 0, orientation: 'landscape' });
    expect(loadDocumentPaper()).toEqual({ widthMm: 58, heightMm: 0, orientation: 'landscape' });
    saveLabelPaper({
      widthMm: 50,
      heightMm: 40,
      orientation: 'portrait',
      fields: ['piece_no', 'color_no'],
    });
    expect(loadLabelPaper().fields).toEqual(['piece_no', 'color_no']);
  });

  it('行宽按纸宽换算，58 与 80 保持原基准', () => {
    expect(lineWidthForPaperWidth(58)).toBe(32);
    expect(lineWidthForPaperWidth(80)).toBe(48);
    expect(lineWidthForPaperWidth(40)).toBe(22);
  });

  it('页面尺寸横向交换宽高，连续纸只声明宽度', () => {
    expect(buildPageSizeCss({ widthMm: 58, heightMm: 0, orientation: 'portrait' })).toBe(
      '58mm auto'
    );
    expect(buildPageSizeCss({ widthMm: 210, heightMm: 297, orientation: 'portrait' })).toBe(
      '210mm 297mm'
    );
    expect(buildPageSizeCss({ widthMm: 210, heightMm: 297, orientation: 'landscape' })).toBe(
      '297mm 210mm'
    );
  });

  it('单据纸张设置写入系统打印页面', async () => {
    saveDocumentPaper({ widthMm: 58, heightMm: 0, orientation: 'portrait' });
    const calls = stubBridge(() => ({ jobName: '库存' }));

    await printTableOnAndroid({ title: '库存', rows: [{ a: 1 }], columns: ['a'] });

    const html = String(calls[0].args?.html);
    expect(html).toContain('size: 58mm auto');
    expect(html).toContain('margin: 2mm');
  });

  it('标签字段按设置过滤并生成标签页面尺寸', () => {
    const labels: Record<LabelFieldKey, string> = {
      piece_no: '匹号',
      dye_lot_no: '缸号',
      color_no: '色号',
      length: '米数',
      weight: '重量',
      width: '幅宽',
      gram_weight: '克重',
      barcode: '条码',
      warehouse_in_at: '入库时间',
    };
    const fields = buildPieceLabelFields(
      {
        piece_no: 'P-1',
        dye_lot_no: 'L-1',
        color_no: null,
        length: '12.5',
        weight: null,
        width: null,
        gram_weight: null,
        barcode: null,
        warehouse_in_at: null,
      },
      labels,
      ['piece_no', 'color_no']
    );

    expect(fields).toEqual([
      { label: '匹号', value: 'P-1' },
      { label: '色号', value: '-' },
    ]);
    const html = buildPieceLabelHtml('成品布标签', fields, {
      widthMm: 40,
      heightMm: 30,
      orientation: 'landscape',
      fields: [],
    });
    expect(html).toContain('size: 30mm 40mm');
    expect(html).toContain('P-1');
  });
});
