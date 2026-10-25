/**
 * 公告投递数键名与消费形态锁（防止编造驼峰键与 ?? 兜底掩盖回退）。
 * 背景：后端投递结果的送达数字段是 snake_case 且恒出键，前端一度声明为驼峰假键并
 * 在消费侧用 ?? 0 兜底，导致投递数恒显示 0、把契约错静默掩盖。
 * 功能：pin 住两处——接口类型用后端真实键 delivered_count 且非可选；消费视图直读真值，
 * 取不到即显式失败，禁止 ?? 0 静默兜底。
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

function readRepoSource(absPath: string): string {
  return readFileSync(absPath, 'utf8').replace(/\r\n/g, '\n');
}

const NOTIFICATION_API_SRC = readRepoSource(path.join(FRONTEND_ROOT, 'src/api/notification.ts'));
const NOTIFICATION_PAGE_SRC = readRepoSource(
  path.join(FRONTEND_ROOT, 'src/views/notification/index.vue')
);

describe('公告投递数键名与消费形态锁', () => {
  it('接口类型：AnnouncementResult 使用后端真实键 delivered_count 且非可选，驼峰假键不得回流', () => {
    expect(NOTIFICATION_API_SRC).toMatch(/interface AnnouncementResult \{/);
    expect(NOTIFICATION_API_SRC).toContain('delivered_count: number;');
    expect(NOTIFICATION_API_SRC, '编造驼峰假键一旦回流即恒 undefined').not.toMatch(
      /deliveredCount/
    );
  });

  it('消费视图：直读 delivered_count 真值并做类型判定，缺失走显式失败而非 ?? 0 兜底', () => {
    expect(NOTIFICATION_PAGE_SRC).toContain('res.data?.delivered_count');
    expect(NOTIFICATION_PAGE_SRC, '兜底成 0 会掩盖契约错，禁止回流').not.toMatch(
      /delivered_count\s*\?\?\s*0/
    );
    expect(NOTIFICATION_PAGE_SRC).toContain(
      'system.notification.announce.message.deliveryCountMissing'
    );
  });

  it('检测力自证：把 ?? 0 兜底塞回的夹具必须被 not 判据抓到', () => {
    const mutated = NOTIFICATION_PAGE_SRC.replace(
      'const count = res.data?.delivered_count;',
      'const count = res.data?.delivered_count ?? 0;'
    );
    expect(mutated, '夹具替换本身必须生效').not.toBe(NOTIFICATION_PAGE_SRC);
    expect(mutated, '兜底回流必须被 not 判据抓到').toMatch(/delivered_count\s*\?\?\s*0/);
  });
});
