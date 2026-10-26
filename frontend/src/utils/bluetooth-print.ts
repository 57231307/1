/**
 * 安卓客户端蓝牙热敏打印桥。
 *
 * 调用方 全局打印工具在安卓壳内改走本模块，打印目标页负责打印机选择与连接。
 * 入参 小票标题、元信息、列定义与已格式化的数据行，可按单据或标签带打印机路由键。
 * 传给谁 经 window.__TAURI__ 调壳内 Rust 命令，最终由 Kotlin 插件写 SPP 蓝牙 socket。
 * 存什么 默认打印机、按单据与标签分配的打印机名称地址，以及纸张宽度。
 * 存哪里 浏览器 localStorage。
 */
import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { lineWidthForPaperWidth } from '@/utils/print-settings';

export interface BluetoothPrinter {
  name: string;
  address: string;
}

export interface BluetoothPermissionResult {
  granted: boolean;
  sdkInt: number;
}

export interface BluetoothAdapterState {
  sdkInt: number;
  granted: boolean;
  supported: boolean;
  enabled: boolean;
  connectedAddress?: string | null;
  printers: BluetoothPrinter[];
}

export interface BluetoothConnection {
  address: string;
  name: string;
}

export interface BluetoothPrintResult {
  bytes: number;
}

export type ReceiptPaperWidth = '58' | '80';
export type ReceiptAlign = 'left' | 'center' | 'right';

export interface ReceiptColumn {
  title: string;
  align?: ReceiptAlign;
}

export interface ReceiptContent {
  title: string;
  meta: string[];
  columns: ReceiptColumn[];
  rows: string[][];
  footer: string[];
}

export interface ReceiptPrintOptions {
  paperWidth?: ReceiptPaperWidth;
  /** 自定义纸张宽度，毫米，优先于 58 与 80 两档预设 */
  paperWidthMm?: number;
  cut?: boolean;
  feedLines?: number;
  target?: PrintRouteKey;
  /** 成功提示的 message 命名空间键，缺省为小票文案 */
  successMessage?: string;
}

/** 支持按单据选择打印机的打印入口。 */
export type DocumentPrintType =
  | 'purchase_order'
  | 'voucher'
  | 'inventory_stock'
  | 'ap_invoice'
  | 'ar_invoice'
  | 'finance_subject'
  | 'customer'
  | 'supplier'
  | 'warehouse';

/** 支持按标签选择打印机的标签类型。 */
export type LabelPrintType = 'fabric_piece';

/** 打印机路由键，doc 前缀为单据，label 前缀为标签。 */
export type PrintRouteKey = `doc:${DocumentPrintType}` | `label:${LabelPrintType}`;

/** 单据与标签到打印机的对照表，未出现的键沿用默认打印机。 */
export type PrintRouteTable = Partial<Record<PrintRouteKey, BluetoothPrinter>>;

/** 打印目标页与各单据入口共用的单据类型清单。 */
export const DOCUMENT_PRINT_TYPES: readonly DocumentPrintType[] = [
  'purchase_order',
  'voucher',
  'inventory_stock',
  'ap_invoice',
  'ar_invoice',
  'finance_subject',
  'customer',
  'supplier',
  'warehouse',
];

/** 标签类型清单，路由键与标签打印入口共用。 */
export const LABEL_PRINT_TYPES: readonly LabelPrintType[] = ['fabric_piece'];

/** 单据类型换成路由键，供读取与保存打印机分配。 */
export function documentPrintRoute(type: DocumentPrintType): PrintRouteKey {
  return `doc:${type}`;
}

/** 标签类型换成路由键，与单据键共用一个存储表。 */
export function labelPrintRoute(type: LabelPrintType): PrintRouteKey {
  return `label:${type}`;
}

const PRINTER_STORAGE_KEY = 'bingxi.bluetooth.printer';
const ROUTES_STORAGE_KEY = 'bingxi.bluetooth.routes';
const PAPER_STORAGE_KEY = 'bingxi.bluetooth.paper';
const LINE_WIDTH: Record<ReceiptPaperWidth, number> = { '58': 32, '80': 48 };
const MIN_COLUMN_WIDTH = 6;

/** 壳内是否注入 Tauri 全局桥，且运行在安卓 WebView 里。 */
export function isAndroidClient(): boolean {
  if (typeof window === 'undefined') return false;
  const bridge = window.__TAURI__;
  if (!bridge || typeof (bridge.core?.invoke ?? bridge.invoke) !== 'function') return false;
  return /Android/i.test(window.navigator.userAgent);
}

/** 供系统打印等其它壳内桥共用同一套 Tauri 注入形态识别。 */
export function invokeBridge<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const bridge = typeof window === 'undefined' ? undefined : window.__TAURI__;
  if (bridge?.core?.invoke) {
    return bridge.core.invoke<T>(command, args);
  }
  if (bridge?.invoke) {
    return bridge.invoke<T>(command, args) as Promise<T>;
  }
  return Promise.reject(new Error('当前环境没有可用的原生打印桥'));
}

export function requestBluetoothPermission(): Promise<BluetoothPermissionResult> {
  return invokeBridge<BluetoothPermissionResult>('bluetooth_request_permission');
}

export function openBluetoothSettings(): Promise<void> {
  return invokeBridge<void>('bluetooth_open_settings');
}

export function listBluetoothPrinters(): Promise<BluetoothAdapterState> {
  return invokeBridge<BluetoothAdapterState>('bluetooth_list_printers');
}

export function connectBluetoothPrinter(address: string): Promise<BluetoothConnection> {
  return invokeBridge<BluetoothConnection>('bluetooth_connect', { address });
}

export function disconnectBluetoothPrinter(): Promise<void> {
  return invokeBridge<void>('bluetooth_disconnect');
}

export function sendBluetoothText(
  text: string,
  options: { cut?: boolean; feedLines?: number; address?: string } = {}
): Promise<BluetoothPrintResult> {
  return invokeBridge<BluetoothPrintResult>('bluetooth_print_text', {
    text,
    cut: options.cut,
    feedLines: options.feedLines,
    address: options.address,
  });
}

export function loadPrinterPreference(): BluetoothPrinter | null {
  try {
    const raw = window.localStorage.getItem(PRINTER_STORAGE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<BluetoothPrinter>;
    if (!parsed?.address) return null;
    return { name: parsed.name || parsed.address, address: parsed.address };
  } catch {
    return null;
  }
}

export function savePrinterPreference(printer: BluetoothPrinter | null): void {
  try {
    if (!printer) {
      window.localStorage.removeItem(PRINTER_STORAGE_KEY);
      return;
    }
    window.localStorage.setItem(
      PRINTER_STORAGE_KEY,
      JSON.stringify({ name: printer.name || printer.address, address: printer.address })
    );
  } catch {
    // 隐私模式下存储不可用，本次连接仍然有效，仅重启后需重新选择
  }
}

/** 读取全部打印机分配，损坏条目直接跳过。 */
export function loadPrintRoutes(): PrintRouteTable {
  try {
    const raw = window.localStorage.getItem(ROUTES_STORAGE_KEY);
    if (!raw) return {};
    const parsed = JSON.parse(raw) as Record<string, Partial<BluetoothPrinter>>;
    const routes: PrintRouteTable = {};
    for (const [key, value] of Object.entries(parsed ?? {})) {
      if (!value?.address) continue;
      routes[key as PrintRouteKey] = {
        name: value.name || value.address,
        address: value.address,
      };
    }
    return routes;
  } catch {
    return {};
  }
}

/** 保存一个单据或标签的打印机，传空表示取消分配并回退默认。 */
export function savePrintRoute(key: PrintRouteKey, printer: BluetoothPrinter | null): void {
  try {
    const routes = loadPrintRoutes();
    if (printer) {
      routes[key] = { name: printer.name || printer.address, address: printer.address };
    } else {
      delete routes[key];
    }
    window.localStorage.setItem(ROUTES_STORAGE_KEY, JSON.stringify(routes));
  } catch {
    // 存储不可用时仅本次分配不落盘，后续打印仍按默认打印机发送
  }
}

/** 按单据或标签取打印机，该类型未单独分配时回退默认打印机。 */
export function resolvePrinterForTarget(target?: PrintRouteKey): BluetoothPrinter | null {
  if (target) {
    const routed = loadPrintRoutes()[target];
    if (routed) return routed;
  }
  return loadPrinterPreference();
}

export function loadPaperWidth(): ReceiptPaperWidth {
  try {
    const stored = window.localStorage.getItem(PAPER_STORAGE_KEY);
    return stored === '80' ? '80' : '58';
  } catch {
    return '58';
  }
}

export function savePaperWidth(width: ReceiptPaperWidth): void {
  try {
    window.localStorage.setItem(PAPER_STORAGE_KEY, width);
  } catch {
    // 存储不可用时仅影响下次默认纸张，本次打印继续
  }
}

/** 半角字符计 1 宽，中文与全角符号计 2 宽。 */
function charWidth(char: string): number {
  const code = char.codePointAt(0) ?? 0;
  return code > 0x7f ? 2 : 1;
}

export function displayWidth(text: string): number {
  let width = 0;
  for (const char of Array.from(text)) width += charWidth(char);
  return width;
}

export function wrapDisplay(text: string, width: number): string[] {
  if (width <= 0) return [text];
  const lines: string[] = [];
  let current = '';
  let currentWidth = 0;
  for (const char of Array.from(text)) {
    const w = charWidth(char);
    if (currentWidth + w > width && current) {
      lines.push(current);
      current = '';
      currentWidth = 0;
    }
    current += char;
    currentWidth += w;
  }
  if (current || lines.length === 0) lines.push(current);
  return lines;
}

export function truncateDisplay(text: string, width: number): string {
  if (displayWidth(text) <= width) return text;
  if (width <= 2) return '';
  const limit = width - 2;
  let result = '';
  let used = 0;
  for (const char of Array.from(text)) {
    const w = charWidth(char);
    if (used + w > limit) break;
    result += char;
    used += w;
  }
  return `${result}..`;
}

export function padDisplay(text: string, width: number, align: ReceiptAlign = 'left'): string {
  const clipped = truncateDisplay(text, width);
  const padding = width - displayWidth(clipped);
  if (padding <= 0) return clipped;
  if (align === 'right') return `${' '.repeat(padding)}${clipped}`;
  if (align === 'center') {
    const left = Math.floor(padding / 2);
    return `${' '.repeat(left)}${clipped}${' '.repeat(padding - left)}`;
  }
  return `${clipped}${' '.repeat(padding)}`;
}

function maxTableColumns(lineWidth: number): number {
  return Math.max(1, Math.floor((lineWidth + 1) / (MIN_COLUMN_WIDTH + 1)));
}

function allocateWidths(columns: ReceiptColumn[], rows: string[][], budget: number): number[] {
  const ideal = columns.map((column, index) => {
    let width = displayWidth(column.title);
    for (const row of rows.slice(0, 100)) {
      width = Math.max(width, displayWidth(row[index] ?? ''));
    }
    return width;
  });
  const total = ideal.reduce((sum, width) => sum + width, 0);
  if (total <= budget) {
    const widths = [...ideal];
    let spare = budget - total;
    let index = 0;
    while (spare > 0) {
      widths[index % widths.length] += 1;
      spare -= 1;
      index += 1;
    }
    return widths;
  }
  const widths = ideal.map(width =>
    Math.max(MIN_COLUMN_WIDTH, Math.floor((width * budget) / total))
  );
  let over = widths.reduce((sum, width) => sum + width, 0) - budget;
  while (over > 0) {
    const widest = widths.indexOf(Math.max(...widths));
    if (widths[widest] <= MIN_COLUMN_WIDTH) break;
    widths[widest] -= 1;
    over -= 1;
  }
  let index = 0;
  while (over < 0) {
    widths[index % widths.length] += 1;
    over += 1;
    index += 1;
  }
  return widths;
}

function formatTableRow(cells: string[], widths: number[], aligns: ReceiptAlign[]): string {
  return cells.map((cell, index) => padDisplay(cell, widths[index], aligns[index])).join(' ');
}

function formatRecordBlock(
  index: number,
  columns: ReceiptColumn[],
  row: string[],
  width: number
): string[] {
  const lines: string[] = [];
  const headPrefix = `${index} `;
  const headLabel = `${columns[0]?.title ?? ''} `;
  const headText = row[0] ?? '';
  const headBody = wrapDisplay(
    headText,
    Math.max(4, width - displayWidth(headPrefix) - displayWidth(headLabel))
  );
  headBody.forEach((line, lineIndex) => {
    if (lineIndex === 0) {
      lines.push(`${headPrefix}${headLabel}${line}`);
    } else {
      lines.push(`${' '.repeat(displayWidth(headPrefix) + displayWidth(headLabel))}${line}`);
    }
  });
  for (let i = 1; i < columns.length; i += 1) {
    const cell = row[i] ?? '';
    if (!cell) continue;
    const label = `${columns[i].title} `;
    const indent = ' '.repeat(displayWidth(headPrefix));
    const body = wrapDisplay(
      cell,
      Math.max(4, width - displayWidth(headPrefix) - displayWidth(label))
    );
    body.forEach((line, lineIndex) => {
      if (lineIndex === 0) {
        lines.push(`${indent}${label}${line}`);
      } else {
        lines.push(`${' '.repeat(displayWidth(indent) + displayWidth(label))}${line}`);
      }
    });
  }
  return lines;
}

/** 将已格式化的表格内容排成热敏小票纯文本，宽度按纸张换算，可另行指定行宽。 */
export function formatReceiptText(
  content: ReceiptContent,
  paperWidth: ReceiptPaperWidth,
  lineWidthOverride?: number
): string {
  const lineWidth = lineWidthOverride ?? LINE_WIDTH[paperWidth];
  const separator = '-'.repeat(lineWidth);
  const lines: string[] = [];

  for (const line of wrapDisplay(content.title, lineWidth)) {
    lines.push(padDisplay(line, lineWidth, 'center'));
  }
  for (const item of content.meta) {
    lines.push(...wrapDisplay(item, lineWidth));
  }
  lines.push(separator);

  if (content.columns.length > 0 && content.rows.length > 0) {
    if (content.columns.length <= maxTableColumns(lineWidth)) {
      const gaps = content.columns.length - 1;
      const widths = allocateWidths(content.columns, content.rows, lineWidth - gaps);
      const aligns = content.columns.map(column => column.align ?? 'left');
      lines.push(
        formatTableRow(
          content.columns.map(column => column.title),
          widths,
          aligns
        )
      );
      lines.push(separator);
      for (const row of content.rows) {
        lines.push(formatTableRow(row, widths, aligns));
      }
    } else {
      content.rows.forEach((row, index) => {
        lines.push(...formatRecordBlock(index + 1, content.columns, row, lineWidth));
      });
    }
    lines.push(separator);
  }

  for (const item of content.footer) {
    lines.push(...wrapDisplay(item, lineWidth));
  }
  return lines.join('\n');
}

/**
 * 发送小票到蓝牙打印机，结果以消息提示，不向调用方抛错。
 * 带路由键时先按单据或标签找打印机，未分配该类型则用默认打印机；
 * 每次打印前确认蓝牙权限并把打印机地址一并下发，壳侧据此在会话失效或换机时重连。
 */
export async function printReceiptToBluetooth(
  content: ReceiptContent,
  options: ReceiptPrintOptions = {}
): Promise<boolean> {
  const printer = resolvePrinterForTarget(options.target);
  if (!printer) {
    msg.warning('printBluetoothPrinterNotSelected');
    return false;
  }
  try {
    const permission = await requestBluetoothPermission();
    if (!permission.granted) {
      msg.warning('printBluetoothPermissionDenied');
      return false;
    }
    const paperWidth = options.paperWidth ?? loadPaperWidth();
    const text = formatReceiptText(
      content,
      paperWidth,
      options.paperWidthMm != null ? lineWidthForPaperWidth(options.paperWidthMm) : undefined
    );
    await sendBluetoothText(text, {
      cut: options.cut,
      feedLines: options.feedLines,
      address: printer.address,
    });
    msg.success(options.successMessage ?? 'printBluetoothSent');
    return true;
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    ElMessage.error(`${msg.translate('printBluetoothFailed')}，${detail}`);
    return false;
  }
}

export interface BluetoothLabelField {
  label: string;
  value: string;
}

/** 把标签字段排成热敏文本发送到标签打印机，标题居中、字段逐行左对齐。 */
export async function printLabelToBluetooth(
  title: string,
  fields: BluetoothLabelField[],
  options: ReceiptPrintOptions = {}
): Promise<boolean> {
  return printReceiptToBluetooth(
    {
      title,
      meta: [],
      columns: [],
      rows: [],
      footer: fields.map(field => `${field.label} ${field.value}`),
    },
    { successMessage: 'printBluetoothSent', ...options }
  );
}
