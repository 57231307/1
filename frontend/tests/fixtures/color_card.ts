/**
 * 色卡域测试 mock 数据夹具（V15 批次 06 P1-6 修复）
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures。
 * 覆盖色卡列表项、色卡明细、颜色项、发放记录等核心色卡域实体。
 */
import type {
  ColorCardListItem,
  ColorCardDetail,
  ColorItemInfo,
  IssueRecordInfo,
} from '@/api/color-card';

/** 创建颜色项 mock（单个颜色信息） */
export function createColorItemMock(overrides: Partial<ColorItemInfo> = {}): ColorItemInfo {
  return {
    id: 1,
    color_code: '18-1664',
    color_name: '番茄红',
    rgb_r: 220,
    rgb_g: 50,
    rgb_b: 50,
    cmyk_c: 0,
    cmyk_m: 77,
    cmyk_y: 77,
    cmyk_k: 14,
    lab_l: 50,
    lab_a: 60,
    lab_b: 40,
    pantone_code: '18-1664 TPX',
    hex_value: '#DC3232',
    sequence: 1,
    ...overrides,
  };
}

/** 创建色卡列表项 mock（在用状态，可通过 overrides 覆盖） */
export function createColorCardListItemMock(
  overrides: Partial<ColorCardListItem> = {}
): ColorCardListItem {
  const now = new Date().toISOString();
  return {
    id: 1,
    card_no: 'CC2026001',
    card_name: '春夏主推色卡',
    card_type: 'PANTONE',
    season: '2025SS',
    brand: '主品牌',
    total_colors: 5,
    status: 'active',
    cover_image_url: '',
    stock_quantity: 10,
    issued_quantity: 2,
    created_at: now,
    ...overrides,
  };
}

/** 创建已归档色卡 mock */
export function createArchivedColorCardMock(
  overrides: Partial<ColorCardListItem> = {}
): ColorCardListItem {
  return createColorCardListItemMock({ status: 'archived', ...overrides });
}

/** 创建遗失色卡 mock */
export function createLostColorCardMock(
  overrides: Partial<ColorCardListItem> = {}
): ColorCardListItem {
  return createColorCardListItemMock({ status: 'lost', ...overrides });
}

/** 创建色卡明细 mock（含颜色项列表） */
export function createColorCardDetailMock(
  overrides: Partial<ColorCardDetail> = {}
): ColorCardDetail {
  const now = new Date().toISOString();
  return {
    ...createColorCardListItemMock(),
    description: '测试色卡详情',
    items: [
      createColorItemMock(),
      createColorItemMock({
        id: 2,
        color_code: '19-4052',
        color_name: '经典蓝',
        rgb_r: 0,
        rgb_g: 73,
        rgb_b: 130,
        hex_value: '#004982',
        sequence: 2,
      }),
    ],
    updated_at: now,
    ...overrides,
  };
}

/** 创建色卡发放记录 mock（已发放状态） */
export function createIssueRecordMock(overrides: Partial<IssueRecordInfo> = {}): IssueRecordInfo {
  const now = new Date().toISOString();
  return {
    id: 1,
    color_card_id: 1,
    customer_id: 1,
    issue_qty: 1,
    issued_by: 1,
    issued_at: now,
    expected_return_date: now.slice(0, 10),
    status: 'issued',
    purpose: '客户审样',
    remark: '',
    created_at: now,
    updated_at: now,
    ...overrides,
  };
}

/** 创建已归还的发放记录 mock */
export function createReturnedIssueRecordMock(
  overrides: Partial<IssueRecordInfo> = {}
): IssueRecordInfo {
  const now = new Date().toISOString();
  return createIssueRecordMock({
    status: 'returned',
    actual_return_date: now.slice(0, 10),
    returned_by: 2,
    ...overrides,
  });
}

/** 创建遗失发放记录 mock（含赔偿金额） */
export function createLostIssueRecordMock(
  overrides: Partial<IssueRecordInfo> = {}
): IssueRecordInfo {
  return createIssueRecordMock({
    status: 'lost',
    compensation_amount: 500,
    ...overrides,
  });
}

/** 创建色卡列表 mock（默认 3 个不同状态） */
export function createColorCardListMock(count = 3): ColorCardListItem[] {
  const statuses = ['active', 'archived', 'lost'];
  return Array.from({ length: count }, (_, i) =>
    createColorCardListItemMock({
      id: i + 1,
      card_no: `CC2026${String(i + 1).padStart(3, '0')}`,
      card_name: `测试色卡${i + 1}`,
      status: statuses[i % statuses.length] ?? 'active',
      total_colors: (i + 1) * 2,
    })
  );
}
