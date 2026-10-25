/**
 * 移动端首装免配置的启动与回弹判定回归。
 *
 * 被测两处，一是壳内 gateway 卡页 mobile-app/dist/gateway.js 的纯函数面，
 * 该文件是无构建产物直进 APK 的普通脚本，带 module.exports 通道，本测试经 createRequire
 * 从仓内相对路径载入（不经 vite 解析，绕开工程根外文件的加载限制），
 * jsdom 下其装配函数找不到卡页元素即自行退出，不影响纯函数面。
 * 二是 frontend/src/utils/login-failure.ts 的连续失败计数与阈值判定，
 * 它是远端 ERP 登录页在壳内回弹的同款单点，两侧共用一份口径。
 * 覆盖场景，首装手填，已存地址先探测，探测通过自动进入且先不露地址输入，
 * 探测不通回空表单不自动跳回，两条失败路径（reconfigure=1 与登录不可达）都不带旧地址，
 * 连续 5 次 401 达到回弹阈值，成功登录清零，跨 host 重计。
 * 传给谁，vitest 报告。存什么，不新增存储，注入的内存假存储仅在本测试内。
 */
import { describe, expect, it } from 'vitest';
import { createRequire } from 'node:module';

import {
  clearLoginFailure,
  invokeReopenGateway,
  LOGIN_FAIL_COUNT_KEY,
  nextLoginFailureRecord,
  parseLoginFailureRecord,
  recordLoginFailure,
  REOPEN_GATEWAY_AFTER_FAILURES,
  shouldReopenGateway,
  type KeyValueStore,
} from '@/utils/login-failure';

interface GatewayApi {
  KEY: string;
  normalized: (raw: string) => string;
  decideStartup: (input: {
    reconfigureRequested: boolean;
    hasSaved: boolean;
    probeSupported: boolean;
  }) => string;
  decideProbeOutcome: (input: { outcome: string }) => string;
  probeMessage: (outcome: string) => string;
  decideSubmit: (input: { valid: boolean; probeSupported: boolean; outcome?: string }) => string;
}

const require = createRequire(import.meta.url);
const gateway = require('../../../mobile-app/dist/gateway.js') as GatewayApi;

/** 内存假存储，实现 KeyValueStore 面，供计数函数注入。 */
function memoryStore(): KeyValueStore & { dump: Record<string, string> } {
  const map = new Map<string, string>();
  return {
    getItem: key => map.get(key) ?? null,
    setItem: (key, value) => {
      map.set(key, value);
    },
    removeItem: key => {
      map.delete(key);
    },
    get dump() {
      return Object.fromEntries(map);
    },
  };
}

describe('gateway 卡页启动视图决策', () => {
  it('首装没有存档走空表单手填', () => {
    expect(
      gateway.decideStartup({ reconfigureRequested: false, hasSaved: false, probeSupported: true })
    ).toBe('first-install');
  });

  it('有存档且桥可用先探测，不在决策前露出地址输入', () => {
    expect(
      gateway.decideStartup({ reconfigureRequested: false, hasSaved: true, probeSupported: true })
    ).toBe('auto-probe');
  });

  it('探测通过则自动进入，不通回空表单不自动跳回', () => {
    expect(gateway.decideProbeOutcome({ outcome: 'reachable' })).toBe('navigate');
    expect(gateway.decideProbeOutcome({ outcome: 'refused' })).toBe('manual-empty');
    expect(gateway.decideProbeOutcome({ outcome: 'dns' })).toBe('manual-empty');
  });

  it('探测失败提示按结论分因，解析失败与端口拒绝不是同一句话', () => {
    const dns = gateway.probeMessage('dns');
    const refused = gateway.probeMessage('refused');
    expect(dns, '解析失败必须给出可辨认的独立提示').not.toBe('');
    expect(refused, '端口拒绝必须给出可辨认的独立提示').not.toBe('');
    expect(dns, '两类失败不得糊成同一句').not.toBe(refused);
    expect(gateway.probeMessage('timeout')).not.toBe('');
    expect(gateway.probeMessage('unreachable')).not.toBe('');
    expect(gateway.probeMessage('reachable'), '通过结论没有提示文案').toBe('');
  });

  it('reconfigure=1 的两条失败路径都强制空表单，即使有存档也不带旧地址', () => {
    expect(
      gateway.decideStartup({ reconfigureRequested: true, hasSaved: true, probeSupported: true })
    ).toBe('manual-empty');
    expect(
      gateway.decideStartup({ reconfigureRequested: true, hasSaved: false, probeSupported: true })
    ).toBe('manual-empty');
  });

  it('无桥的桌面预览保留手动进入上次地址', () => {
    expect(
      gateway.decideStartup({ reconfigureRequested: false, hasSaved: true, probeSupported: false })
    ).toBe('saved-card');
  });

  it('提交时先探测再落库，探测不通不落库不跳转，避免错地址成为下次启动目标', () => {
    expect(gateway.decideSubmit({ valid: false, probeSupported: true })).toBe('invalid');
    expect(gateway.decideSubmit({ valid: true, probeSupported: true, outcome: 'refused' })).toBe(
      'unreachable'
    );
    expect(gateway.decideSubmit({ valid: true, probeSupported: true, outcome: 'dns' })).toBe(
      'unreachable'
    );
    expect(gateway.decideSubmit({ valid: true, probeSupported: true, outcome: 'reachable' })).toBe(
      'navigate'
    );
  });

  it('规范化地址，缺协议补 https，非 http 协议与空值拒绝', () => {
    expect(gateway.normalized('erp.example.com')).toBe('https://erp.example.com');
    expect(gateway.normalized(' http://192.168.1.5:3000/erp/ ')).toBe(
      'http://192.168.1.5:3000/erp/'
    );
    expect(gateway.normalized('javascript:alert(1)')).toBe('');
    expect(gateway.normalized('')).toBe('');
  });
});

describe('登录连续失败计数与回弹阈值', () => {
  it('解析存储值，空值与损坏值与非法计数都按无记录', () => {
    expect(parseLoginFailureRecord(null)).toBeNull();
    expect(parseLoginFailureRecord('不是JSON')).toBeNull();
    expect(parseLoginFailureRecord('{"host":"a","count":0}')).toBeNull();
    expect(parseLoginFailureRecord('{"count":3}')).toBeNull();
    expect(parseLoginFailureRecord('{"host":"a","count":3}')).toEqual({ host: 'a', count: 3 });
  });

  it('同 host 累计，换 host 从 1 重计', () => {
    expect(nextLoginFailureRecord(null, 'https://a.test')).toEqual({
      host: 'https://a.test',
      count: 1,
    });
    expect(nextLoginFailureRecord({ host: 'https://a.test', count: 3 }, 'https://a.test')).toEqual({
      host: 'https://a.test',
      count: 4,
    });
    expect(nextLoginFailureRecord({ host: 'https://a.test', count: 3 }, 'https://b.test')).toEqual({
      host: 'https://b.test',
      count: 1,
    });
  });

  it('recordLoginFailure 写入存储并按次累计，clearLoginFailure 清除', () => {
    const store = memoryStore();
    expect(recordLoginFailure('https://a.test', store).count).toBe(1);
    expect(recordLoginFailure('https://a.test', store).count).toBe(2);
    expect(JSON.parse(store.dump[LOGIN_FAIL_COUNT_KEY])).toEqual({
      host: 'https://a.test',
      count: 2,
    });
    clearLoginFailure(store);
    expect(parseLoginFailureRecord(store.getItem(LOGIN_FAIL_COUNT_KEY))).toBeNull();
  });

  it('第 5 次连续失败达到阈值触发回弹，第 4 次不触发', () => {
    expect(REOPEN_GATEWAY_AFTER_FAILURES).toBe(5);
    expect(shouldReopenGateway({ host: 'https://a.test', count: 4 })).toBe(false);
    expect(shouldReopenGateway({ host: 'https://a.test', count: 5 })).toBe(true);
  });

  it('回弹命令仅在有壳桥时发出，无桥什么都不做', () => {
    const calls: string[] = [];
    const w = window as unknown as { __TAURI__?: unknown };
    delete w.__TAURI__;
    expect(() => invokeReopenGateway()).not.toThrow();

    w.__TAURI__ = {
      core: {
        invoke: (cmd: string) => {
          calls.push(cmd);
          return Promise.resolve(null);
        },
      },
    };
    invokeReopenGateway();
    expect(calls).toEqual(['reopen_gateway']);
    delete w.__TAURI__;
  });
});
