/**
 * 成品布标签的字段选择与页面排版。
 *
 * 调用方 验布页标签入口在安卓端打印标签时使用，桌面端仍走后端 docx 下载。
 * 入参 匹数据、字段名称映射与标签纸张设置。
 * 传给谁 蓝牙打印用字段行文本，系统打印用带 @page 的整页 HTML。
 * 存什么 不存，字段选择与纸张尺寸来自打印设置。
 * 存哪里 无。
 */
import type { BluetoothLabelField } from '@/utils/bluetooth-print';
import { escapeHtml } from '@/utils/print';
import {
  buildPageSizeCss,
  type LabelFieldKey,
  type LabelPaperSettings,
} from '@/utils/print-settings';

export interface LabelPieceData {
  piece_no: string;
  dye_lot_no: string | null;
  color_no: string | null;
  length: string;
  weight: string | null;
  width: string | null;
  gram_weight: string | null;
  barcode: string | null;
  warehouse_in_at: string | null;
}

function describeField(piece: LabelPieceData, field: LabelFieldKey): string {
  switch (field) {
    case 'piece_no':
      return piece.piece_no;
    case 'dye_lot_no':
      return piece.dye_lot_no || '-';
    case 'color_no':
      return piece.color_no || '-';
    case 'length':
      return String(Number(piece.length));
    case 'weight':
      return piece.weight == null ? '-' : String(Number(piece.weight));
    case 'width':
      return piece.width == null ? '-' : String(Number(piece.width));
    case 'gram_weight':
      return piece.gram_weight == null ? '-' : String(Number(piece.gram_weight));
    case 'barcode':
      return piece.barcode || '-';
    case 'warehouse_in_at':
      return piece.warehouse_in_at || '-';
  }
}

/** 按标签设置里的字段选择生成字段行，顺序与设置一致。 */
export function buildPieceLabelFields(
  piece: LabelPieceData,
  labels: Record<LabelFieldKey, string>,
  fields: LabelFieldKey[]
): BluetoothLabelField[] {
  return fields.map(field => ({ label: labels[field], value: describeField(piece, field) }));
}

/** 生成标签整页 HTML，纸张与方向来自标签设置，供系统打印对话框取页。 */
export function buildPieceLabelHtml(
  title: string,
  fields: BluetoothLabelField[],
  paper: LabelPaperSettings
): string {
  const rows = fields
    .map(
      field =>
        `<div class="row"><span class="label">${escapeHtml(field.label)}</span><span class="value">${escapeHtml(field.value)}</span></div>`
    )
    .join('');
  return `
    <!DOCTYPE html>
    <html>
    <head>
      <meta charset="utf-8">
      <title>${escapeHtml(title)}</title>
      <style>
        @media print {
          @page { size: ${buildPageSizeCss(paper)}; margin: 2mm; }
        }
        body { font-family: "Microsoft YaHei", "SimSun", sans-serif; font-size: 12px; color: #000; padding: 2mm; }
        h1 { font-size: 14px; text-align: center; margin: 0 0 2mm; }
        .row { display: flex; justify-content: space-between; gap: 8px; line-height: 1.6; border-bottom: 1px dashed #999; }
        .label { color: #555; }
        .value { font-weight: 600; }
      </style>
    </head>
    <body>
      <h1>${escapeHtml(title)}</h1>
      ${rows}
    </body>
    </html>
  `;
}
