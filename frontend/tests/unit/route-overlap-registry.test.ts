/**
 * 重叠入口登记册锁（vitest，无浏览器、无后端依赖）
 *
 * 功能：把入口重复这笔债变成机器可查的五条不变量。
 *   1. 解析有效：在册叶子数落在合理区间，防解析塌缩成空集后所有断言假绿。
 *   2. 机器可判的重叠必须入账：标题词干相同的每一组路由都要被某条登记项完整覆盖，
 *      新增一条与既有入口同词干的入口而不登记即判红。
 *   3. 登记项自洽：id 唯一、成员不少于两条、disposition 在允许取值内、
 *      待裁项必须带默认建议、已收口项必须给出存活成员。
 *   4. 账不能悬空：除已收口项外，登记成员必须仍在册；同一路径不得出现在两条登记项里。
 *   5. 只准降的棘轮：待裁组数不得超过基线，收一组就下调基线，防止登记册只增不减。
 *
 * 调用方：frontend vitest 单测轨（vitest.config.ts include tests/**），CI ci-frontend-test job 执行。
 * 入参：只读 src/router/index.ts 与本目录登记册，不读写后端、不起浏览器。
 * 传给谁：断言结果只进 vitest 报告。
 * 存什么/存哪里：不落盘、不存储。
 */
import { describe, expect, it } from 'vitest';
import { ROUTER_SRC, collectShellRouteEntries } from './route-source';
import { ROUTE_OVERLAPS, type RouteOverlapEntry } from './route-overlap-registry';

const DISPOSITIONS: RouteOverlapEntry['disposition'][] = [
  'distinct-after-review',
  'pending-sign-off',
  'resolved-by-redirect',
];

/** 待裁组数基线，只准下调：每收口一组就把它改小，涨即判红 */
const PENDING_SIGN_OFF_BASELINE = 7;

const entries = collectShellRouteEntries(ROUTER_SRC);
const routablePaths = new Set(entries.map(entry => entry.path));

const SUFFIXES = [
  '管理', '审批', '列表', '明细', '查询', '分析', '设置', '配置', '中心',
  '高级', '增强版', '概览', '统计', '计划', '记录', '台账', '核验', '360',
];

/** 标题去后缀词干：机器可判的重叠信号，只用于"必须入账"，不用于判定是否重复 */
function titleStem(title: string): string {
  let stem = title.replace(/\s/g, '');
  for (const suffix of SUFFIXES) stem = stem.replace(new RegExp(`${suffix}$`), '');
  return stem;
}

const stemGroups = new Map<string, string[]>();
for (const entry of entries) {
  const stem = titleStem(entry.title);
  if (!stem) continue;
  const group = stemGroups.get(stem) ?? [];
  group.push(entry.path);
  stemGroups.set(stem, group);
}
const collisionGroups = [...stemGroups.entries()].filter(([, paths]) => paths.length > 1);

describe('重叠入口登记册', () => {
  it('解析器有效且登记册非空（防塌缩成空集后假绿）', () => {
    expect(entries.length).toBeGreaterThan(100);
    expect(routablePaths.has('/dashboard')).toBe(true);
    expect(routablePaths.has('/login')).toBe(false);
    expect(ROUTE_OVERLAPS.length).toBeGreaterThan(0);
  });

  it('机器可判的词干碰撞组全部入账', () => {
    const unregistered = collisionGroups
      .filter(([, paths]) => {
        const covered = ROUTE_OVERLAPS.filter(row => paths.every(p => row.members.includes(p)));
        return covered.length === 0;
      })
      .map(([stem, paths]) => `${stem}: ${paths.join(' 与 ')}`);
    expect(
      unregistered,
      '新增与既有入口同词干的路由必须先在登记册里声明处置状态，不许留空'
    ).toEqual([]);
  });

  it('登记项自洽（id 唯一、成员数、状态取值、待裁带建议、收口给存活成员）', () => {
    const ids = ROUTE_OVERLAPS.map(row => row.id);
    expect(ids.sort()).toEqual([...new Set(ids)].sort());

    for (const row of ROUTE_OVERLAPS) {
      expect(row.members.length, `${row.id} 至少两条成员`).toBeGreaterThanOrEqual(2);
      expect(DISPOSITIONS, `${row.id} 处置状态取值非法`).toContain(row.disposition);
      if (row.disposition === 'pending-sign-off') {
        expect(row.proposal, `${row.id} 待裁项必须给出默认建议`).toBeTruthy();
        expect(row.note, `${row.id} 待裁项必须写明依据`).toBeTruthy();
      }
      if (row.disposition === 'resolved-by-redirect') {
        expect(row.survivor, `${row.id} 收口项必须给出存活入口`).toBeTruthy();
        expect(routablePaths.has(row.survivor as string), `${row.id} 存活入口应在册`).toBe(true);
      }
    }
  });

  it('账不悬空（成员在册且同一对不重复登记）', () => {
    for (const row of ROUTE_OVERLAPS) {
      if (row.disposition === 'resolved-by-redirect') continue;
      const stale = row.members.filter(p => !routablePaths.has(p));
      expect(stale, `${row.id} 登记了已不在册的入口，先销账再改路由`).toEqual([]);
    }
    // 枢纽入口可以与多条入口分别重叠（例如财务管理既与凭证管理重叠、又与财务分析同词干），
    // 因此唯一性按"成员配对"判，不按成员判：同一对成员不得登记两次。
    const pairSeen = new Map<string, string>();
    const problems: string[] = [];
    for (const row of ROUTE_OVERLAPS) {
      const unique = [...new Set(row.members)];
      if (unique.length !== row.members.length) problems.push(`${row.id} 成员自身重复`);
      for (let i = 0; i < unique.length; i += 1) {
        for (let j = i + 1; j < unique.length; j += 1) {
          const pair = [unique[i], unique[j]].sort().join(' 与 ');
          if (pairSeen.has(pair)) {
            problems.push(`${pair} 同属 ${pairSeen.get(pair)} 与 ${row.id}`);
          } else {
            pairSeen.set(pair, row.id);
          }
        }
      }
    }
    expect(problems).toEqual([]);
  });

  it('待裁组数只准降（棘轮）', () => {
    const pending = ROUTE_OVERLAPS.filter(row => row.disposition === 'pending-sign-off').length;
    expect(
      pending,
      `待裁重复入口组数 ${pending} 超过基线 ${PENDING_SIGN_OFF_BASELINE}，` +
        '新增重叠必须先按不同功能判定或收口，不许把债务往上加'
    ).toBeLessThanOrEqual(PENDING_SIGN_OFF_BASELINE);
  });
});
