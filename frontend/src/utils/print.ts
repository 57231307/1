import { ElMessage } from 'element-plus';
import { msg } from '@/utils/message';
import { recordPrintAudit } from '@/api/audit';
import { useUserStore } from '@/store/user';
import {
  documentPrintRoute,
  isAndroidClient,
  printReceiptToBluetooth,
  resolvePrinterForTarget,
  type DocumentPrintType,
  type ReceiptColumn,
  type ReceiptContent,
} from '@/utils/bluetooth-print';
import { printHtmlOnAndroid } from '@/utils/system-print';
import { buildPageSizeCss, loadDocumentPaper } from '@/utils/print-settings';

/**
 * 打印列定义接口
 * @template T - 数据行类型
 */
export interface PrintColumn<T extends Record<string, unknown> = Record<string, unknown>> {
  key: keyof T & string;
  title: string;
  width?: string;
  align?: 'left' | 'center' | 'right';
  formatter?: (value: unknown, row: T) => string;
}

/**
 * 打印选项接口
 * @template T - 数据行类型
 */
export interface PrintOptions<T extends Record<string, unknown> = Record<string, unknown>> {
  title: string;
  columns: PrintColumn<T>[];
  data: T[];
  extraInfo?: { label: string; value: string }[];
  orientation?: 'portrait' | 'landscape';
  /** V15 P1-5-3：资源类型（用于前端打印审计埋点，与后端权限码 resource_type 对应） */
  resourceType?: string;
  /** V15 P1-5-3：资源 ID（单据打印时传入，用于审计追溯） */
  resourceId?: string;
  /** 单据类型，安卓端据此选择为该单据分配的蓝牙打印机 */
  documentKey?: DocumentPrintType;
}

/**
 * 当前打印人名称，供桌面打印页与蓝牙小票共用。
 * 优先取用户信息，Pinia 未初始化时回退到 localStorage 记下的用户名。
 */
function resolvePrintUser(): string {
  try {
    const userStore = useUserStore();
    if (userStore.userInfo?.username) {
      return userStore.userInfo.username;
    }
  } catch {
    const stored = localStorage.getItem('username');
    if (stored) {
      return stored;
    }
  }
  return '未知用户';
}

function formatPrintTimestamp(now: Date): string {
  const date = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
  const time = `${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;
  return `${date} ${time}`;
}

/** 蓝牙小票头部的打印人与时间信息，字段之间用两个空格分隔。 */
function describeBluetoothMeta(): string {
  return `打印人 ${resolvePrintUser()}  打印时间 ${formatPrintTimestamp(new Date())}  IP ${window.location.hostname}`;
}

function recordPrintAuditBestEffort(options: {
  resourceType?: string;
  resourceId?: string;
  title: string;
  recordCount: number;
}): void {
  if (!options.resourceType) return;
  recordPrintAudit({
    resourceType: options.resourceType,
    recordCount: options.recordCount,
    title: options.title,
    resourceId: options.resourceId,
  }).catch(err => {
    console.warn('[打印] 审计埋点失败', err);
  });
}

function buildListReceiptContent<T extends Record<string, unknown>>(
  options: PrintOptions<T>
): ReceiptContent {
  const columns: ReceiptColumn[] = options.columns.map(column => ({
    title: column.title,
    align: column.align,
  }));
  const rows = options.data.map(row =>
    options.columns.map(column => {
      const value = row[column.key];
      const formatted = column.formatter ? column.formatter(value, row) : (value ?? '');
      return String(formatted ?? '');
    })
  );
  return {
    title: options.title,
    meta: [
      ...(options.extraInfo ?? []).map(info => `${info.label} ${info.value}`),
      describeBluetoothMeta(),
    ],
    columns,
    rows,
    footer: [`共 ${options.data.length} 条记录`],
  };
}

function buildDocumentReceiptContent<T extends Record<string, unknown>>(options: {
  title: string;
  info: Record<string, string>;
  items: T[];
  itemColumns: PrintColumn<T>[];
  footer?: Record<string, string>;
}): ReceiptContent {
  const columns: ReceiptColumn[] = options.itemColumns.map(column => ({
    title: column.title,
    align: column.align,
  }));
  const rows = options.items.map(row =>
    options.itemColumns.map(column => {
      const value = row[column.key];
      const formatted = column.formatter ? column.formatter(value, row) : (value ?? '');
      return String(formatted ?? '');
    })
  );
  return {
    title: options.title,
    meta: [
      ...Object.entries(options.info).map(([label, value]) => `${label} ${value}`),
      describeBluetoothMeta(),
    ],
    columns,
    rows,
    footer: [
      ...Object.entries(options.footer ?? {}).map(([label, value]) => `${label} ${value}`),
      `共 ${options.items.length} 条明细`,
    ],
  };
}

/** 单据系统打印的页面规则，窄纸收紧边距，方向可由调用方覆盖。 */
function buildPrintPageRule(orientation?: 'portrait' | 'landscape'): string {
  const paper = loadDocumentPaper();
  const effective = orientation ? { ...paper, orientation } : paper;
  const margin = effective.widthMm < 100 ? '2mm' : '15mm';
  return `@page { size: ${buildPageSizeCss(effective)}; margin: ${margin}; }`;
}

/** 把整页 HTML 交给安卓系统打印，失败时把壳侧原因带给用户。 */
async function printHtmlToSystem(html: string, title: string): Promise<void> {
  try {
    await printHtmlOnAndroid(html, title);
    msg.success('printSystemOpened');
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    ElMessage.error(`${msg.translate('printSystemFailed')}，${detail}`);
  }
}

/**
 * 安卓端列表打印入口。
 * 该单据分配过蓝牙打印机或存在默认打印机时走蓝牙小票，
 * 否则把桌面同款表格 HTML 交给系统打印对话框。
 */
async function printListOnAndroid<T extends Record<string, unknown>>(
  options: PrintOptions<T>
): Promise<void> {
  recordPrintAuditBestEffort({
    resourceType: options.resourceType,
    resourceId: options.resourceId,
    title: options.title,
    recordCount: options.data.length,
  });
  const target = options.documentKey ? documentPrintRoute(options.documentKey) : undefined;
  if (resolvePrinterForTarget(target)) {
    await printReceiptToBluetooth(buildListReceiptContent(options), {
      target,
      paperWidthMm: loadDocumentPaper().widthMm,
    });
    return;
  }
  await printHtmlToSystem(generatePrintHTML(options), options.title);
}

/** 安卓端单据打印入口，与列表打印同样的按单据选打印机规则。 */
async function printDocumentOnAndroid<T extends Record<string, unknown>>(options: {
  title: string;
  info: Record<string, string>;
  items: T[];
  itemColumns: PrintColumn<T>[];
  footer?: Record<string, string>;
  resourceType?: string;
  resourceId?: string;
  documentKey?: DocumentPrintType;
}): Promise<void> {
  recordPrintAuditBestEffort({
    resourceType: options.resourceType,
    resourceId: options.resourceId,
    title: options.title,
    recordCount: options.items.length,
  });
  const target = options.documentKey ? documentPrintRoute(options.documentKey) : undefined;
  if (resolvePrinterForTarget(target)) {
    await printReceiptToBluetooth(buildDocumentReceiptContent(options), {
      target,
      paperWidthMm: loadDocumentPaper().widthMm,
    });
    return;
  }
  await printHtmlToSystem(generateDocumentHTML(options), options.title);
}

export interface PrintTableColumn {
  field: string;
  displayName: string;
}

/**
 * 安卓端打印一组 JSON 行，供桌面端走 printJS 的页面在壳内复用。
 * 该单据分配过蓝牙打印机时改走蓝牙小票，其余情况交给系统打印；
 * 返回 true 表示本次已在安卓端处理（含无数据提示），调用方不必再走桌面分支。
 */
export async function printTableOnAndroid(options: {
  title: string;
  rows: Record<string, unknown>[];
  columns: Array<string | PrintTableColumn>;
  documentKey?: DocumentPrintType;
}): Promise<boolean> {
  if (!isAndroidClient()) return false;
  if (options.rows.length === 0) {
    msg.warning('noDataToPrint');
    return true;
  }
  const columns = options.columns.map(column =>
    typeof column === 'string' ? { field: column, displayName: column } : column
  );
  const target = options.documentKey ? documentPrintRoute(options.documentKey) : undefined;
  if (resolvePrinterForTarget(target)) {
    await printReceiptToBluetooth(
      {
        title: options.title,
        meta: [describeBluetoothMeta()],
        columns: columns.map(column => ({ title: column.displayName })),
        rows: options.rows.map(row => columns.map(column => String(row[column.field] ?? ''))),
        footer: [`共 ${options.rows.length} 条记录`],
      },
      { target, paperWidthMm: loadDocumentPaper().widthMm }
    );
    return true;
  }
  const headerCells = columns
    .map(
      column =>
        `<th style="padding: 8px 12px; border: 1px solid #333; background: #f5f5f5; text-align: left">${escapeHtml(column.displayName)}</th>`
    )
    .join('');
  const bodyRows = options.rows
    .map(
      row =>
        `<tr>${columns
          .map(
            column =>
              `<td style="padding: 6px 12px; border: 1px solid #333">${escapeHtml(row[column.field] ?? '')}</td>`
          )
          .join('')}</tr>`
    )
    .join('');
  const html = `
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="utf-8">
      <title>${escapeHtml(options.title)}</title>
      <style>
        @media print {
          ${buildPrintPageRule()}
        }
        body { font-family: "Microsoft YaHei", "SimSun", sans-serif; font-size: 12px; color: #333; padding: 20px; }
        h1 { font-size: 20px; text-align: center; margin-bottom: 8px; }
        .print-meta { text-align: center; color: #666; font-size: 11px; margin-bottom: 16px; }
        table { width: 100%; border-collapse: collapse; }
      </style>
    </head>
    <body>
      <h1>${escapeHtml(options.title)}</h1>
      <div class="print-meta">打印人 ${escapeHtml(resolvePrintUser())} 打印时间 ${formatPrintTimestamp(new Date())} 共 ${options.rows.length} 条记录</div>
      <table>
        <thead><tr>${headerCells}</tr></thead>
        <tbody>${bodyRows}</tbody>
      </table>
    </body>
    </html>
  `;
  await printHtmlToSystem(html, options.title);
  return true;
}

/**
 * 生成打印 HTML 内容
 */
function generatePrintHTML<T extends Record<string, unknown>>(options: PrintOptions<T>): string {
  const { title, columns, data, extraInfo, orientation = 'portrait' } = options;

  const headerCells = columns
    .map(
      col =>
        `<th style="padding: 8px 12px; border: 1px solid #333; background: #f5f5f5; text-align: ${col.align || 'left'}; ${col.width ? `width: ${col.width}` : ''}">${escapeHtml(col.title)}</th>`
    )
    .join('');

  const bodyRows = data
    .map(row => {
      const cells = columns
        .map(col => {
          const value = row[col.key];
          const formatted = col.formatter ? col.formatter(value, row) : (value ?? '');
          return `<td style="padding: 6px 12px; border: 1px solid #333; text-align: ${col.align || 'left'}">${escapeHtml(formatted)}</td>`;
        })
        .join('');
      return `<tr>${cells}</tr>`;
    })
    .join('');

  const infoSection = extraInfo
    ? `<div style="margin: 16px 0; display: flex; gap: 32px;">
        ${extraInfo.map(info => `<span><strong>${escapeHtml(info.label)}:</strong> ${escapeHtml(info.value)}</span>`).join('')}
       </div>`
    : '';

  const now = new Date();
  const printDate = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

  const printUser = resolvePrintUser();
  const printContent = `打印人: ${printUser}  打印时间: ${printDate}  IP: ${window.location.hostname}`;

  return `
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="utf-8">
      <title>${escapeHtml(title)}</title>
      <style>
        @media print {
          ${buildPrintPageRule(orientation)}
          body { -webkit-print-color-adjust: exact; print-color-adjust: exact; }
        }
        body { font-family: "Microsoft YaHei", "SimSun", sans-serif; font-size: 12px; color: #333; }
        h1 { font-size: 20px; text-align: center; margin-bottom: 8px; }
        .print-meta { text-align: center; color: #666; font-size: 11px; margin-bottom: 16px; }
        table { width: 100%; border-collapse: collapse; margin-top: 8px; }
        th, td { border: 1px solid #333; }
        .footer { margin-top: 20px; display: flex; justify-content: space-between; font-size: 11px; color: #666; }
      </style>
    </head>
    <body>
      <h1>${escapeHtml(title)}</h1>
      <div class="print-meta">${escapeHtml(printContent)} | 共 ${data.length} 条记录</div>
      ${infoSection}
      <table>
        <thead><tr>${headerCells}</tr></thead>
        <tbody>${bodyRows}</tbody>
      </table>
      <div class="footer">
        <span>打印人: ___________</span>
        <span>审核人: ___________</span>
        <span>日期: ___________</span>
      </div>
    </body>
    </html>
  `;
}

/**
 * 打印数据表格
 */
export function printData<T extends Record<string, unknown>>(options: PrintOptions<T>) {
  if (!options.data || options.data.length === 0) {
    msg.warning('noDataToPrint');
    return;
  }

  if (isAndroidClient()) {
    void printListOnAndroid(options);
    return;
  }

  recordPrintAuditBestEffort({
    resourceType: options.resourceType,
    resourceId: options.resourceId,
    title: options.title,
    recordCount: options.data.length,
  });

  const html = generatePrintHTML(options);
  const printWindow = window.open('', '_blank');
  if (!printWindow) {
    msg.printBlocked();
    return;
  }

  printWindow.document.write(html);
  printWindow.document.close();
  printWindow.onload = () => {
    printWindow.print();
  };
  msg.printOpened();
}
/**
 * 生成单据打印 HTML，桌面弹窗与安卓系统打印共用同一份排版。
 */
function generateDocumentHTML<T extends Record<string, unknown>>(options: {
  title: string;
  info: Record<string, string>;
  items: T[];
  itemColumns: PrintColumn<T>[];
  footer?: Record<string, string>;
}): string {
  const { title, info, items, itemColumns, footer } = options;

  const infoHTML = Object.entries(info)
    .map(
      ([key, value]) =>
        `<span style="margin-right: 32px;"><strong>${escapeHtml(key)}:</strong> ${escapeHtml(value)}</span>`
    )
    .join('');

  const headerCells = itemColumns
    .map(
      col =>
        `<th style="padding: 8px 12px; border: 1px solid #333; background: #f5f5f5; text-align: ${col.align || 'left'}">${escapeHtml(col.title)}</th>`
    )
    .join('');

  const bodyRows = items
    .map(row => {
      const cells = itemColumns
        .map(col => {
          const value = row[col.key];
          const formatted = col.formatter ? col.formatter(value, row) : (value ?? '');
          return `<td style="padding: 6px 12px; border: 1px solid #333; text-align: ${col.align || 'left'}">${escapeHtml(formatted)}</td>`;
        })
        .join('');
      return `<tr>${cells}</tr>`;
    })
    .join('');

  const footerHTML = footer
    ? `<div style="margin-top: 20px; display: flex; justify-content: space-between;">
        ${Object.entries(footer)
          .map(
            ([key, value]) =>
              `<span><strong>${escapeHtml(key)}:</strong> ${escapeHtml(value)}</span>`
          )
          .join('')}
       </div>`
    : '';

  const now = new Date();
  const printDate = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')} ${String(now.getHours()).padStart(2, '0')}:${String(now.getMinutes()).padStart(2, '0')}`;

  const printUser = resolvePrintUser();
  const watermarkText = `打印人: ${printUser}  打印时间: ${printDate}  IP: ${window.location.hostname}`;

  const html = `
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="utf-8">
      <title>${escapeHtml(title)}</title>
      <style>
        @media print {
          ${buildPrintPageRule()}
        }
        body { font-family: "Microsoft YaHei", "SimSun", sans-serif; font-size: 12px; color: #333; padding: 20px; }
        h1 { font-size: 20px; text-align: center; margin-bottom: 16px; }
        .info-section { margin: 16px 0; line-height: 1.8; }
        table { width: 100%; border-collapse: collapse; margin-top: 12px; }
        .sign-area { margin-top: 40px; display: flex; justify-content: space-between; }
        .sign-item { text-align: center; }
        .sign-line { width: 150px; border-bottom: 1px solid #333; margin-top: 40px; }
        .print-meta { text-align: center; color: #666; font-size: 11px; margin-bottom: 16px; }
      </style>
    </head>
    <body>
      <h1>${escapeHtml(title)}</h1>
      <div class="print-meta">${escapeHtml(watermarkText)}</div>
      <div class="info-section">${infoHTML}</div>
      <table>
        <thead><tr>${headerCells}</tr></thead>
        <tbody>${bodyRows}</tbody>
      </table>
      ${footerHTML}
      <div class="sign-area">
        <div class="sign-item"><div class="sign-line"></div>制单人</div>
        <div class="sign-item"><div class="sign-line"></div>审核人</div>
        <div class="sign-item"><div class="sign-line"></div>收货人</div>
      </div>
      <div style="text-align: right; margin-top: 16px; font-size: 11px; color: #999;">打印日期: ${printDate}</div>
    </body>
    </html>
  `;

  return html;
}

/**
 * 打印单个单据（含表头信息、明细行、页脚）
 */
export function printSingleDocument<T extends Record<string, unknown>>(options: {
  title: string;
  info: Record<string, string>;
  items: T[];
  itemColumns: PrintColumn<T>[];
  footer?: Record<string, string>;
  /** 资源类型，用于前端打印审计埋点，与后端权限码 resource_type 对应。 */
  resourceType?: string;
  /** 资源 ID，单据打印时传入，用于审计追溯。 */
  resourceId?: string;
  /** 单据类型，安卓端据此选择为该单据分配的蓝牙打印机 */
  documentKey?: DocumentPrintType;
}) {
  const { title, items } = options;

  if (isAndroidClient()) {
    void printDocumentOnAndroid(options);
    return;
  }

  recordPrintAuditBestEffort({
    resourceType: options.resourceType,
    resourceId: options.resourceId,
    title,
    recordCount: items.length,
  });

  const html = generateDocumentHTML(options);

  const printWindow = window.open('', '_blank');
  if (!printWindow) {
    msg.printBlocked();
    return;
  }
  printWindow.document.write(html);
  printWindow.document.close();
  printWindow.onload = () => {
    printWindow.print();
  };
}

/**
 * HTML 实体转义函数（P1-C MED-4 修复）
 *
 * 用于 printWindow.document.write 之前对用户可控字符串做转义，
 * 防止 XSS 攻击（攻击者可通过设置客户名/产品名/订单备注等字段嵌入 `<script>`）。
 *
 * 为什么不直接用 DOMPurify：
 * - 打印 HTML 是 window 内部 document.write，DOMPurify 需等待 DOM 挂载；
 *   转义函数无依赖，开销可忽略，可在任何地方调用
 * - 转义仅针对 5 个特殊字符（< > & " '），不破坏中文/数字
 *
 * 防御范围：
 * - `<` → `&lt;`
 * - `>` → `&gt;`
 * - `&` → `&amp;`
 * - `"` → `&quot;`
 * - `'` → `&#39;`
 *
 * 不防御：
 * - javascript: URL（document.write 不会执行跳转）
 * - CSS injection（已通过 `<style>` 标签硬编码，攻击者无法注入新样式块）
 *
 * 使用示例：
 * ```ts
 * import { escapeHtml } from '@/utils/print'
 * const safeName = escapeHtml(item.customer_name)
 * printWindow.document.write(`<td>${safeName}</td>`)
 * ```
 */
export function escapeHtml(input: unknown): string {
  if (input === null || input === undefined) {
    return '';
  }
  const str = String(input);
  return str.replace(/[&<>"']/g, char => {
    switch (char) {
      case '&':
        return '&amp;';
      case '<':
        return '&lt;';
      case '>':
        return '&gt;';
      case '"':
        return '&quot;';
      case "'":
        return '&#39;';
      default:
        return char;
    }
  });
}
