import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

/**
 * 页面崩溃巡检防线（flow/46-page-health-patrol）
 *
 * 背景（生产崩溃复盘 2026-09-13）：面料列表/胚布管理/BI 销售分析
 * "r is not iterable"、报表模板/数据导入 "toUpperCase of undefined"、
 * 邮件管理 TDZ ReferenceError、AI 扩展 "unacknowledged of undefined"
 * ——全部是页面级白屏崩溃，此前 smoke 仅断言"可达"未断言"健康"。
 *
 * 防线规则：每页巡检断言
 *   1. 无致命 console error（TypeError/ReferenceError/SyntaxError 类）
 *   2. 根容器渲染（app 根节点有内容，非白屏）
 *   3. 页面无未捕获 Promise rejection（pageerror 事件）
 *
 * 命中崩溃的页面即 CI 失败并列出错误明细。
 */

const PATROL_ROUTES: Array<{ path: string; name: string }> = [
  { path: '/product', name: '产品管理（面料列表）' },
  { path: '/greige-fabrics', name: '胚布管理' },
  { path: '/fabric', name: '面料管理' },
  { path: '/sales-analysis', name: 'BI 销售分析' },
  { path: '/report-templates', name: '报表模板' },
  { path: '/data-import', name: '数据导入' },
  { path: '/email', name: '邮件管理' },
  { path: '/ai-extend', name: 'AI 扩展' },
  { path: '/dashboard', name: '仪表盘' },
  { path: '/system', name: '系统管理' },
  { path: '/departments', name: '部门管理' },
  { path: '/voucher', name: '凭证列表' },
  { path: '/account-subject', name: '会计科目' },
  { path: '/inventory', name: '库存列表' },
  { path: '/inventory-transfer', name: '库存调拨' },
  { path: '/inventory-count', name: '库存盘点' },
  { path: '/purchase-return', name: '采购退货' },
  { path: '/fixed-assets', name: '固定资产' },
  { path: '/budget', name: '预算' },
  { path: '/system/audit-log', name: '审计日志' },
  { path: '/flow-cards', name: '流转卡管理' },
  { path: '/lab-dip', name: '打样管理' },
  { path: '/bulk-color-approval', name: '大货批色审批' },
  { path: '/production-recipes', name: '生产配方' },
  { path: '/quality-8d', name: '质量 8D' },
  { path: '/outsourcing', name: '委外管理' },
  { path: '/wage', name: '产量工资' },
  { path: '/fabric-inspections', name: '验布管理' },
  { path: '/chemicals', name: '染化料管理' },
  { path: '/bad-debts', name: '坏账管理' },
  { path: '/period-adjustments', name: '期末调整' },
  { path: '/budgets', name: '预算审批' },
  { path: '/invoice-details', name: '发票管理' },
  { path: '/periods', name: '会计期间' },
  { path: '/labor-contracts', name: '劳动合同' },
  { path: '/social-insurance', name: '社保管理' },
  { path: '/occupational-health', name: '职业健康' },
  { path: '/export-compliance', name: '外贸合规中心' },
  { path: '/system-governance', name: '系统治理中心' },
  { path: '/customer-collab', name: '合同签署与客户协作' },
  { path: '/custom-orders', name: '定制订单' },
  { path: '/crm-enhanced', name: 'CRM 高级分析' },
  { path: '/supplier-enhanced', name: '供应商 360' },
];

const FATAL_PATTERNS = [
  /TypeError:/,
  /ReferenceError:/,
  /SyntaxError:/,
  /is not iterable/,
  /Cannot read propert/,
  /Cannot access .* before initialization/,
  /null is not an object/,
  /undefined is not an object/,
];

test.describe.serial('页面崩溃巡检（生产崩溃回归防线）', () => {
  test('46-1 全路由健康巡检：无致命错误+非白屏', async ({ page }) => {
    await loginViaUI(page);

    const crashes: string[] = [];

    page.on('console', msg => {
      if (msg.type() !== 'error') return;
      const text = msg.text();
      if (FATAL_PATTERNS.some(p => p.test(text))) {
        crashes.push(`[console] ${text.slice(0, 300)}`);
      }
    });
    page.on('pageerror', err => {
      if (FATAL_PATTERNS.some(p => p.test(String(err)))) {
        crashes.push(`[pageerror] ${String(err).slice(0, 300)}`);
      }
    });

    for (const route of PATROL_ROUTES) {
      const before = crashes.length;
      await page.goto(`${BASE_URL}${route.path}`, { waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(2500); // 等异步数据加载 + 渲染

      // 白屏检测：#app 根容器必须有实际内容
      const rootText = await page
        .locator('#app')
        .innerText()
        .catch(() => '');
      if (rootText.trim().length < 10) {
        crashes.push(`[白屏] ${route.path} 根容器文本长度 ${rootText.trim().length}`);
      }

      if (crashes.length > before) {
        crashes.splice(before, 0, `--- 页面 ${route.name}(${route.path}) ---`);
      }
    }

    expect(
      crashes,
      `发现 ${crashes.length} 处页面崩溃（生产同类问题回归）：\n${crashes.join('\n')}`
    ).toHaveLength(0);
  });

  test('46-2 崩溃页面复检：修复后的表格数据形态可渲染（数组可迭代）', async ({ page }) => {
    await loginViaUI(page);

    // 针对性复检"r is not iterable"家族：数据形态错误时 el-table 内部
    // updateAllSelected 抛 TypeError 且表格无法完成渲染
    const tableChecks: Array<[string, string]> = [
      ['/product', '面料列表（产品管理）'],
      ['/greige-fabrics', '胚布管理'],
      ['/sales-analysis', 'BI 销售分析'],
    ];
    for (const [path, label] of tableChecks) {
      const errors: string[] = [];
      page.on('pageerror', err => errors.push(String(err)));
      await page.goto(`${BASE_URL}${path}`, { waitUntil: 'domcontentloaded' });
      await page.waitForTimeout(2000);
      expect(errors, `${label}(${path}) 不应有未捕获异常`).toHaveLength(0);
      const hasTable = await page
        .locator('.el-table, .el-card, .el-empty')
        .first()
        .isVisible()
        .catch(() => false);
      expect(hasTable, `${label}(${path}) 表格/卡片容器应渲染`).toBe(true);
    }
  });
});
