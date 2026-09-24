// P9-4 采购 E2E 套件 — 07 供应商评估（真实视图）
// 覆盖范围：/supplier-evaluation 供应商评估管理页 tab 切换 / 排名 / 指标渲染

import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';

/**
 * 真实 UI 事实（据 router 与 views/supplier-evaluation/index.vue、locales 核对）：
 * - 原用例 goto '/purchase/report/supplier-summary'、'/purchase/report/on-time-rate'、
 *   '/purchase/report/quality-rate'、'/purchase/report/ap-aging'、'/purchase/report/supplier-grade' ——
 *   这些 /purchase/report/* 子路由在真实应用完全不存在（router 无任何该前缀路由）。
 * - 真实“供应商评级/分析”视图为 /supplier-evaluation（router path:'supplier-evaluation' →
 *   views/supplier-evaluation/index.vue），为多 Tab 页（locales supplierEvaluation.index.*）：
 *   标题 '供应商评估管理'；tab '评估记录'(records,默认) / '供应商排名'(rankings) /
 *   '评估指标'(indicators) / '评估管理'(evaluations)；排名表 aria-label '供应商排名列表'
 *   含列 '排名'/'评级'(A/B/C tag)；'刷新排名'(button.refresh) 触发 getSupplierRankings；
 *   记录 Tab 有 '新建评估'(button.create)。
 *   —— 到货及时率 / 质检合格率 / 应付账龄 三项无供应商维度对应页面（应付账龄位于 /ap 应付发票
 *   的 '账龄分析'，非供应商报表），故不在此文件虚构断言（详见交付报告“删除/降级清单”）。
 */
test.describe('07 供应商评估（真实视图）', () => {
  test.beforeEach(async ({ context }) => {
    await applyAuthMocks(context);
  });

  test('07-01 供应商评估页可访问并渲染主 Tab', async ({ page }) => {
    await page.goto('/supplier-evaluation');
    await expect(page.getByText('供应商评估管理').first()).toBeVisible();
    await expect(page.getByRole('tab', { name: '评估记录' })).toBeVisible();
    await expect(page.getByRole('tab', { name: '供应商排名' })).toBeVisible();
    await expect(page.getByRole('tab', { name: '评估指标' })).toBeVisible();
  });

  test('07-02 供应商排名 Tab 可刷新并渲染排名表', async ({ page }) => {
    await page.goto('/supplier-evaluation');
    await page.getByRole('tab', { name: '供应商排名' }).click();
    const table = page.getByRole('table', { name: '供应商排名列表' });
    await expect(table).toBeVisible();
    // 列头真实存在
    await expect(table.getByText('排名').first()).toBeVisible();
    await expect(table.getByText('评级').first()).toBeVisible();
    // 点击 '刷新排名' 触发后端排名查询，不报错且表格仍在
    await page.getByRole('button', { name: '刷新排名' }).click();
    await expect(table).toBeVisible();
  });

  test('07-03 评估指标 Tab 渲染指标编码列', async ({ page }) => {
    await page.goto('/supplier-evaluation');
    await page.getByRole('tab', { name: '评估指标' }).click();
    await expect(page.getByText('指标编码').first()).toBeVisible();
    await expect(page.getByText('指标名称').first()).toBeVisible();
  });

  test('07-04 评估记录 Tab 提供新建评估入口', async ({ page }) => {
    await page.goto('/supplier-evaluation');
    // '新建评估' 按钮在“评估记录”“评估管理”两个工具栏各有一个；默认停“评估记录” Tab，取其首个可见者
    await expect(page.getByRole('button', { name: '新建评估' }).first()).toBeVisible();
  });
});
