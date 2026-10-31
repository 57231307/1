// check-zizmor-ignore-drift.mjs —— 核对 zizmor 豁免清单与实际命中位点是否仍然一致
// 功能：把"已被登记的豁免"从一次性决定变成每轮都要重新证明的事实，防止豁免面在无人
//       审查的情况下扩大，也防止代码改完后留下永不生效的空豁免条目。
// 调用方：.github/actions/vuln-scan/action.yml 的 zizmor 抑制清单对账步（CI），
//         以及本地 E:/tmp/tools/zizmor_check.sh --drift
// 入参：argv[2]=按默认口径（遵守 .github/zizmor.yml）跑出的 zizmor JSON，
//       argv[3]=带 --no-ignores（无视一切豁免）跑出的 zizmor JSON，
//       argv[4]=.github/zizmor.yml 路径，argv[5]=.github/zizmor-ignore-manifest.json 路径
// 传给谁：判定结果只写标准输出与退出码，不落盘
// 存什么：ZIZMOR_DRIFT_OK 或逐条点名的 ZIZMOR_DRIFT_FAIL；另输出各豁免位点的实测条数
// 存哪里：不落盘
// 退出码：0 一致，1 漂移（判红），9 入参或解析异常（不得当通过）
import fs from 'node:fs';

// 夹具自证：--self-test 用内存里的三组数据验判定器确实有检出力
const SEV_BLOCK = ['high', 'critical'];

function basenameOf(p) {
  const norm = String(p).replace(/\\/g, '/');
  const parts = norm.split('/');
  return parts[parts.length - 1];
}

function loadFindings(path, label) {
  let data;
  try {
    data = JSON.parse(fs.readFileSync(path, 'utf8'));
  } catch (e) {
    console.log('TOOL_BUG 无法读取 ' + label + '：' + e.message);
    process.exit(9);
  }
  if (!Array.isArray(data)) {
    console.log('TOOL_BUG ' + label + ' 顶层不是发现数组');
    process.exit(9);
  }
  return data;
}

function groupHigh(findings) {
  const map = new Map();
  for (const f of findings) {
    const sev = String(((f.determinations || {}).severity) || '').toLowerCase();
    if (!SEV_BLOCK.includes(sev)) continue;
    const locs = f.locations || [];
    if (!locs.length) {
      console.log('TOOL_BUG 发现 ' + f.ident + ' 没有任何定位信息，无法核对位点');
      process.exit(9);
    }
    const first = locs[0];
    const vp =
      (((first.symbolic || {}).key || {}).Local || {}).verbatim_path || '';
    const row =
      (((first.concrete || {}).location || {}).start_point || {}).row;
    if (typeof row !== 'number') {
      console.log('TOOL_BUG 发现 ' + f.ident + ' 的定位没有行号，无法核对位点');
      process.exit(9);
    }
    const key = f.ident + '@' + basenameOf(vp) + ':' + (row + 1);
    map.set(key, (map.get(key) || 0) + 1);
  }
  return map;
}

function parseConfigIgnores(path) {
  const text = fs
    .readFileSync(path, 'utf8')
    .replace(/\r\n/g, '\n');
  const set = new Set();
  let currentAudit = null;
  let inIgnore = false;
  for (const line of text.split('\n')) {
    if (/^[A-Za-z0-9_-]+:$/.test(line.trim()) && line.startsWith('  ') && !line.startsWith('    ')) {
      currentAudit = line.trim().replace(/:$/, '');
      inIgnore = false;
      continue;
    }
    if (line.trim() === 'ignore:' && currentAudit) {
      inIgnore = true;
      continue;
    }
    if (inIgnore) {
      const m = line.match(/^\s*-\s*(\S.*\S)\s*$/);
      if (m) {
        const v = m[1].replace(/^["']|["']$/g, '');
        set.add(currentAudit + '@' + v);
        continue;
      }
      if (line.trim() && !line.trim().startsWith('#')) inIgnore = false;
    }
  }
  return set;
}

function compare(honoring, noIgnores, cfgSet, manifestMap) {
  const fails = [];
  const honoringHigh = groupHigh(honoring);
  const actual = groupHigh(noIgnores);

  if (honoringHigh.size > 0) {
    const still = [...honoringHigh.entries()]
      .map(([k, n]) => k + '（' + n + ' 条）')
      .join('、');
    fails.push('遵守豁免清单后仍有 high 阻塞项：' + still + '，说明出现未登记的位点');
  }

  const declaredKeys = [...cfgSet].sort();
  const manifestKeys = [...manifestMap.keys()].sort();
  const onlyCfg = declaredKeys.filter((k) => !manifestMap.has(k));
  const onlyMan = manifestKeys.filter((k) => !cfgSet.has(k));
  if (onlyCfg.length) {
    fails.push('.github/zizmor.yml 里有 ' + onlyCfg.length + ' 条豁免未登记进清单文件：' + onlyCfg.join('、'));
  }
  if (onlyMan.length) {
    fails.push('豁免清单文件里有 ' + onlyMan.length + ' 条未写进 zizmor 配置：' + onlyMan.join('、'));
  }

  for (const key of declaredKeys) {
    const n = actual.get(key) || 0;
    if (n === 0) {
      fails.push('豁免条目 ' + key + ' 本轮零命中：该位点已失效，必须删除条目而不是留着兜底');
    }
    const want = manifestMap.get(key);
    if (want !== undefined && want !== n) {
      fails.push(
        '豁免位点 ' + key + ' 实测 ' + n + ' 条与清单登记 ' + want + ' 条不符：新增的同位点发现不得自动享受豁免',
      );
    }
  }

  // 反向核对：实测被抑制的位点必须每一条都在清单里（防止清单外的静默放过）
  const undeclared = [...actual.keys()].filter((k) => !cfgSet.has(k));
  if (undeclared.length) {
    fails.push('存在清单之外的高危位点（必须显式登记并说明理由）：' + undeclared.join('、'));
  }

  return { fails, actual };
}

function selfTest() {
  const mkFinding = (ident, file, row) => ({
    ident,
    determinations: { severity: 'High', confidence: 'Low' },
    locations: [
      {
        symbolic: { key: { Local: { verbatim_path: './.github/' + file } } },
        concrete: { location: { start_point: { row } } },
      },
    ],
  });
  const cfg = new Set(['github-env@action.yml:95', 'cache-poisoning@ci-cd.yml:35']);
  const man = new Map([
    ['github-env@action.yml:95', 2],
    ['cache-poisoning@ci-cd.yml:35', 1],
  ]);
  const base = [mkFinding('github-env', 'action.yml', 94), mkFinding('github-env', 'action.yml', 94), mkFinding('cache-poisoning', 'ci-cd.yml', 34)];
  let pass = 0;
  let fail = 0;
  const check = (name, got, wantRed) => {
    const isRed = got.fails.length > 0;
    if (isRed === wantRed) {
      pass++;
      console.log('  自证 ' + name + ' 通过（判' + (isRed ? '红' : '绿') + '）');
    } else {
      fail++;
      console.log('  自证 ' + name + ' 失败：期望判' + (wantRed ? '红' : '绿') + '，实得' + (isRed ? '红' : '绿') + ' :: ' + got.fails.join(' | '));
    }
  };
  check('一致形态判绿', compare([], base, cfg, man), false);
  const extra = base.concat([mkFinding('cache-poisoning', 'android.yml', 9)]);
  check('新位点未登记判红', compare([mkFinding('cache-poisoning', 'android.yml', 9)], extra, cfg, man), true);
  const staleCfg = new Set(['github-env@action.yml:95', 'cache-poisoning@ci-cd.yml:999']);
  check('失效条目判红', compare([], base, staleCfg, man), true);
  const countDrift = base.concat([mkFinding('cache-poisoning', 'ci-cd.yml', 34)]);
  check('同位点条数变化判红', compare([], countDrift, cfg, man), true);
  const missingFromCfg = new Set(['cache-poisoning@ci-cd.yml:35']);
  check('配置与清单不一致判红', compare([], base, missingFromCfg, man), true);
  console.log('SELFTEST 通过 ' + pass + ' 失败 ' + fail);
  process.exit(fail ? 1 : 0);
}

if (process.argv[2] === '--self-test') selfTest();

const [honoringPath, noIgnoresPath, cfgPath, manPath] = process.argv.slice(2);
if (!honoringPath || !noIgnoresPath || !cfgPath || !manPath) {
  console.log('TOOL_BUG 用法: check-zizmor-ignore-drift.mjs <honoring.json> <no-ignores.json> <zizmor.yml> <manifest.json> | --self-test');
  process.exit(9);
}
let manifest;
try {
  manifest = JSON.parse(fs.readFileSync(manPath, 'utf8'));
} catch (e) {
  console.log('TOOL_BUG 无法读取豁免清单 ' + manPath + '：' + e.message);
  process.exit(9);
}
if (!Array.isArray(manifest)) {
  console.log('TOOL_BUG 豁免清单顶层不是数组（每条须含 audit 与 site 与 count 与 reason）');
  process.exit(9);
}
const manifestMap = new Map();
for (const e of manifest) {
  const key = e.audit + '@' + e.site;
  if (manifestMap.has(key)) {
    console.log('TOOL_BUG 豁免清单里 ' + key + ' 重复登记');
    process.exit(9);
  }
  if (!Number.isInteger(e.count) || e.count < 1) {
    console.log('TOOL_BUG 豁免清单里 ' + key + ' 的 count 不是正整数');
    process.exit(9);
  }
  if (!e.reason) {
    console.log('TOOL_BUG 豁免清单里 ' + key + ' 缺 reason，不许无理由豁免');
    process.exit(9);
  }
  manifestMap.set(key, e.count);
}

const res = compare(
  loadFindings(honoringPath, '遵守豁免的 zizmor 输出'),
  loadFindings(noIgnoresPath, '无视豁免的 zizmor 输出'),
  parseConfigIgnores(cfgPath),
  manifestMap,
);
for (const [key, n] of [...res.actual.entries()].sort()) {
  console.log('豁免位点 ' + key + ' 本轮实测 ' + n + ' 条');
}
if (res.fails.length) {
  for (const m of res.fails) console.log('ZIZMOR_DRIFT_FAIL ' + m);
  process.exit(1);
}
console.log('ZIZMOR_DRIFT_OK 豁免清单与实测位点逐条相符（登记 ' + res.actual.size + ' 个位点）');
