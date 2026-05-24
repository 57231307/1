/**
 * 全面功能测试脚本 - 测试所有 CRUD 操作和按钮功能
 */

const puppeteer = require('puppeteer');
const fs = require('fs');

const BASE_URL = 'http://111.230.99.236';
const LOGIN_CREDENTIALS = { username: 'admin', password: 'admin123' };

// 收集所有错误
const errorReport = {
  api404: [],
  api500: [],
  buttonNotWorking: [],
  saveErrors: [],
  printErrors: [],
  consoleErrors: [],
  pageErrors: []
};

// 需要测试的页面和功能
const pagesToTest = [
  { name: '客户管理', path: '/crm/customers', buttons: ['新增', '编辑', '删除', '查看', '打印', '导出'] },
  { name: '供应商管理', path: '/supplier', buttons: ['新增', '编辑', '删除', '查看'] },
  { name: '产品管理', path: '/product', buttons: ['新增', '编辑', '删除', '查看'] },
  { name: '仓库管理', path: '/warehouse', buttons: ['新增', '编辑', '删除'] },
  { name: '销售管理', path: '/sales', buttons: ['新增', '编辑', '删除', '审核', '打印'] },
  { name: '采购管理', path: '/purchase', buttons: ['新增', '编辑', '删除', '审核'] },
  { name: '库存管理', path: '/inventory', buttons: ['入库', '出库', '调拨', '盘点'] },
  { name: '财务管理', path: '/finance', buttons: ['凭证', '审核', '打印'] },
  { name: '应付管理', path: '/ap', buttons: ['新增', '付款', '核销'] },
  { name: '应收管理', path: '/ar', buttons: ['新增', '收款', '核销'] },
  { name: '系统管理', path: '/system', buttons: ['新增角色', '编辑角色', '删除角色'] },
  { name: '部门管理', path: '/departments', buttons: ['新增', '编辑', '删除'] }
];

async function login(page) {
  await page.goto(`${BASE_URL}/login`, { waitUntil: 'networkidle0', timeout: 30000 });
  await page.waitForSelector('input[type="text"]', { timeout: 10000 });
  await page.type('input[type="text"]', LOGIN_CREDENTIALS.username);
  await page.type('input[type="password"]', LOGIN_CREDENTIALS.password);
  await page.click('button[type="submit"]');
  await page.waitForNavigation({ waitUntil: 'networkidle0', timeout: 30000 });
  await new Promise(r => setTimeout(r, 2000));
}

async function testPage(page, pageConfig) {
  console.log(`\n测试 ${pageConfig.name} (${pageConfig.path})...`);
  
  try {
    await page.goto(`${BASE_URL}${pageConfig.path}`, { waitUntil: 'networkidle0', timeout: 30000 });
    await new Promise(r => setTimeout(r, 3000));
    
    // 监听控制台错误
    page.on('console', msg => {
      if (msg.type() === 'error') {
        errorReport.consoleErrors.push({
          page: pageConfig.name,
          message: msg.text()
        });
      }
    });
    
    // 监听请求失败
    page.on('response', async response => {
      const status = response.status();
      const url = response.url();
      
      if (status === 404) {
        errorReport.api404.push({ page: pageConfig.name, url });
        console.log(`  ❌ 404: ${url}`);
      } else if (status >= 500) {
        errorReport.api500.push({ page: pageConfig.name, url, status });
        console.log(`  ❌ ${status}: ${url}`);
      }
    });
    
    // 检查页面是否有错误
    const pageError = await page.evaluate(() => {
      const errors = [];
      window.addEventListener('error', (e) => errors.push(e.message));
      return errors;
    });
    
    if (pageError.length > 0) {
      errorReport.pageErrors.push({ page: pageConfig.name, errors: pageError });
    }
    
    // 测试按钮存在性和可点击性
    for (const buttonName of pageConfig.buttons) {
      try {
        const button = await page.$(`button:contains("${buttonName}")`);
        if (!button) {
          // 尝试其他选择器
          const altButton = await page.$(`[class*="btn"]`, `[role="button"]`);
          if (!altButton) {
            errorReport.buttonNotWorking.push({
              page: pageConfig.name,
              button: buttonName,
              reason: '按钮不存在'
            });
            console.log(`  ⚠️ 按钮不存在：${buttonName}`);
          }
        }
      } catch (e) {
        errorReport.buttonNotWorking.push({
          page: pageConfig.name,
          button: buttonName,
          reason: e.message
        });
      }
    }
    
    // 测试新增功能
    const addBtn = await page.$('button:contains("新增"), button:contains("添加"), .el-button--primary');
    if (addBtn) {
      try {
        await addBtn.click();
        await new Promise(r => setTimeout(r, 2000));
        
        // 查找表单
        const form = await page.$('form, .el-form, .modal, .dialog');
        if (form) {
          console.log(`  ✓ 打开新增表单成功`);
          
          // 尝试保存
          const saveBtn = await page.$('button:contains("保存"), button:contains("确定"), .el-button--primary');
          if (saveBtn) {
            await saveBtn.click();
            await new Promise(r => setTimeout(r, 3000));
            
            // 检查是否有错误提示
            const errorMsg = await page.$('.el-message--error, .error, [class*="error"]');
            if (errorMsg) {
              const text = await page.evaluate(el => el.textContent, errorMsg);
              errorReport.saveErrors.push({
                page: pageConfig.name,
                action: '新增保存',
                error: text
              });
              console.log(`  ❌ 保存错误：${text}`);
            }
          }
        }
        
        // 关闭弹窗
        await page.keyboard.press('Escape');
        await new Promise(r => setTimeout(r, 1000));
      } catch (e) {
        errorReport.saveErrors.push({
          page: pageConfig.name,
          action: '新增',
          error: e.message
        });
      }
    }
    
    // 测试打印按钮
    const printBtn = await page.$('button:contains("打印"), [class*="print"]');
    if (printBtn) {
      try {
        await printBtn.click();
        await new Promise(r => setTimeout(r, 2000));
        console.log(`  ✓ 打印按钮响应`);
      } catch (e) {
        errorReport.printErrors.push({
          page: pageConfig.name,
          error: e.message
        });
        console.log(`  ❌ 打印按钮无响应`);
      }
    }
    
    // 测试导出按钮
    const exportBtn = await page.$('button:contains("导出"), [class*="export"]');
    if (exportBtn) {
      try {
        await exportBtn.click();
        await new Promise(r => setTimeout(r, 2000));
        console.log(`  ✓ 导出按钮响应`);
      } catch (e) {
        console.log(`  ❌ 导出按钮无响应`);
      }
    }
    
  } catch (e) {
    errorReport.pageErrors.push({
      page: pageConfig.name,
      error: e.message
    });
    console.log(`  ❌ 页面加载失败：${e.message}`);
  }
}

async function main() {
  console.log('=== 全面功能测试开始 ===\n');
  
  const browser = await puppeteer.launch({
    headless: 'new',
    args: ['--no-sandbox', '--disable-setuid-sandbox', '--disable-dev-shm-usage']
  });
  
  const page = await browser.newPage();
  await page.setViewport({ width: 1920, height: 1080 });
  
  // 登录
  console.log('正在登录...');
  await login(page);
  console.log('✓ 登录成功\n');
  
  // 测试每个页面
  for (const pageConfig of pagesToTest) {
    await testPage(page, pageConfig);
  }
  
  await browser.close();
  
  // 生成报告
  console.log('\n=== 测试报告 ===');
  console.log(`\n📊 统计:`);
  console.log(`  404 错误：${errorReport.api404.length}`);
  console.log(`  500 错误：${errorReport.api500.length}`);
  console.log(`  按钮问题：${errorReport.buttonNotWorking.length}`);
  console.log(`  保存错误：${errorReport.saveErrors.length}`);
  console.log(`  打印问题：${errorReport.printErrors.length}`);
  console.log(`  控制台错误：${errorReport.consoleErrors.length}`);
  console.log(`  页面错误：${errorReport.pageErrors.length}`);
  
  // 保存详细报告
  fs.writeFileSync(
    '/workspace/frontend/test_report.json',
    JSON.stringify(errorReport, null, 2)
  );
  console.log(`\n📄 详细报告已保存到：/workspace/frontend/test_report.json`);
  
  // 生成修复建议
  console.log('\n=== 修复建议 ===');
  if (errorReport.api404.length > 0) {
    console.log('\n1. 404 错误修复:');
    const unique404s = [...new Set(errorReport.api404.map(e => e.url))];
    unique404s.forEach(url => console.log(`   - 检查路由：${url}`));
  }
  
  if (errorReport.saveErrors.length > 0) {
    console.log('\n2. 保存错误修复:');
    errorReport.saveErrors.forEach(e => console.log(`   - ${e.page}: ${e.action} - ${e.error}`));
  }
  
  if (errorReport.printErrors.length > 0) {
    console.log('\n3. 打印功能修复:');
    errorReport.printErrors.forEach(e => console.log(`   - ${e.page}: ${e.error}`));
  }
  
  if (errorReport.buttonNotWorking.length > 0) {
    console.log('\n4. 按钮功能修复:');
    errorReport.buttonNotWorking.forEach(e => console.log(`   - ${e.page}: ${e.button} - ${e.reason}`));
  }
  
  console.log('\n=== 测试完成 ===\n');
}

main().catch(console.error);
