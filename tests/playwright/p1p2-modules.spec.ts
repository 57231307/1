import { test, expect } from '@playwright/test';
import { recordResult, generateTestId } from './helpers';

const MODULE = 'P1P2模块';

let adminToken = '';

test.beforeAll(async ({ request }) => {
  await new Promise(r => setTimeout(r, 3000));
  const resp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login', {
    data: { username: 'admin', password: 'admin123456' },
    headers: { 'Content-Type': 'application/json' },
  });
  const body = await resp.json();
  adminToken = body?.data?.token || '';
  if (!adminToken) {
    console.warn('P1P2-Login failed, using fallback token');
    const resetResp = await request.post('http://127.0.0.1:8082/api/v1/erp/init/reset-password', {
      data: { username: 'admin', new_password: 'admin123456' },
      headers: { 'Content-Type': 'application/json' },
    });
    const retryResp = await request.post('http://127.0.0.1:8082/api/v1/erp/auth/login', {
      data: { username: 'admin', password: 'admin123456' },
      headers: { 'Content-Type': 'application/json' },
    });
    const retryBody = await retryResp.json();
    adminToken = retryBody?.data?.token || '';
  }
  console.log('P1P2-Token:', adminToken ? 'OK' : 'FAILED');
});

function h() { return { Authorization: `Bearer ${adminToken}`, 'Content-Type': 'application/json' }; }

test.describe('总账管理模块 (GL)', () => {
  test('科目列表查询', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/gl/subjects',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'科目列表查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'科目列表查询','fail',e.message,Date.now()-s); }
  });
  test('科目树查询', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/gl/subjects/tree',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'科目树查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'科目树查询','fail',e.message,Date.now()-s); }
  });
  test('凭证列表查询', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/gl/vouchers',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'凭证列表查询','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'凭证列表查询','fail',e.message,Date.now()-s); }
  });
});

test.describe('应付管理模块 (AP)', () => {
  test('应付发票列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/invoices',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'应付发票列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'应付发票列表','fail',e.message,Date.now()-s); }
  });
  test('应付付款列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/payments',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'应付付款列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'应付付款列表','fail',e.message,Date.now()-s); }
  });
  test('付款申请列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/payment-requests',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'付款申请列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'付款申请列表','fail',e.message,Date.now()-s); }
  });
  test('核销列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/verifications',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'核销列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'核销列表','fail',e.message,Date.now()-s); }
  });
  test('对账列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/reconciliations',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'对账列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'对账列表','fail',e.message,Date.now()-s); }
  });
  test('应付统计报表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/reports/statistics',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'应付统计报表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'应付统计报表','fail',e.message,Date.now()-s); }
  });
  test('应付日报', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/reports/daily',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'应付日报','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'应付日报','fail',e.message,Date.now()-s); }
  });
  test('应付月报', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/reports/monthly',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'应付月报','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'应付月报','fail',e.message,Date.now()-s); }
  });
  test('应付账龄分析', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ap/invoices/aging',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'应付账龄分析','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'应付账龄分析','fail',e.message,Date.now()-s); }
  });
});

test.describe('应收管理模块 (AR)', () => {
  test('应收发票列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/ar/invoices',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'应收发票列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'应收发票列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('成本管理模块 (Cost)', () => {
  test('成本归集列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/cost-collections',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'成本归集列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'成本归集列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('预算管理模块 (Budget)', () => {
  test('预算列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/budgets',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'预算列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'预算列表','fail',e.message,Date.now()-s); }
  });
  test('预算项目列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/budgets/items',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'预算项目列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'预算项目列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('固定资产模块 (Fixed Assets)', () => {
  test('固定资产列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/fixed-assets',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'固定资产列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'固定资产列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('客户信用模块 (Customer Credit)', () => {
  test('客户信用列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/customer-credits',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'客户信用列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'客户信用列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('资金管理模块 (Fund)', () => {
  test('资金账户列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/fund-management/accounts',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'资金账户列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'资金账户列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('质量检验模块 (Quality)', () => {
  test('质量标准列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/quality-inspections/standards',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'质量标准列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'质量标准列表','fail',e.message,Date.now()-s); }
  });
  test('检验记录列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/quality-inspections/records',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'检验记录列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'检验记录列表','fail',e.message,Date.now()-s); }
  });
  test('缺陷列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/quality-inspections/defects',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'缺陷列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'缺陷列表','fail',e.message,Date.now()-s); }
  });
  test('质量标准API', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/quality-standards',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'质量标准API','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'质量标准API','fail',e.message,Date.now()-s); }
  });
});

test.describe('财务分析模块 (Financial Analysis)', () => {
  test('财务分析报表列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/financial-analysis/reports',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'财务分析报表列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'财务分析报表列表','fail',e.message,Date.now()-s); }
  });
  test('财务趋势', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/financial-analysis/trends',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'财务趋势','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'财务趋势','fail',e.message,Date.now()-s); }
  });
});

test.describe('销售分析模块 (Sales Analysis)', () => {
  test('销售统计', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales-analysis/statistics',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'销售统计','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'销售统计','fail',e.message,Date.now()-s); }
  });
  test('销售趋势', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales-analysis/trends',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'销售趋势','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'销售趋势','fail',e.message,Date.now()-s); }
  });
  test('销售排名', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales-analysis/rankings',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'销售排名','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'销售排名','fail',e.message,Date.now()-s); }
  });
});

test.describe('销售价格模块 (Sales Price)', () => {
  test('销售价格列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales-prices',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'销售价格列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'销售价格列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('采购价格模块 (Purchase Price)', () => {
  test('采购价格列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/purchase-prices',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'采购价格列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'采购价格列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('合同管理模块', () => {
  test('销售合同列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales-contracts',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'销售合同列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'销售合同列表','fail',e.message,Date.now()-s); }
  });
  test('采购合同列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/purchase-contracts',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'采购合同列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'采购合同列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('供应商评估模块', () => {
  test('供应商评估列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/supplier-evaluations',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'供应商评估列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'供应商评估列表','fail',e.message,Date.now()-s); }
  });
  test('评估指标列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/supplier-evaluations/indicators',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'评估指标列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'评估指标列表','fail',e.message,Date.now()-s); }
  });
  test('供应商排名', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/supplier-evaluations/rankings',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'供应商排名','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'供应商排名','fail',e.message,Date.now()-s); }
  });
});

test.describe('销售退货模块', () => {
  test('销售退货列表', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/sales-returns',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'销售退货列表','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'销售退货列表','fail',e.message,Date.now()-s); }
  });
});

test.describe('会计期间模块', () => {
  test('当前会计期间', async ({ request }) => {
    const s = Date.now();
    try { const r = await request.get('http://127.0.0.1:8082/api/v1/erp/finance/accounting-periods/current',{headers:h()}); expect(r.status()).toBe(200); recordResult(MODULE,'当前会计期间','pass',undefined,Date.now()-s); } catch(e:any) { recordResult(MODULE,'当前会计期间','fail',e.message,Date.now()-s); }
  });
});
