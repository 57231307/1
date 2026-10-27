/**
 * 手机端打印纸张与标签字段设置。
 *
 * 调用方 打印目标页负责编辑，打印工具与标签打印读取后生成页面与文本排版。
 * 入参 纸张宽度、高度与方向，标签另带需要打印的字段键。
 * 传给谁 系统打印用宽高与方向拼 @page，蓝牙文本只用纸宽换算行宽。
 * 存什么 单据纸张与标签纸张两份设置。
 * 存哪里 浏览器 localStorage。
 */

export type PrintOrientation = 'portrait' | 'landscape';

export interface PaperSettings {
  /** 纸张宽度，单位毫米 */
  widthMm: number;
  /** 纸张高度，单位毫米，0 表示连续纸 */
  heightMm: number;
  orientation: PrintOrientation;
}

export const LABEL_FIELD_KEYS = [
  'piece_no',
  'dye_lot_no',
  'color_no',
  'length',
  'weight',
  'width',
  'gram_weight',
  'barcode',
  'warehouse_in_at',
] as const;

export type LabelFieldKey = (typeof LABEL_FIELD_KEYS)[number];

export interface LabelPaperSettings extends PaperSettings {
  /** 需要打印到标签上的字段，顺序与 LABEL_FIELD_KEYS 一致 */
  fields: LabelFieldKey[];
}

const DOCUMENT_PAPER_KEY = 'bingxi.print.documentPaper';
const LABEL_PAPER_KEY = 'bingxi.print.labelPaper';
const DEFAULT_DOCUMENT: PaperSettings = { widthMm: 210, heightMm: 297, orientation: 'portrait' };
const DEFAULT_LABEL: LabelPaperSettings = {
  widthMm: 40,
  heightMm: 30,
  orientation: 'portrait',
  fields: [...LABEL_FIELD_KEYS],
};

function clampNumber(value: unknown, min: number, max: number, fallback: number): number {
  const parsed = typeof value === 'number' ? value : Number(value);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.min(max, Math.max(min, parsed));
}

function readStored<T>(key: string): Partial<T> | null {
  try {
    const raw = window.localStorage.getItem(key);
    if (!raw) return null;
    const parsed: unknown = JSON.parse(raw);
    return parsed && typeof parsed === 'object' ? (parsed as Partial<T>) : null;
  } catch {
    return null;
  }
}

/**
 * 未单独保存时，已连接过蓝牙打印机的设备按旧纸张宽度回退成连续纸，
 * 其余设备保持 A4，避免热敏小票被排成整页宽度。
 */
function defaultDocumentPaper(): PaperSettings {
  try {
    if (!window.localStorage.getItem('bingxi.bluetooth.printer')) {
      return { ...DEFAULT_DOCUMENT };
    }
    const legacyWidth = window.localStorage.getItem('bingxi.bluetooth.paper') === '80' ? 80 : 58;
    return { widthMm: legacyWidth, heightMm: 0, orientation: 'portrait' };
  } catch {
    return { ...DEFAULT_DOCUMENT };
  }
}

/** 读取单据纸张，未保存过时按设备是否用过蓝牙打印机给出连续纸或 A4。 */
export function loadDocumentPaper(): PaperSettings {
  const stored = readStored<PaperSettings>(DOCUMENT_PAPER_KEY);
  if (!stored) return defaultDocumentPaper();
  return {
    widthMm: clampNumber(stored.widthMm, 20, 1000, DEFAULT_DOCUMENT.widthMm),
    heightMm: clampNumber(stored.heightMm, 0, 1000, DEFAULT_DOCUMENT.heightMm),
    orientation: stored.orientation === 'landscape' ? 'landscape' : 'portrait',
  };
}

/** 保存单据纸张，宽高按界面可选范围收敛后再落盘。 */
export function saveDocumentPaper(settings: PaperSettings): void {
  try {
    window.localStorage.setItem(
      DOCUMENT_PAPER_KEY,
      JSON.stringify({
        widthMm: clampNumber(settings.widthMm, 20, 1000, DEFAULT_DOCUMENT.widthMm),
        heightMm: clampNumber(settings.heightMm, 0, 1000, DEFAULT_DOCUMENT.heightMm),
        orientation: settings.orientation === 'landscape' ? 'landscape' : 'portrait',
      })
    );
  } catch {
    // 存储不可用时本次仍按内存设置打印，重启后回到默认
  }
}

/** 读取标签纸张与需要打印的字段，字段按固定顺序过滤掉未知键。 */
export function loadLabelPaper(): LabelPaperSettings {
  const stored = readStored<LabelPaperSettings>(LABEL_PAPER_KEY);
  if (!stored) return { ...DEFAULT_LABEL, fields: [...DEFAULT_LABEL.fields] };
  const fields = Array.isArray(stored.fields)
    ? LABEL_FIELD_KEYS.filter(field => (stored.fields ?? []).includes(field))
    : [...LABEL_FIELD_KEYS];
  return {
    widthMm: clampNumber(stored.widthMm, 20, 300, DEFAULT_LABEL.widthMm),
    heightMm: clampNumber(stored.heightMm, 10, 300, DEFAULT_LABEL.heightMm),
    orientation: stored.orientation === 'landscape' ? 'landscape' : 'portrait',
    fields,
  };
}

/** 保存标签纸张与字段，未知字段键在落盘前丢弃。 */
export function saveLabelPaper(settings: LabelPaperSettings): void {
  try {
    window.localStorage.setItem(
      LABEL_PAPER_KEY,
      JSON.stringify({
        widthMm: clampNumber(settings.widthMm, 20, 300, DEFAULT_LABEL.widthMm),
        heightMm: clampNumber(settings.heightMm, 10, 300, DEFAULT_LABEL.heightMm),
        orientation: settings.orientation === 'landscape' ? 'landscape' : 'portrait',
        fields: LABEL_FIELD_KEYS.filter(field => settings.fields.includes(field)),
      })
    );
  } catch {
    // 存储不可用时本次仍按内存设置打印，重启后回到默认
  }
}

/** 蓝牙文本行宽按纸张物理宽度换算，58 毫米 32 字符、80 毫米 48 字符为基准。 */
export function lineWidthForPaperWidth(widthMm: number): number {
  const width = clampNumber(widthMm, 20, 1000, 58);
  if (width <= 58) return Math.max(16, Math.round((width * 32) / 58));
  return Math.max(32, Math.round((width * 48) / 80));
}

/** 系统打印的页面尺寸，横向交换宽高，高度为 0 时只声明宽度。 */
export function buildPageSizeCss(paper: PaperSettings): string {
  const { widthMm, heightMm, orientation } = paper;
  if (heightMm <= 0) return `${widthMm}mm auto`;
  if (orientation === 'landscape') return `${heightMm}mm ${widthMm}mm`;
  return `${widthMm}mm ${heightMm}mm`;
}
