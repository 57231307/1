#!/usr/bin/env node
// 工作流第三方 action 引用固定性静态锁。
// 功能：判定 .github/workflows 下全部工作流与 .github/actions 下全部 composite action 里的
//   uses 引用，凡指向第三方仓库者必须是 40 位 commit sha，可变 tag 与短 sha 一律判红。
// 调用方：CI job「🧹 前端静态检查与契约门禁」的「工作流第三方引用固定性检查」步，
//   以及本地推送前门组 scripts 档。
// 入参：无必填参数；--self-test 跑判定器双向自证（变异夹具必须判红、恢复后必须判绿），
//   --root <目录> 指定仓库根，缺省按脚本自身位置回溯两级。
// 传给谁：结果只写 stdout 哨兵行与逐条 HARD 行，由调用方 grep 判红，本脚本自身退出码
//   0 为绿、1 为红、2 为判定器异常，异常不得被当成绿。
// 存什么：不写任何文件，不生成基线，判定完全由被扫文件内容当场算出。
// 存哪里：无持久化。
//
// 为什么可变 tag 必须判红：actions/owner/repo@v4 这类引用由上游仓库单方面重新指向，
//   tag 一旦改名重打或被账号劫持，同一份流水线定义在不同时间会执行不同代码，而本仓
//   的缓存与 runner 镜像都不会提示这一变化。固定到 sha 后引用不可变，升级必须显式改
//   这一行并被本锁的自证夹具兜住。行末保留 # tag 注释只为可读，判定不看注释。
// 为什么本地引用不参与：./.github/actions/* 与 ./.github/workflows/* 是本仓内容，
//   其不可变性由 git 提交本身保证，固定 sha 既不可能也无意义。

import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';

const SELF_TEST = process.argv.includes('--self-test');
const rootIndex = process.argv.indexOf('--root');
const ROOT =
  rootIndex >= 0 ? process.argv[rootIndex + 1] : path.resolve(import.meta.dirname, '..', '..');

const SHA40 = /^[0-9a-f]{40}$/;

const WORKFLOW_DIR = path.join(ROOT, '.github', 'workflows');
const ACTIONS_DIR = path.join(ROOT, '.github', 'actions');

function scanTargets() {
  const targets = [];
  if (fs.existsSync(WORKFLOW_DIR)) {
    for (const name of fs.readdirSync(WORKFLOW_DIR)) {
      if (name.endsWith('.yml') || name.endsWith('.yaml')) {
        targets.push(path.join(WORKFLOW_DIR, name));
      }
    }
  }
  if (fs.existsSync(ACTIONS_DIR)) {
    for (const name of fs.readdirSync(ACTIONS_DIR)) {
      const actionFile = path.join(ACTIONS_DIR, name, 'action.yml');
      if (fs.existsSync(actionFile)) targets.push(actionFile);
    }
  }
  return targets.sort();
}

function findUnpinned(text, file) {
  const findings = [];
  const lines = text.split(/\r?\n/);
  for (let i = 0; i < lines.length; i++) {
    const match = /^\s*(?:-\s+)?uses:\s*(['"]?)([^'"\s]+)\1/.exec(lines[i]);
    if (!match) continue;
    const ref = match[2];
    if (ref.startsWith('./') || ref.startsWith('/')) continue;
    if (!ref.includes('@')) {
      findings.push({ file, line: i + 1, ref, reason: '引用未带 ref 段' });
      continue;
    }
    const sha = ref.slice(ref.lastIndexOf('@') + 1);
    if (!SHA40.test(sha)) {
      findings.push({
        file,
        line: i + 1,
        ref,
        reason: SHA40.test(sha.slice(0, 40)) ? 'sha 非 40 位' : '可变 tag 或短 sha',
      });
    }
  }
  return findings;
}

function runOnce(targets) {
  const findings = [];
  for (const file of targets) {
    const text = fs.readFileSync(file, 'utf8');
    findings.push(...findUnpinned(text, path.relative(ROOT, file).replace(/\\/g, '/')));
  }
  return findings;
}

// 变异夹具：三类必须各自被检出，任一漏检即判定器无检测力，自测判失败。
// 直接改写内存副本再走同一个 findUnpinned，不改生产路径的解析方式，
// 避免"夹具走一条路、生产走另一条路"的自证盲区。
const MUTANTS = [
  {
    name: '可变 tag 形态',
    apply: text => text.replace(/uses:\s*actions/,'uses: actions/checkout@v9'),
    make: text => text.replace(/uses: [^\s]+@[0-9a-f]{40}/, 'uses: actions/checkout@v9'),
    expect: 1,
  },
  {
    name: '短 sha 形态',
    make: text => text.replace(/uses: [^\s]+@[0-9a-f]{40}/, 'uses: actions/checkout@abc123'),
    expect: 1,
  },
  {
    name: '缺 ref 段形态',
    make: text => text.replace(/uses: [^\s]+@[0-9a-f]{40}/, 'uses: actions/checkout'),
    expect: 1,
  },
];

function selfTest(targets) {
  let pass = 0;
  let fail = 0;
  const real = runOnce(targets);
  if (real.length === 0) {
    console.log('SELFTEST baseline=clean detected=0');
  } else {
    console.log(`SELFTEST_WARN baseline 不干净 detected=${real.length}，夹具结论仍可判但基线本身要修`);
  }
  for (const mutant of MUTANTS) {
    const sample = targets.find(t => /uses: [^\s"]+@[0-9a-f]{40}/.test(fs.readFileSync(t, 'utf8')));
    if (!sample) {
      console.log(`SELFTEST_SKIP ${mutant.name} 理由=仓库内无可变异样本`);
      fail++;
      continue;
    }
    const text = fs.readFileSync(sample, 'utf8');
    const mutated = mutant.make(text);
    if (mutated === text) {
      console.log(`SELFTEST_FAIL ${mutant.name} 理由=变异未命中目标串，夹具失效`);
      fail++;
      continue;
    }
    const hits = findUnpinned(mutated, path.relative(ROOT, sample).replace(/\\/g, '/'));
    if (hits.length >= mutant.expect) {
      console.log(`SELFTEST_OK ${mutant.name} detected=${hits.length}`);
      pass++;
    } else {
      console.log(`SELFTEST_FAIL ${mutant.name} detected=0 期望>=${mutant.expect}，判定器对该形态无检测力`);
      fail++;
    }
  }
  return { pass, fail };
}

function main() {
  const targets = scanTargets();
  if (targets.length === 0) {
    console.log(`PINNING_TOOL_BUG 未找到任何工作流文件 root=${ROOT}`);
    process.exitCode = 2;
    return;
  }
  console.log(`PINNING_SCOPE files=${targets.length}`);

  if (SELF_TEST) {
    const { pass, fail } = selfTest(targets);
    console.log(`SELFTEST_RESULT pass=${pass} fail=${fail}`);
    console.log(fail === 0 ? 'SELFTEST GREEN' : 'SELFTEST RED');
    process.exitCode = fail === 0 ? 0 : 1;
    return;
  }

  const findings = runOnce(targets);
  for (const hit of findings) {
    console.log(`HARD | 第三方引用未固定 | ${hit.file}:${hit.line} | ${hit.ref} | ${hit.reason}`);
  }
  console.log(`TOTAL hard=${findings.length} files=${targets.length}`);
  if (findings.length === 0) {
    console.log('PINNING_GREEN');
    process.exitCode = 0;
  } else {
    console.log('PINNING_RED');
    process.exitCode = 1;
  }
}

main();
