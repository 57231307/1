// 质量管理 E2E 套件 — 03 四分制评分数值断言
// 覆盖范围：创建验布单(four_point) → 开始 → 添加疵点（验证自动扣分）→ 评级（验证等级公式）
// 四分制规则（AATCC D5430）：
//   疵点长度 ≤3寸=1分, 3-6寸=2分, 6-9寸=3分, >9寸=4分, 破洞/连续性=4分（不论长度）
// 等级公式：每百平方码分数 = (总扣分 * 36 * 100) / (受检码数 * 幅宽英寸)
//   ≤40 → first（首级），>40 → second（次级）
import { test, expect } from '@playwright/test';
import { applyAuthMocks } from '../smoke/_helpers';
import { apiCall, apiCallRaw, genCode, tryCleanup } from '../flow/helpers';

const CLEANUP: Array<{ path: string; label: string }> = [];
test.afterEach(async ({ page }) => {
  for (const c of CLEANUP.reverse()) await tryCleanup(page, 'DELETE', c.path, c.label);
  CLEANUP.length = 0;
});

interface InspectionModel {
  id: number;
  inspection_no: string;
  status: string;
  scoring_system: string;
  fabric_width_inches: string | number | null;
  inspected_yards: string | number;
  total_defect_points: number;
  points_per_100_sq_yards: string | number | null;
  grade: string | null;
}

interface DefectModel {
  id: number;
  inspection_id: number;
  defect_type: string;
  defect_length_inches: string | number;
  is_hole: boolean;
  is_continuous: boolean;
  points: number;
}

test.describe('03 四分制评分数值断言', () => {
  test.beforeEach(async ({ page, context }) => {
    await applyAuthMocks(context);
    await page.goto('/');
  });

  test('03-01 疵点扣分按长度分档：≤3寸=1 / 3-6寸=2 / 6-9寸=3 / >9寸=4', async ({ page }) => {
    // 创建四分制验布单（幅宽 60 英寸）
    const inspection = await apiCallRaw<InspectionModel>(
      page,
      'POST',
      '/production/fabric-inspections',
      {
        inspection_date: new Date().toISOString().slice(0, 10),
        scoring_system: 'four_point',
        fabric_width_inches: '60.00',
        product_name: 'E2E四分制测试布',
        remarks: genCode('E2E-FP'),
      }
    );
    expect(inspection.id, '创建验布记录应返回 id').toBeTruthy();
    CLEANUP.push({ path: `/production/fabric-inspections/${inspection.id}`, label: 'inspection' });

    // 开始验布
    await apiCall(page, 'POST', `/production/fabric-inspections/${inspection.id}/start`);

    // 添加 4 个疵点覆盖各分档
    const testCases = [
      { length: '2.0', expectedPoints: 1, desc: '2寸→1分' },
      { length: '5.0', expectedPoints: 2, desc: '5寸→2分' },
      { length: '8.0', expectedPoints: 3, desc: '8寸→3分' },
      { length: '12.0', expectedPoints: 4, desc: '12寸→4分' },
    ];

    for (const tc of testCases) {
      const defect = await apiCallRaw<DefectModel>(page, 'POST', '/production/fabric-defects', {
        inspection_id: inspection.id,
        defect_type: 'broken_end',
        position_yards: '10.00',
        defect_length_inches: tc.length,
        direction: 'warp',
        is_hole: false,
        is_continuous: false,
        is_half_width: false,
      });
      expect(
        defect.points,
        `疵点长度 ${tc.length}寸 应扣 ${tc.expectedPoints} 分（${tc.desc}），实际 ${defect.points}`
      ).toBe(tc.expectedPoints);
      CLEANUP.push({ path: `/production/fabric-defects/${defect.id}`, label: 'defect' });
    }
  });

  test('03-02 破洞不论长度一律 4 分', async ({ page }) => {
    const inspection = await apiCallRaw<InspectionModel>(
      page,
      'POST',
      '/production/fabric-inspections',
      {
        inspection_date: new Date().toISOString().slice(0, 10),
        scoring_system: 'four_point',
        fabric_width_inches: '60.00',
        product_name: 'E2E破洞测试',
        remarks: genCode('E2E-HOLE'),
      }
    );
    CLEANUP.push({ path: `/production/fabric-inspections/${inspection.id}`, label: 'inspection' });
    await apiCall(page, 'POST', `/production/fabric-inspections/${inspection.id}/start`);

    // 破洞 1 寸（本应 1 分），因 is_hole=true → 4 分
    const defect = await apiCallRaw<DefectModel>(page, 'POST', '/production/fabric-defects', {
      inspection_id: inspection.id,
      defect_type: 'hole',
      position_yards: '5.00',
      defect_length_inches: '1.00',
      direction: 'warp',
      is_hole: true,
      is_continuous: false,
      is_half_width: false,
    });
    expect(defect.points, `破洞不论长度应扣 4 分，实际 ${defect.points}`).toBe(4);
    CLEANUP.push({ path: `/production/fabric-defects/${defect.id}`, label: 'defect' });
  });

  test('03-03 连续性疵点不论长度一律 4 分', async ({ page }) => {
    const inspection = await apiCallRaw<InspectionModel>(
      page,
      'POST',
      '/production/fabric-inspections',
      {
        inspection_date: new Date().toISOString().slice(0, 10),
        scoring_system: 'four_point',
        fabric_width_inches: '45.00',
        product_name: 'E2E连续性测试',
        remarks: genCode('E2E-CONT'),
      }
    );
    CLEANUP.push({ path: `/production/fabric-inspections/${inspection.id}`, label: 'inspection' });
    await apiCall(page, 'POST', `/production/fabric-inspections/${inspection.id}/start`);

    // 连续性疵点 2 寸（本应 1 分），因 is_continuous=true → 4 分
    const defect = await apiCallRaw<DefectModel>(page, 'POST', '/production/fabric-defects', {
      inspection_id: inspection.id,
      defect_type: 'streak',
      position_yards: '3.00',
      defect_length_inches: '2.00',
      direction: 'weft',
      is_hole: false,
      is_continuous: true,
      is_half_width: false,
    });
    expect(defect.points, `连续性疵点不论长度应扣 4 分，实际 ${defect.points}`).toBe(4);
    CLEANUP.push({ path: `/production/fabric-defects/${defect.id}`, label: 'defect' });
  });

  test('03-04 评级等级判定公式：p100≤40→first / p100>40→second', async ({ page }) => {
    // 公式：每百平方码分数 = (总扣分 * 36 * 100) / (受检码数 * 幅宽英寸)
    // 设计两组用例：一组 first，一组 second

    // --- 组1: 应判 first ---
    // 幅宽 60 英寸, 受检 100 码, 总扣分 = 5 (5个疵点各1分)
    // p100 = (5 * 3600) / (100 * 60) = 18000 / 6000 = 3.0 ≤ 40 → first
    const insp1 = await apiCallRaw<InspectionModel>(
      page,
      'POST',
      '/production/fabric-inspections',
      {
        inspection_date: new Date().toISOString().slice(0, 10),
        scoring_system: 'four_point',
        fabric_width_inches: '60.00',
        product_name: 'E2E首级测试',
        remarks: genCode('E2E-FIRST'),
      }
    );
    CLEANUP.push({ path: `/production/fabric-inspections/${insp1.id}`, label: 'inspection_first' });
    await apiCall(page, 'POST', `/production/fabric-inspections/${insp1.id}/start`);

    // 添加 5 个各 1 寸（≤3寸→1分）的疵点 → 总扣分 = 5
    for (let i = 0; i < 5; i++) {
      const d = await apiCallRaw<DefectModel>(page, 'POST', '/production/fabric-defects', {
        inspection_id: insp1.id,
        defect_type: 'oil_stain',
        position_yards: `${(i + 1) * 5}.00`,
        defect_length_inches: '1.00',
        direction: 'warp',
        is_hole: false,
        is_continuous: false,
        is_half_width: false,
      });
      CLEANUP.push({ path: `/production/fabric-defects/${d.id}`, label: 'defect' });
    }

    const graded1 = await apiCallRaw<InspectionModel>(
      page,
      'POST',
      `/production/fabric-inspections/${insp1.id}/grade`,
      { inspected_yards: '100.00', qualification_rate: '95' }
    );
    // p100 = (5*3600)/(100*60) = 3.0
    expect(graded1.total_defect_points, '组1 总扣分应为 5').toBe(5);
    expect(
      Number(graded1.points_per_100_sq_yards),
      `组1 每百平方码分数应≈3.0，实际: ${graded1.points_per_100_sq_yards}`
    ).toBeCloseTo(3.0, 1);
    expect(graded1.grade, `组1 p100≤40 应判 first，实际: ${graded1.grade}`).toBe('first');

    // --- 组2: 应判 second ---
    // 幅宽 45 英寸, 受检 50 码, 总扣分 = 40 (10个疵点各4分)
    // p100 = (40 * 3600) / (50 * 45) = 144000 / 2250 = 64.0 > 40 → second
    const insp2 = await apiCallRaw<InspectionModel>(
      page,
      'POST',
      '/production/fabric-inspections',
      {
        inspection_date: new Date().toISOString().slice(0, 10),
        scoring_system: 'four_point',
        fabric_width_inches: '45.00',
        product_name: 'E2E次级测试',
        remarks: genCode('E2E-SECOND'),
      }
    );
    CLEANUP.push({
      path: `/production/fabric-inspections/${insp2.id}`,
      label: 'inspection_second',
    });
    await apiCall(page, 'POST', `/production/fabric-inspections/${insp2.id}/start`);

    // 添加 10 个破洞（各 4 分）→ 总扣分 = 40
    for (let i = 0; i < 10; i++) {
      const d = await apiCallRaw<DefectModel>(page, 'POST', '/production/fabric-defects', {
        inspection_id: insp2.id,
        defect_type: 'hole',
        position_yards: `${(i + 1) * 3}.00`,
        defect_length_inches: '1.00',
        direction: 'warp',
        is_hole: true,
        is_continuous: false,
        is_half_width: false,
      });
      CLEANUP.push({ path: `/production/fabric-defects/${d.id}`, label: 'defect' });
    }

    const graded2 = await apiCallRaw<InspectionModel>(
      page,
      'POST',
      `/production/fabric-inspections/${insp2.id}/grade`,
      { inspected_yards: '50.00', qualification_rate: '70' }
    );
    // p100 = (40*3600)/(50*45) = 64.0
    expect(graded2.total_defect_points, '组2 总扣分应为 40').toBe(40);
    expect(
      Number(graded2.points_per_100_sq_yards),
      `组2 每百平方码分数应≈64.0，实际: ${graded2.points_per_100_sq_yards}`
    ).toBeCloseTo(64.0, 0);
    expect(graded2.grade, `组2 p100>40 应判 second，实际: ${graded2.grade}`).toBe('second');
  });
});
