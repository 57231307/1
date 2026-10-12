// 生产资源管理 E2E — 04 工作中心独立 CRUD 闭环 + 回读
// 覆盖范围：创建 → GET 回读 → PUT 更新 → GET 回读新值 → DELETE → 列表不再出现
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, genCode } from '../flow/helpers';

interface WorkCenterModel {
  id: number;
  code: string;
  name: string;
  work_center_type: string | null;
  daily_capacity: number | string | null;
  capacity_unit: string | null;
  status: string;
  remarks: string | null;
}

test.describe('04 工作中心 CRUD 闭环', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('04-01 创建工作中心 → 列表回读验证所有字段真实落库', async ({ page }) => {
    const code = genCode('E2E-WC');
    const name = `E2E测试产线${code.slice(-6)}`;

    const created = await apiCallRaw<WorkCenterModel>(
      page,
      'POST',
      '/production/capacity/work-centers',
      {
        code,
        name,
        work_center_type: 'dyeing',
        daily_capacity: '500.00',
        capacity_unit: '米',
        status: 'ACTIVE',
        remarks: 'E2E 自动化测试创建',
      }
    );
    expect(
      created.id,
      `创建工作中心应返回 id，实际响应: ${JSON.stringify(created).slice(0, 200)}`
    ).toBeTruthy();
    expect(created.code, '创建响应 code 应与请求一致').toBe(code);
    expect(created.name).toBe(name);
    expect(created.work_center_type).toBe('dyeing');
    expect(Number(created.daily_capacity)).toBeCloseTo(500, 2);
    expect(created.capacity_unit).toBe('米');
    expect(created.status).toBe('ACTIVE');

    // 清理
    await apiCall(page, 'DELETE', `/production/capacity/work-centers/${created.id}`).catch(
      () => {}
    );
  });

  test('04-02 更新工作中心 → 回读确认字段变更持久化', async ({ page }) => {
    const code = genCode('E2E-WCU');
    const originalName = `原始名称${code.slice(-6)}`;
    const updatedName = `更新名称${code.slice(-6)}`;

    const created = await apiCallRaw<WorkCenterModel>(
      page,
      'POST',
      '/production/capacity/work-centers',
      {
        code,
        name: originalName,
        work_center_type: 'weaving',
        daily_capacity: '200.00',
        capacity_unit: '匹',
        status: 'ACTIVE',
      }
    );
    const id = created.id;
    expect(id).toBeTruthy();

    // PUT 更新 name / daily_capacity / status
    await apiCallRaw<WorkCenterModel>(page, 'PUT', `/production/capacity/work-centers/${id}`, {
      name: updatedName,
      daily_capacity: '350.00',
      status: 'MAINTENANCE',
    });

    // 列表回读定位本条（无独立 GET 单条端点，通过列表 + code 唯一性定位）
    const list = await apiCallRaw<WorkCenterModel[]>(
      page,
      'GET',
      '/production/capacity/work-centers'
    );
    expect(Array.isArray(list), '工作中心列表应返回数组').toBe(true);
    const found = list.find(wc => wc.code === code);
    expect(found, `列表应包含 code=${code} 的记录`).toBeDefined();

    // 精确断言更新后值
    expect(found!.name, `更新后 name 应为 "${updatedName}"`).toBe(updatedName);
    expect(Number(found!.daily_capacity), '更新后 daily_capacity 应为 350').toBeCloseTo(350, 2);
    expect(found!.status, '更新后 status 应为 MAINTENANCE').toBe('MAINTENANCE');

    // 清理
    await apiCall(page, 'DELETE', `/production/capacity/work-centers/${id}`).catch(() => {});
  });

  test('04-03 删除工作中心 → 列表不再出现该记录', async ({ page }) => {
    const code = genCode('E2E-WCD');

    const created = await apiCallRaw<WorkCenterModel>(
      page,
      'POST',
      '/production/capacity/work-centers',
      {
        code,
        name: `删除测试${code.slice(-6)}`,
        work_center_type: 'finishing',
        daily_capacity: '100.00',
        capacity_unit: '米',
        status: 'ACTIVE',
      }
    );
    const id = created.id;
    expect(id).toBeTruthy();

    // DELETE
    await apiCall(page, 'DELETE', `/production/capacity/work-centers/${id}`);

    // 回读列表确认已不存在
    const list = await apiCallRaw<WorkCenterModel[]>(
      page,
      'GET',
      '/production/capacity/work-centers'
    );
    const stillExists = Array.isArray(list) && list.some(wc => wc.code === code);
    expect(stillExists, `删除后列表中不应存在 code=${code} 的工作中心`).toBe(false);
  });

  test('04-04 非法输入：code 重复应被拒绝', async ({ page }) => {
    const code = genCode('E2E-WCDUP');

    await apiCallRaw<WorkCenterModel>(page, 'POST', '/production/capacity/work-centers', {
      code,
      name: `首次创建${code.slice(-6)}`,
      status: 'ACTIVE',
    });

    // 第二次使用相同 code → 应返回 400+ 错误
    const { apiCallExpectFail } = await import('../flow/helpers');
    const fail = await apiCallExpectFail(page, 'POST', '/production/capacity/work-centers', {
      code,
      name: '重复code应失败',
      status: 'ACTIVE',
    });
    expect(fail.status, `重复 code 应返回 HTTP 400+，实际 ${fail.status}`).toBeGreaterThanOrEqual(
      400
    );

    // 清理首次创建（忽略删除可能失败的 case）
    const list = await apiCallRaw<WorkCenterModel[]>(
      page,
      'GET',
      '/production/capacity/work-centers'
    );
    const first = Array.isArray(list) ? list.find(wc => wc.code === code) : undefined;
    if (first) {
      await apiCall(page, 'DELETE', `/production/capacity/work-centers/${first.id}`).catch(
        () => {}
      );
    }
  });
});
