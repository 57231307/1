import { test, Page } from '@playwright/test';
import path from 'path';
import fs from 'fs';
import http from 'http';

const BASE = 'http://127.0.0.1:3000';
const SCREEN = path.resolve(__dirname, 'test-results/console-screenshots2');
fs.mkdirSync(SCREEN, { recursive: true });

function getLoginToken(): Promise<string> {
  return new Promise((resolve, reject) => {
    const data = JSON.stringify({ username: 'admin', password: 'admin123456' });
    const req = http.request({
      hostname: '127.0.0.1', port: 8082, path: '/api/v1/erp/auth/login',
      method: 'POST',
      headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(data) }
    }, res => {
      let body = '';
      res.on('data', c => body += c);
      res.on('end', () => {
        try {
          const json = JSON.parse(body);
          if (json.success && json.data && json.data.token) resolve(json.data.token);
          else reject(new Error(`登录失败: ${body.substring(0, 200)}`));
        } catch (e: any) { reject(new Error(`解析登录失败: ${e.message}`)); }
      });
    });
    req.on('error', e => reject(e));
    req.write(data); req.end();
  });
}

async function shot(page: Page, name: string) {
  const p = path.join(SCREEN, `${name.replace(/[\/\\\s:]/g, '_')}_${Date.now()}.png`);
  await page.screenshot({ path: p, fullPage: true });
  return p;
}

const remainingModules = [
  { name: '资金管理', route: '/fund-management' },
  { name: '固定资产', route: '/fixed-assets' },
  { name: '会计科目', route: '/account-subjects' },
  { name: '记账凭证', route: '/vouchers' },
  { name: '客户信用', route: '/customer-credits' },
];

test.describe('剩余5个模块控制台错误测试', () => {
  test.setTimeout(600000);

  test('测试剩余5个模块', async ({ page }) => {
    const capturedErrors: string[] = [];
    
    page.on('console', msg => {
      const t = msg.type();
      const txt = msg.text();
      if (t === 'error') capturedErrors.push(`[${t}] ${txt.substring(0, 200)}`);
      else if (txt.includes('Error') || txt.includes('unwrapped') || txt.includes('panic'))
        capturedErrors.push(`[${t}] ${txt.substring(0, 200)}`);
    });
    
    page.on('pageerror', err => {
      capturedErrors.push(`[PAGE] ${err.message.substring(0, 200)}`);
    });

    console.log('\n🔑 获取登录token...');
    const token = await getLoginToken();
    console.log('✅ token获取成功');

    await page.goto(BASE, { waitUntil: 'domcontentloaded', timeout: 15000 });
    await page.evaluate((t) => {
      sessionStorage.setItem('auth_token', t);
      sessionStorage.setItem('is_authenticated', 'true');
    }, token);

    for (let i = 0; i < remainingModules.length; i++) {
      const m = remainingModules[i];
      const prevErrCount = capturedErrors.length;

      console.log(`\n${i+1}. [${m.name}] ${m.route}`);
      
      try {
        await page.goto(`${BASE}/#${m.route}`, { waitUntil: 'domcontentloaded', timeout: 15000 });
        await page.waitForTimeout(3000);
      } catch(e: any) {
        console.log(`   ❌ 导航超时: ${e.message}`);
        continue;
      }

      const text = await page.textContent('body') || '';
      const rendered = text.length > 100 || text.includes('秉羲');

      const newConsoleErrs = capturedErrors.slice(prevErrCount);
      
      if (rendered) {
        console.log(`   ✅ 页面渲染成功`);
        try {
          await shot(page, m.name);
        } catch { /* ignore */ }
        
        // 用户交互
        try {
          const btns = page.locator('button');
          const count = await btns.count();
          for (let j = 0; j < Math.min(count, 3); j++) {
            const btn = btns.nth(j);
            if (await btn.isVisible({ timeout: 1000 }).catch(() => false)) {
              const btnText = (await btn.textContent()) || '';
              console.log(`      🖱 点击按钮: "${btnText.trim()}"`);
              await btn.click();
              await page.waitForTimeout(500);
            }
          }
        } catch { /* ignore */ }
      } else {
        console.log(`   ⚠️ 可能未渲染, body长度: ${text.length}`);
      }

      if (newConsoleErrs.length > 0) {
        console.log(`   ❌ 控制台有 ${newConsoleErrs.length} 个错误:`);
        for (const e of newConsoleErrs) console.log(`      • ${e}`);
      }
    }

    console.log(`\n📊 总计控制台错误: ${capturedErrors.length}`);
    if (capturedErrors.length > 0) {
      console.log('错误详情:');
      for (const e of capturedErrors) console.log(`  • ${e}`);
    } else {
      console.log('✅ 所有5个模块无控制台错误!');
    }
  });
});
