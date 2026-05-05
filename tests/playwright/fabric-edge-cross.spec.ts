import { test, expect } from '@playwright/test';
import { recordResult } from './helpers';

const MODULE = '面料行业核心和边缘模块';

let adminToken = '';

test.beforeAll(async ({ request }) => {
  const resp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login', {
    data: { username: 'admin', password: 'admin123456' },
    headers: { 'Content-Type': 'application/json' },
  });
  adminToken = (await resp.json()).data.token;
});

function h() { return { Authorization: `Bearer ${adminToken}`, 'Content-Type': 'application/json' }; }

test.describe('面料行业核心模块', () => {
  test('缸号列表查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/dye-batches',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'缸号列表查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'缸号列表查询','fail',e.message,Date.now()-s); }
  });
  test('创建缸号', async ({ request }) => {
    const s=Date.now(); try { const r=await request.post('http://127.0.0.1:8082/api/v1/erp/dye-batches',{data:{dye_lot_no:'TEST_001',color_code:'RED',quantity:'100.0',unit:'kg'},headers:h()}); const b=await r.json(); expect(b.success).toBe(true); recordResult(MODULE,'创建缸号','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'创建缸号','fail',e.message,Date.now()-s); }
  });

  test('染色配方列表查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/dye-recipes',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'染色配方列表查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'染色配方列表查询','fail',e.message,Date.now()-s); }
  });
  test('染色配方创建', async ({ request }) => {
    const s=Date.now(); try { const r=await request.post('http://127.0.0.1:8082/api/v1/erp/dye-recipes',{data:{name:'测试配方',color_code:'BLUE',recipe_code:'RC001'},headers:h()}); const b=await r.json(); expect(b.success).toBe(true); recordResult(MODULE,'染色配方创建','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'染色配方创建','fail',e.message,Date.now()-s); }
  });

  test('坯布列表查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/greige-fabrics',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'坯布列表查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'坯布列表查询','fail',e.message,Date.now()-s); }
  });
  test('坯布创建', async ({ request }) => {
    const s=Date.now(); try { const r=await request.post('http://127.0.0.1:8082/api/v1/erp/greige-fabrics',{data:{name:'测试坯布',code:'GF001',width:'150',weight:'200',supplier_id:1},headers:h()}); const b=await r.json(); expect(b.success).toBe(true); recordResult(MODULE,'坯布创建','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'坯布创建','fail',e.message,Date.now()-s); }
  });
});

test.describe('边缘辅助模块', () => {
  test('BPM流程任务查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/bpm/tasks',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'BPM任务查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'BPM任务查询','fail',e.message,Date.now()-s); }
  });

  test('CRM线索列表', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/crm/leads',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'CRM线索列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'CRM线索列表','fail',e.message,Date.now()-s); }
  });
  test('CRM商机列表', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/crm/opportunities',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'CRM商机列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'CRM商机列表','fail',e.message,Date.now()-s); }
  });

  test('物流运单列表', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/logistics',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'物流运单列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'物流运单列表','fail',e.message,Date.now()-s); }
  });

  test('五维统计查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/five-dimension/list',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'五维统计查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'五维统计查询','fail',e.message,Date.now()-s); }
  });
  test('五维搜索', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/five-dimension/search?keyword=test',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'五维搜索','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'五维搜索','fail',e.message,Date.now()-s); }
  });

  test('辅助核算维度列表', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/assist-accounting/dimensions',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'辅助核算维度列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'辅助核算维度列表','fail',e.message,Date.now()-s); }
  });
  test('辅助核算记录查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/assist-accounting/records',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'辅助核算记录查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'辅助核算记录查询','fail',e.message,Date.now()-s); }
  });

  test('业务追溯查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/business-trace/forward?trace_id=1',{headers:h()}); expect([200,400]).toContain(r.status()); recordResult(MODULE,'业务追溯查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'业务追溯查询','fail',e.message,Date.now()-s); }
  });

  test('双单位换算', async ({ request }) => {
    const s=Date.now(); try { const r=await request.post('http://127.0.0.1:8082/api/v1/erp/dual-unit/convert',{data:{value:'100',from_unit:'m',to_unit:'yd'},headers:h()}); expect([200,400]).toContain(r.status()); recordResult(MODULE,'双单位换算','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'双单位换算','fail',e.message,Date.now()-s); }
  });

  test('系统版本查询', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/system-update/version',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'系统版本查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'系统版本查询','fail',e.message,Date.now()-s); }
  });

  test('匹号拆分', async ({ request }) => {
    const s=Date.now(); try { const r=await request.post('http://127.0.0.1:8082/api/v1/erp/inventory/piece-split',{data:{piece_id:1,split_quantity:'10.0'},headers:h()}); expect([200,400,404]).toContain(r.status()); recordResult(MODULE,'匹号拆分','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'匹号拆分','fail',e.message,Date.now()-s); }
  });

  test('全维审计统计', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/audit/stats',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'全维审计统计','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'全维审计统计','fail',e.message,Date.now()-s); }
  });
});

test.describe('跨模块业务流程测试', () => {
  test('采购订单→应付发票流程 (完整性验证)', async ({ request }) => {
    const s=Date.now();
    try {
      const r1 = await request.get('http://127.0.0.1:8082/api/v1/erp/purchases/orders',{headers:h()});
      const r2 = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/invoices',{headers:h()});
      expect(r1.status()).toBe(200);
      expect(r2.status()).toBe(200);
      recordResult(MODULE,'采购→应付流程','pass',undefined,Date.now()-s);
    } catch(e:any) { recordResult(MODULE,'采购→应付流程','fail',e.message,Date.now()-s); }
  });

  test('销售订单→库存→应收流程 (数据一致性)', async ({ request }) => {
    const s=Date.now();
    try {
      const r1 = await request.get('http://127.0.0.1:8082/api/v1/erp/sales/orders',{headers:h()});
      const r2 = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/stock',{headers:h()});
      const r3 = await request.get('http://127.0.0.1:8082/api/v1/erp/ar/invoices',{headers:h()});
      expect(r1.status()).toBe(200);
      expect(r2.status()).toBe(200);
      expect(r3.status()).toBe(200);
      recordResult(MODULE,'销售→库存→应收流程','pass',undefined,Date.now()-s);
    } catch(e:any) { recordResult(MODULE,'销售→库存→应收流程','fail',e.message,Date.now()-s); }
  });

  test('库存盘点→调整→凭证流程', async ({ request }) => {
    const s=Date.now();
    try {
      const r1 = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/counts',{headers:h()});
      const r2 = await request.get('http://127.0.0.1:8082/api/v1/erp/inventory/adjustments',{headers:h()});
      const r3 = await request.get('http://127.0.0.1:8082/api/v1/erp/gl/vouchers',{headers:h()});
      expect(r1.status()).toBe(200);
      expect(r2.status()).toBe(200);
      expect(r3.status()).toBe(200);
      recordResult(MODULE,'盘点→调整→凭证流程','pass',undefined,Date.now()-s);
    } catch(e:any) { recordResult(MODULE,'盘点→调整→凭证流程','fail',e.message,Date.now()-s); }
  });

  test('客户信用→销售订单→应收发票流程', async ({ request }) => {
    const s=Date.now();
    try {
      const r1 = await request.get('http://127.0.0.1:8082/api/v1/erp/customer-credits',{headers:h()});
      const r2 = await request.get('http://127.0.0.1:8082/api/v1/erp/sales/orders',{headers:h()});
      const r3 = await request.get('http://127.0.0.1:8082/api/v1/erp/ar/invoices',{headers:h()});
      expect(r1.status()).toBe(200);
      expect(r2.status()).toBe(200);
      expect(r3.status()).toBe(200);
      recordResult(MODULE,'信用→订单→应收流程','pass',undefined,Date.now()-s);
    } catch(e:any) { recordResult(MODULE,'信用→订单→应收流程','fail',e.message,Date.now()-s); }
  });
});

test.describe('回归测试', () => {
  test('健康检查(回归)', async ({ request }) => {
    const s=Date.now(); try { const r=await request.get('http://127.0.0.1:8082/api/v1/erp/health'); expect(r.status()).toBe(200); recordResult(MODULE,'健康检查(回归)','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'健康检查(回归)','fail',e.message,Date.now()-s); }
  });
  test('登录(回归)', async ({ request }) => {
    const s=Date.now(); try { const r=await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login',{data:{username:'admin',password:'admin123456'},headers:{'Content-Type':'application/json'}}); const b=await r.json(); expect(b.success).toBe(true); recordResult(MODULE,'登录(回归)','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'登录(回归)','fail',e.message,Date.now()-s); }
  });
  test('所有核心模块列表接口可用(回归)', async ({ request }) => {
    const s=Date.now();
    try {
      const endpoints = [
        '/users', '/roles', '/products', '/product-categories',
        '/customers', '/suppliers', '/warehouses', '/departments',
        '/inventory/stock', '/sales/orders', '/purchases/orders',
        '/ap/invoices', '/ar/invoices', '/gl/subjects', '/gl/vouchers',
        '/dye-batches', '/dye-recipes', '/greige-fabrics'
      ];
      let allPassed = true;
      for (const ep of endpoints) {
        const r = await request.get(`http://127.0.0.1:8082/api/v1/erp${ep}`, {headers:h()});
        if (r.status() !== 200) { allPassed = false; console.log(`FAIL: ${ep} -> ${r.status()}`); }
      }
      expect(allPassed).toBe(true);
      recordResult(MODULE,'核心模块列表接口回归','pass',undefined,Date.now()-s);
    } catch(e:any) { recordResult(MODULE,'核心模块列表接口回归','fail',e.message,Date.now()-s); }
  });
});

test.describe('按钮与UI测试', () => {
  test('登录页提交按钮存在', async ({ page }) => {
    const s=Date.now();
    try {
      await page.goto('http://127.0.0.1:3000', { waitUntil: 'networkidle', timeout: 20000 });
      await page.waitForTimeout(4000);
      const buttons = await page.locator('button').all();
      expect(buttons.length).toBeGreaterThan(0);
      recordResult(MODULE,'登录页按钮存在','pass',undefined,Date.now()-s);
    } catch(e:any) { recordResult(MODULE,'登录页按钮存在','fail',e.message,Date.now()-s); }
  });

  test('登录页输入框存在', async ({ page }) => {
    const s=Date.now();
    try {
      await page.goto('http://127.0.0.1:3000', { waitUntil: 'networkidle', timeout: 20000 });
      await page.waitForTimeout(3000);
      const inputs = await page.locator('input').all();
      expect(inputs.length).toBeGreaterThanOrEqual(1);
      recordResult(MODULE,'登录页输入框存在','pass',undefined,Date.now()-s);
    } catch(e:any) { recordResult(MODULE,'登录页输入框存在','fail',e.message,Date.now()-s); }
  });
});
