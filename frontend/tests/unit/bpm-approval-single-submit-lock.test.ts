/**
 * bpm-approval-single-submit-lock.test.ts - 审批两入口共用唯一提交口的静态契约锁
 *
 * 功能：钉住「流程审批中心对话框办理」与「审批管理页待办行内办理」两个入口
 * 必须经同一个提交函数发出审批请求，且办理后两侧回读的任务状态与任务列表同源。
 *
 * 被钉住的真实缺陷：两入口曾各自拼审批载荷并各指一个写口（中心指
 * POST /bpm/approval/execute，工作台指 POST /bpm/tasks/approve），载荷键、
 * 办理人身份取值与办理后的回读节奏由两份代码各写一遍。任一侧改动都不会传导到
 * 另一侧，表现为同一单据经不同入口办理后任务状态与列表刷新不一致。
 *
 * 判据（正向与反向都要，缺检测力即本锁空转）：
 * 正例：两个入口文件都调用唯一写口的 submitBpmApproval；审批载荷键字面量
 * （task_id 与 handler_id 同时出现）在 src 下只允许存在于该模块；
 * 两个入口文件都不得直接调用底层 API 函数 executeBpmApproval 与 approveBpmTask；
 * 办理成功后中心侧同时刷新待办与已办，工作台侧同时刷新待办与已办。
 * 反例（变异夹具自证）：把「各自拼载荷」的两种历史形态写成内联夹具喂给同一个
 * 探测器，必须全部命中；探测器对当前源码必须零命中。
 * 载荷本身：buildBpmApprovalPayload 的键集合与取值域逐字对齐后端
 * ExecuteApprovalRequest，身份非法或动作域外一律抛错，不静默兜底。
 *
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：仅读仓库内源码文本并调用被测纯函数，不发网络请求、不起浏览器。
 * 传给谁：纯断言，无下游；存什么／存哪里：不落盘、不写缓存。
 */
import { describe, it, expect } from 'vitest';
import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import {
  buildBpmApprovalPayload,
  type BpmApprovalSubmitInput,
} from '@/views/bpm/approval/composables/bpmApprovalSubmit';

const FRONTEND = process.cwd();
const SRC = join(FRONTEND, 'src');
if (!existsSync(join(SRC, 'views'))) {
  throw new Error(`该用例需在 frontend 目录下运行以扫描 src 源码，当前 ${FRONTEND} 内没有 src/views`);
}

/** 唯一写口模块（相对 frontend 的正斜杠路径） */
const SUBMIT_MODULE = 'src/views/bpm/approval/composables/bpmApprovalSubmit.ts';
/** 入口一：流程审批中心（对话框办理） */
const CENTER_ENTRY = 'src/views/bpm/approval/composables/useBpmApProc.ts';
/** 入口二：审批管理页（待办行内快捷办理） */
const WORKBENCH_ENTRY = 'src/views/bpm/index.vue';

function readSrc(relPath: string): string {
  return readFileSync(join(FRONTEND, relPath), 'utf-8');
}

/** 剔除以 `//` 起始的注释行：守卫对象是代码，注释里点名被禁形态是正当用途。 */
function codeOnly(src: string): string {
  return src
    .split(/\r?\n/)
    .filter(l => !l.trimStart().startsWith('//'))
    .filter(l => !l.trimStart().startsWith('<!--'))
    .join('\n');
}

/** 递归收集 src 下的 vue 与 ts 文件（跳过 node_modules 与隐藏目录）。 */
function walkSourceFiles(dir: string, out: string[] = []): string[] {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.name.startsWith('.') || entry.name === 'node_modules') continue;
    const p = join(dir, entry.name);
    if (entry.isDirectory()) walkSourceFiles(p, out);
    else if (p.endsWith('.vue') || p.endsWith('.ts')) out.push(p);
  }
  return out;
}

const rel = (absPath: string): string => relative(FRONTEND, absPath).split(sep).join('/');

/**
 * 探测器一：手拼审批载荷的形态 = 同一份源码里出现 task_id 与 handler_id 两个
 * 键字面量（后端 ExecuteApprovalRequest 的必填键对）。扫描面取 src/views（办理
 * 入口所在面）：api 模块里同名的两处是契约类型声明，不是载荷组装，不纳入本判据。
 * 唯一写口之外任何视图层文件命中，即为「各自拼 payload」回潮。
 */
function handAssembledPayloadFiles(sources: Array<{ path: string; text: string }>): string[] {
  return sources
    .filter(
      s =>
        s.path.startsWith('src/views/') &&
        s.path !== SUBMIT_MODULE &&
        /\btask_id\s*:/.test(s.text) &&
        /\bhandler_id\s*:/.test(s.text)
    )
    .map(s => s.path);
}

/**
 * 探测器二：入口绕过唯一写口、直接调底层审批 API 的形态。
 * 白名单只放「声明该函数的 api 模块」与「唯一写口本身」。
 */
function directApiCallFiles(sources: Array<{ path: string; text: string }>): string[] {
  const allowed = new Set([SUBMIT_MODULE, 'src/api/bpm-enhanced.ts', 'src/api/bpm.ts']);
  return sources
    .filter(
      s =>
        !allowed.has(s.path) &&
        (/\bexecuteBpmApproval\s*\(/.test(s.text) || /\bapproveBpmTask\s*\(/.test(s.text))
    )
    .map(s => s.path);
}

/** 取 const 箭头函数体（从签名到该文件下一个顶层 const 之前）。 */
function constFnBody(src: string, signature: string): string {
  const anchor = src.indexOf(signature);
  if (anchor < 0) throw new Error(`找不到待锁函数：${signature}`);
  const tail = src.slice(anchor + signature.length);
  const next = tail.search(/\n(?:const|function|async function)\s/);
  return next < 0 ? tail : tail.slice(0, next);
}

describe('审批唯一写口：两入口静态形态锁', () => {
  const sources = walkSourceFiles(SRC).map(p => ({
    path: rel(p),
    text: codeOnly(readFileSync(p, 'utf-8')),
  }));

  it('正例：审批载荷键字面量在 src 下只出现在唯一写口模块', () => {
    expect(handAssembledPayloadFiles(sources)).toEqual([]);
  });

  it('正例：没有任何入口绕过唯一写口直接调 executeBpmApproval 或 approveBpmTask', () => {
    expect(directApiCallFiles(sources)).toEqual([]);
  });

  it('正例：两个入口都调用同一个提交函数', () => {
    for (const entry of [CENTER_ENTRY, WORKBENCH_ENTRY]) {
      expect({ file: entry, hits: (readSrc(entry).match(/\bsubmitBpmApproval\s*\(/g) ?? []).length }).toEqual(
        { file: entry, hits: 1 }
      );
    }
    // 入口二必须经由共享模块导入，而不是在本文件内再写一份
    expect(readSrc(WORKBENCH_ENTRY)).toContain(
      "from '@/views/bpm/approval/composables/bpmApprovalSubmit'"
    );
  });

  it('正例：入口二（审批管理页 handleApprove）是第二个办理入口的实际调用点', () => {
    const body = constFnBody(readSrc(WORKBENCH_ENTRY), 'const handleApprove = async (');
    expect(body).toContain('submitBpmApproval(');
    expect(body).toContain("action: 'approve'");
    expect(body).toContain('userStore.userInfo');
    // 办理人身份取登录态，取不到即拒发请求，不以 0 或空串充当办理人
    expect(body).toMatch(/if \(!user\) \{[\s\S]*?\breturn;\s*\}/);
  });

  it('正例：办理后回读与任务列表侧同源——两入口都同时刷新待办与已办', () => {
    const center = constFnBody(readSrc(CENTER_ENTRY), 'const confirmApproval = async () => {');
    expect(center).toContain('refresh.fetchPendingTasks()');
    expect(center).toContain('refresh.fetchCompletedTasks()');
    expect(center).toMatch(
      /await Promise\.all\(\[refresh\.fetchPendingTasks\(\), refresh\.fetchCompletedTasks\(\)\]\)/
    );

    const workbench = constFnBody(readSrc(WORKBENCH_ENTRY), 'const handleApprove = async (');
    expect(workbench).toContain('fetchPendingTasks()');
    expect(workbench).toContain('fetchProcessedTasks()');
    expect(workbench).toMatch(/await Promise\.all\(\[fetchPendingTasks\(\), fetchProcessedTasks\(\)\]\)/);
  });

  it('正例：入口一的办理人同样取登录态，不以 0 兜底 handler_id', () => {
    const center = readSrc(CENTER_ENTRY);
    expect(center).not.toMatch(/handler_id:\s*\w+\?\.id\s*\?\?\s*0/);
    expect(center).not.toMatch(/handlerId:\s*0\b/);
  });

  it('反向（变异夹具自证）：探测器必须抓住两种历史回潮形态', () => {
    // 夹具一 入口自己拼载荷（对应中心侧旧写法：把后端键名直接写进视图层）
    const legacyCenter = `
      const res = await executeBpmApproval({
        task_id: currentTask.value.id,
        handler_id: userStore.userInfo?.id ?? 0,
        handler_name: '',
        action: 'approve',
      });
    `;
    expect(
      handAssembledPayloadFiles([{ path: 'src/views/fixture-legacy-center.ts', text: legacyCenter }])
    ).toEqual(['src/views/fixture-legacy-center.ts']);
    expect(
      directApiCallFiles([{ path: 'src/views/fixture-legacy-center.ts', text: legacyCenter }])
    ).toEqual(['src/views/fixture-legacy-center.ts']);

    // 夹具二 入口另指一个写口（对应工作台侧旧写法：POST /bpm/tasks/approve）
    const legacyWorkbench = `
      await approveBpmTask({
        task_id: row.id,
        handler_id: user.id,
        handler_name: user.real_name,
        action: 'approve',
        approval_opinion: '同意',
      });
    `;
    expect(
      handAssembledPayloadFiles([
        { path: 'src/views/fixture-legacy-workbench.ts', text: legacyWorkbench },
      ])
    ).toEqual(['src/views/fixture-legacy-workbench.ts']);
    expect(
      directApiCallFiles([{ path: 'src/views/fixture-legacy-workbench.ts', text: legacyWorkbench }])
    ).toEqual(['src/views/fixture-legacy-workbench.ts']);

    // 夹具三 只刷待办不刷已办（回读与任务列表侧不同源的形态）
    const staleReadback = 'const handleApprove = async () => {\n  fetchPendingTasks();\n};\n';
    expect(constFnBody(staleReadback, 'const handleApprove = async ()')).not.toMatch(
      /await Promise\.all\(\[fetchPendingTasks\(\), fetchProcessedTasks\(\)\]\)/
    );

    // 夹具四 注释里的被禁形态不得误报（codeOnly 的剔除面）
    expect(codeOnly("// await approveBpmTask({ task_id: 1, handler_id: 2 })")).toBe('');
  });
});

describe('审批唯一写口：载荷组装的契约形状', () => {
  const ok: BpmApprovalSubmitInput = {
    taskId: 42,
    handlerId: 7,
    handlerName: '张三',
    action: 'approve',
    opinion: '  单价符合协议区间  ',
  };

  it('键集合与后端 ExecuteApprovalRequest 逐字一致，意见去空后原样转达', () => {
    expect(buildBpmApprovalPayload(ok)).toEqual({
      task_id: 42,
      handler_id: 7,
      handler_name: '张三',
      action: 'approve',
      approval_opinion: '单价符合协议区间',
    });
  });

  it('空串与纯空白意见省略该键（后端为 Option，空串不是「未填」的合法编码）', () => {
    expect(buildBpmApprovalPayload({ ...ok, opinion: '   ' })).not.toHaveProperty(
      'approval_opinion'
    );
    expect(buildBpmApprovalPayload({ ...ok, opinion: undefined })).not.toHaveProperty(
      'approval_opinion'
    );
  });

  it('动作取值域只有 approve 与 reject', () => {
    expect(buildBpmApprovalPayload({ ...ok, action: 'reject' }).action).toBe('reject');
    expect(() =>
      buildBpmApprovalPayload({ ...ok, action: 'transfer' as unknown as BpmApprovalSubmitInput['action'] })
    ).toThrow(/取值域外/);
  });

  it('办理人身份与任务主键非法即抛错，不静默发出必然错位或必被拒的请求', () => {
    expect(() => buildBpmApprovalPayload({ ...ok, handlerId: 0 })).toThrow(/handlerId/);
    expect(() => buildBpmApprovalPayload({ ...ok, handlerId: 7.5 })).toThrow(/handlerId/);
    expect(() => buildBpmApprovalPayload({ ...ok, handlerName: '' })).toThrow(/handler_name/);
    expect(() => buildBpmApprovalPayload({ ...ok, taskId: -1 })).toThrow(/taskId/);
    expect(() => buildBpmApprovalPayload({ ...ok, taskId: 1.5 })).toThrow(/taskId/);
  });
});
