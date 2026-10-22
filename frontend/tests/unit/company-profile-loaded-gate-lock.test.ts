/**
 * 公司档案保存门闸形态锁（防止全量替换端点在"读档失败"下被空表单清空）。
 * 背景：保存端点是全量替换语义、档案表为单例行无回退，读档失败时表单停在除名称
 * 外全 null 的初值，仅凭名称必填拦不住，一旦放行 PUT 会静默清空全部可空列。
 * 功能：pin 住消费视图里的已加载门闸链路——拉取进入即撤回标记、出参契约校验不过
 * 即抛错、成功且校验通过才置真；保存发 PUT 之前必须先判该标记，判否即点名拒发。
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅 fs 读取仓库内前端源码文本，不执行组件、不发网络请求。
 * 传给谁：纯断言，无下游；存什么/存哪里：不落盘、不存储。
 */
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { describe, expect, it } from 'vitest';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const FRONTEND_ROOT = path.resolve(HERE, '../..');

/**
 * 读仓库内源码并统一行尾为 LF：Windows 检出为 CRLF、CI 检出为 LF，
 * 不归一会让逐行正则在两种环境下判据不一致。
 */
function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const COMPANY_PAGE_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/views/company/index.vue'));

describe('公司档案保存门闸形态锁', () => {
  it('声明已加载标记初值为 false：拉取成功前不得默认为已加载', () => {
    expect(COMPANY_PAGE_SRC).toMatch(/const profileLoaded = ref\(false\)/);
  });

  it('拉取链路：进入即撤回标记、出参契约校验不过即抛错，成功且校验通过才置真', () => {
    const retractIdx = COMPANY_PAGE_SRC.indexOf('profileLoaded.value = false;');
    const validateIdx = COMPANY_PAGE_SRC.indexOf('typeof f.updated_by ===');
    const markTrueIdx = COMPANY_PAGE_SRC.indexOf('profileLoaded.value = true;');
    expect(retractIdx, '缺少拉取进入即撤回已加载标记').toBeGreaterThanOrEqual(0);
    expect(validateIdx, '缺少对档案出参的契约校验').toBeGreaterThanOrEqual(0);
    expect(markTrueIdx, '缺少拉取成功后置已加载').toBeGreaterThanOrEqual(0);
    expect(retractIdx, '撤回标记必须先于置真（否则陈旧 true 会误放行保存）').toBeLessThan(
      markTrueIdx
    );
    expect(validateIdx, '契约校验必须先于置真').toBeLessThan(markTrueIdx);
    // 校验须覆盖关键非空列类型（不止 name），否则"HTTP 200 但畸形出参"会被误判为已加载
    expect(COMPANY_PAGE_SRC).toContain('typeof f.id ===');
    expect(COMPANY_PAGE_SRC).toContain('typeof f.name ===');
    expect(COMPANY_PAGE_SRC).toContain('isRealProfile');
  });

  it('保存门闸：发 PUT 之前必须先判已加载标记，判否即点名拒发', () => {
    const gateIdx = COMPANY_PAGE_SRC.indexOf('if (!profileLoaded.value)');
    const putIdx = COMPANY_PAGE_SRC.indexOf('updateCompanyProfile(');
    expect(gateIdx, '保存前缺少未加载门闸').toBeGreaterThanOrEqual(0);
    expect(putIdx, '保存链路应发全量替换 PUT').toBeGreaterThanOrEqual(0);
    expect(gateIdx, '门闸必须先于 PUT 调用').toBeLessThan(putIdx);
    expect(COMPANY_PAGE_SRC).toContain('system.company.message.notLoadedSaveBlocked');
  });

  it('检测力自证：把门闸拆成恒不成立的夹具必须使门闸判据失去命中', () => {
    const withoutGate = COMPANY_PAGE_SRC.replace('if (!profileLoaded.value) {', 'if (false) {');
    expect(withoutGate, '夹具替换本身必须生效').not.toBe(COMPANY_PAGE_SRC);
    expect(withoutGate.indexOf('if (!profileLoaded.value)'), '门闸被拆掉后判据必须失去命中').toBe(
      -1
    );
  });
});
