// 列表加载空态门 E2E 锁定
// 覆盖 BUG-003，列表页拉取期间不得闪现"暂无数据"空态，须显示加载遮罩，数据到达后才定稿
import { test, expect } from '../diagnose-fixture';
import { loginViaUI, BASE_URL } from './helpers';

test.describe('列表加载空态门', () => {
  test.beforeEach(async ({ page }) => {
    await loginViaUI(page);
  });

  test('加载窗口内不闪现暂无数据且显示加载遮罩', async ({ page }) => {
    // 用受控门把列表请求挂起，制造一个确定可见的加载窗口，避免依赖真实响应快慢
    let releaseGate: () => void;
    const gate = new Promise<void>(resolve => {
      releaseGate = resolve;
    });
    await page.route(
      url => url.pathname.endsWith('/api/v1/erp/audit-logs'),
      async route => {
        await gate;
        await route.fallback();
      }
    );

    // 目标为 V2Table 页（审计日志），其把 useTableApi 的 loading 透传给 V2Table，
    // 正是本次加载空态门控生效的组件；custom-orders 用的是 el-table 不在此组件覆盖内。
    await page.goto(`${BASE_URL}/system/audit-log`);

    // 进入加载窗口，遮罩必须出现，证明 loading 门已生效
    await expect(page.locator('.el-loading-mask').first(), '加载期间应显示加载遮罩').toBeVisible({
      timeout: 8000,
    });
    // 关键断言，数据尚未到达时不得闪现"暂无数据"空态
    await expect(page.getByText(/暂无数据|No Data/), '加载窗口内不应出现暂无数据空态').toHaveCount(
      0
    );

    // 放行请求，数据到达后遮罩应消失，表格定稿
    releaseGate!();
    await expect(page.locator('.el-loading-mask:visible'), '加载结束后遮罩应消失').toHaveCount(0);
  });
});
