/**
 * 移动端首装免配置的启动、连续失败重试与回弹判定回归。
 *
 * 被测两处，一是壳内 gateway 卡页 mobile-app/dist/gateway.js 的纯函数面，
 * 该文件是无构建产物直进 APK 的普通脚本，带 module.exports 通道，本测试经 createRequire
 * 从仓内相对路径载入（不经 vite 解析，绕开工程根外文件的加载限制），
 * jsdom 下其装配函数找不到卡页元素即自行退出，不影响纯函数面。
 * 二是 frontend/src/utils/login-failure.ts 的连续失败计数与阈值判定，
 * 它是远端 ERP 登录页在壳内回弹的同款单点，两侧共用一份口径。
 * 覆盖场景，首装手填，已存地址先探测，探测通过自动进入且先不露地址输入，
 * 探测不通按连续失败计数推进，未计满 5 次只给重试面板不出配置窗，计满 5 次才重新询问，
 * 重新询问与两条回弹路径（reconfigure=1 与卡页超限）都不回填已配置的地址，
 * 探测成功即清零计数，连续 5 次凭证失败或不可达达到登录侧回弹阈值，成功登录清零，跨 host 重计。
 * 卡页是壳内置顶资源，不在 e2e 可达的部署站点上，状态机迁移只能在此单点钉住；
 * 页内交互契约（蓝牙权限用时申请与登录侧回弹时机）由 e2e 87 那条真跑页面。
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

/** 卡页可见视图模型的字段面，与 gateway.js 的 buildConnectView 返回形态一致。 */
interface ConnectView {
  savedVisible: boolean;
  savedText: string;
  formVisible: boolean;
  formValue: string;
  retryVisible: boolean;
  retryText: string;
  errorText: string;
}

interface GatewayApi {
  KEY: string;
  FAIL_KEY: string;
  FAIL_LIMIT: number;
  normalized: (raw: string) => string;
  parseConnectFailures: (raw: string | null) => number;
  nextConnectFailures: (prev: number, outcome: string) => number;
  decideStartup: (input: {
    reconfigureRequested: boolean;
    hasSaved: boolean;
    probeSupported: boolean;
  }) => string;
  decideProbeOutcome: (input: { outcome: string; failures: number }) => string;
  probeMessage: (outcome: string) => string;
  connectHint: (view: string, reason: string) => string;
  buildConnectView: (input: { view: string; saved?: string; message?: string }) => ConnectView;
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

  it('探测通过则自动进入，不通一律不自动跳回旧地址', () => {
    expect(gateway.decideProbeOutcome({ outcome: 'reachable', failures: 1 })).toBe('navigate');
    expect(gateway.decideProbeOutcome({ outcome: 'refused', failures: 5 })).toBe('manual-empty');
    expect(gateway.decideProbeOutcome({ outcome: 'dns', failures: 5 })).toBe('manual-empty');
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

describe('gateway 卡页连续连接失败状态机', () => {
  it('连续失败上限与登录侧回弹阈值同源同值，两处不许各自漂一个数', () => {
    expect(gateway.FAIL_LIMIT).toBe(REOPEN_GATEWAY_AFTER_FAILURES);
    expect(gateway.FAIL_LIMIT).toBe(5);
  });

  it('计数存储值解析，空值与非数字与负数都按 0 次', () => {
    expect(gateway.parseConnectFailures(null)).toBe(0);
    expect(gateway.parseConnectFailures('')).toBe(0);
    expect(gateway.parseConnectFailures('不是数')).toBe(0);
    expect(gateway.parseConnectFailures('-2')).toBe(0);
    expect(gateway.parseConnectFailures('3')).toBe(3);
  });

  it('不可达在原值上加一，可达即归零，连续的定义是中间没有成功过', () => {
    expect(gateway.nextConnectFailures(0, 'refused')).toBe(1);
    expect(gateway.nextConnectFailures(3, 'dns')).toBe(4);
    expect(gateway.nextConnectFailures(4, 'timeout')).toBe(5);
    expect(gateway.nextConnectFailures(4, 'reachable')).toBe(0);
  });

  it('第 1 到第 4 次连不上只给重试面板，配置窗不得出现', () => {
    for (let failures = 1; failures <= 4; failures += 1) {
      expect(
        gateway.decideProbeOutcome({ outcome: 'unreachable', failures }),
        `第 ${failures} 次连不上不得要求重填地址`
      ).toBe('retry');
    }
  });

  it('第 5 次连不上才重新询问，配置窗出现', () => {
    expect(gateway.decideProbeOutcome({ outcome: 'unreachable', failures: 5 })).toBe(
      'manual-empty'
    );
    expect(gateway.decideProbeOutcome({ outcome: 'timeout', failures: 9 })).toBe('manual-empty');
  });

  it('重新询问与首装都不回填已配置的地址，存档地址不进输入框', () => {
    const reask = gateway.buildConnectView({
      view: 'manual-empty',
      saved: 'http://192.168.1.5:3000',
      message: '请重新填写',
    });
    expect(reask.formVisible).toBe(true);
    expect(reask.formValue, '配置窗的地址框必须是空的').toBe('');
    expect(reask.savedVisible, '重新询问时不再显示上次地址卡片').toBe(false);
    const firstInstall = gateway.buildConnectView({ view: 'first-install', saved: '' });
    expect(firstInstall.formVisible).toBe(true);
    expect(firstInstall.formValue).toBe('');
  });

  it('重试面板不出地址表单，探测中不出任何可点入口，存档卡片不进地址框', () => {
    const retry = gateway.buildConnectView({ view: 'retry', saved: 'http://a.test', message: '稍后再试' });
    expect(retry.retryVisible).toBe(true);
    expect(retry.retryText).toBe('稍后再试');
    expect(retry.formVisible, '重试面板不得带出地址输入').toBe(false);
    const probing = gateway.buildConnectView({ view: 'probing', message: '正在连接服务器…' });
    expect(probing.formVisible).toBe(false);
    expect(probing.retryVisible).toBe(false);
    const card = gateway.buildConnectView({ view: 'saved-card', saved: 'http://a.test' });
    expect(card.savedVisible).toBe(true);
    expect(card.savedText).toContain('http://a.test');
    expect(card.formValue, '存档卡片的地址也不写进输入框').toBe('');
  });

  it('提示句只说下一步该做什么，重试与超限两句不同且都不带出已存地址', () => {
    const saved = 'http://192.168.1.5:3000';
    const retryHint = gateway.connectHint('retry', 'refused');
    const reaskHint = gateway.connectHint('manual-empty', 'refused');
    expect(retryHint).not.toBe(reaskHint);
    expect(retryHint, '提示句不得把已存地址显示出来').not.toContain(saved);
    expect(reaskHint).not.toContain(saved);
    expect(gateway.connectHint('retry', ''), '没有探测结论时也要有一句可操作的话').not.toBe('');
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
