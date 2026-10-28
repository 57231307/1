/**
 * 表格列绑定断言原语（权限域各功能页共用）。
 *
 * 用途：把"某一列有没有按后端字段真实出值"做成可判红的断言，而不是只断页面没崩。
 * 口径：列下标一律由列头文案实时求得，不写死位置；列顺序调整时求得的仍是该列自身的单元格，
 * 列缺失或绑到后端不返回的键都会显式失败（抛错并附实际列头清单），不做软跳过。
 * 表格根节点按 el-table 的 aria-label 锚定（各页 :aria-label 取 i18n 键，与页面同源）。
 */
import { expect, type Locator, type Page } from '@playwright/test';
import { BASE_URL } from './helpers';

/** 主表体行的选择器（Element Plus 标准 el-table，非 el-table-v2 虚拟表） */
const BODY_ROW = '.el-table__body-wrapper .el-table__row';

/**
 * 进入功能页：断言该页 h2 标题可见，并等其主表渲染出来。
 * 出参：主表 Locator（后续列/单元格断言都以它为作用域，避免跨页面误命中同名表头）。
 */
export async function openPageTable(
  page: Page,
  path: string,
  pageTitle: string,
  tableAriaLabel: string
): Promise<Locator> {
  await page.goto(`${BASE_URL}${path}`);
  await expect(
    page.getByRole('heading', { level: 2, name: pageTitle }),
    `未渲染页面标题 ${pageTitle}，实际落点 ${page.url()}`
  ).toBeVisible({ timeout: 30_000 });
  const table = page.locator(`.el-table[aria-label=${JSON.stringify(tableAriaLabel)}]`).first();
  await table.waitFor({ state: 'visible', timeout: 30_000 });
  return table;
}

/**
 * 按列头文案求主表头中的列下标（跳过无对应正文单元格的滚动条 gutter 列头）。
 * 列头不存在即抛错，附实际列头清单，便于直接判是文案漂移还是列缺失。
 */
export async function columnIndex(table: Locator, headerText: string): Promise<number> {
  const headers = table.locator('.el-table__header-wrapper thead th.el-table__cell:not(.gutter)');
  const texts = (await headers.allInnerTexts()).map(text => text.trim());
  const index = texts.indexOf(headerText);
  if (index < 0) {
    throw new Error(`列头 ${headerText} 未找到，实际列头=${JSON.stringify(texts)}`);
  }
  return index;
}

/**
 * 主表体某列全部单元格文本（td 与列头 th 一一对应，下标同源）。
 * 取数方式：整列一次性批量读（按行内第 index 加 1 个 td 的选择器一次查询全部命中节点，
 * 再整体取文本），不做逐格往返。为什么这样取：权限目录一类上千行的表若逐格串行读，
 * 每格一次跨进程往返，行数乘列数直接把用例拖过全局时限；批量读一次往返拿回整列，
 * 断言仍在调用方对每格逐一执行，强度不变。出参下标与表体行序一致，供调用方逐格比对。
 */
export async function columnCellTexts(table: Locator, index: number): Promise<string[]> {
  const cells = table.locator(`${BODY_ROW} > td:nth-child(${index + 1})`);
  return (await cells.allInnerTexts()).map(text => text.trim());
}

/** 指定数据行某列单元格文本 */
export async function rowCellText(row: Locator, index: number): Promise<string> {
  return (await row.locator('> td').nth(index).innerText()).trim();
}

/**
 * 按列头文案锚定某数据行的行内录入框 Locator（rowCellText 的录入面对端）。
 *
 * 用途 表格行内数值录入换成 el-input 底层的手写录入控件后，控件不再有 spinbutton 角色，
 * 行内录入框唯一稳定锚点是列头文案。列下标由 columnIndex 按表头实时求得（与列文本断言同源），
 * 列序调整时取到的仍是该列单元格，列缺失即抛错并附实际列头清单，不做静默兜底。
 * 调用方 采购收货对话框本次收货数量录入（purchase/13、fullflow/20），
 * 采购退货明细退货数量预填回读（purchase/11、fullflow/22）。
 * 入参 table 主表 Locator（与 columnIndex 同容器），row 数据行 Locator，
 * headerText 列头全文（与页面 i18n 列头逐字一致）。
 * 传给谁 返回的 Locator 由调用方 fill、失焦与 toHaveValue 读回，本函数只负责定位并等可见。
 * 存什么 不存储 仅定位
 */
export async function cellInputByColumn(
  table: Locator,
  row: Locator,
  headerText: string
): Promise<Locator> {
  const index = await columnIndex(table, headerText);
  const input = row.locator('> td').nth(index).locator('input').first();
  await input.waitFor({ state: 'visible', timeout: 30_000 });
  return input;
}

/** 主表体行数 */
export async function rowCount(table: Locator): Promise<number> {
  return table.locator(BODY_ROW).count();
}

/** 等表格渲染出至少一行数据（等待本身即断言，超时判红） */
export async function waitFirstRow(table: Locator): Promise<void> {
  await table.locator(BODY_ROW).first().waitFor({ state: 'visible', timeout: 30_000 });
}

/** 按行内文本（自建数据的唯一业务值）定位数据行；传正则时可同时命中渲染真值的多种语言形态 */
export async function rowByText(table: Locator, text: string | RegExp): Promise<Locator> {
  const row = table.locator(BODY_ROW).filter({ hasText: text }).first();
  await row.waitFor({ state: 'visible', timeout: 30_000 });
  return row;
}
