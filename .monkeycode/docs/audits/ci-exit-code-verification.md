# CI 测试编译门禁修复验证

**审计日期**: 2026-08-22
**审计对象**: `.github/workflows/ci-cd.yml`
**审计范围**: 任务 1.6 — CI 测试编译门禁修复确认
**审计类型**: 配置正确性 / 失败传播链路

---

## 结论

**已修复**。预编译失败会真实传播为 CI failure，测试分区 job 的 `EXIT_CODE` 检查逻辑完整，测试编译失败时不会再产生"假绿"。

---

## 一、审计范围

| Job 名 | 定位 | 职责 |
|--------|------|------|
| `ci-build-test-artifacts` | ci-cd.yml:1181 | 预编译所有 Rust 测试并打包为 `nextest-archive.tar.zst` |
| `ci-test-rust` | ci-cd.yml:1242 | 30 个分片从 archive 运行测试（不重复编译） |

两者构成"预编译 → 分片运行"的两段式测试链。门禁修复的核心是：预编译失败必须让整条链失败，不能只产出绿色 archive 后让分片空跑。

---

## 二、ci-build-test-artifacts：预编译失败传播机制

### 2.1 关键步骤

```yaml
- name: 编译所有测试并打包为 archive
  working-directory: backend
  run: |
    cargo nextest archive --archive-file nextest-archive.tar.zst
```

位于 ci-cd.yml:1223-1226。

### 2.2 Shell 上下文分析

**任务 1.6 原始审计指令要求确认 `cargo nextest archive` 是否在 `set -e` 下执行**。直接观察该 `run` 块脚本体内并未显式出现 `set -e` 字样，但失败传播依赖的不是脚本内的 `set -e`，而是 GitHub Actions 的 shell 调用约定：

1. **顶层默认 shell 声明**（ci-cd.yml:74-76）：
   ```yaml
   defaults:
     run:
       shell: bash
   ```
   该声明作用于所有 job 的所有 `run` 步骤，无 job 级或 step 级覆盖（全文 `shell:` 仅出现 1 处，即此顶层声明）。

2. **GitHub Actions 对 `shell: bash` 的实际调用**：
   ```
   bash --noprofile --norc -eo pipefail {0}
   ```
   其中 `-e` 由平台注入，等价于在每个 `run` 脚本首行隐式启用 `set -e`，`-o pipefail` 确保管道中任一环节失败即整体失败。

3. **结论**：`cargo nextest archive` 运行在 `bash -eo pipefail` 下。当编译失败时（cargo 返回非零退出码），`-e` 选项立即终止脚本，step 以非零退出码退出，`ci-build-test-artifacts` job 标记为 failure。

### 2.3 失败传播链路

```
cargo 编译失败 (exit != 0)
  └─ bash -e 立即终止脚本
     └─ step 退出码 != 0
        └─ ci-build-test-artifacts job = failure
           └─ archive artifact 未上传 / 上传空文件
              └─ ci-test-rust (needs: ci-build-test-artifacts) 不会触发
```

`ci-test-rust` 显式声明 `needs: ci-build-test-artifacts`（ci-cd.yml:1247），依赖 job 失败时下游 job 不会运行，因此不会出现"预编译已挂、分片仍绿"的假绿。

---

## 三、ci-test-rust：EXIT_CODE 检查逻辑

### 3.1 关键脚本片段（ci-cd.yml:1300-1376）

```bash
set +e
cargo nextest run \
  --archive-file nextest-archive.tar.zst \
  --partition hash:${{ matrix.partition }}/30 \
  --test-threads=1 \
  --no-fail-fast \
  > reports/nextest-output.txt 2>&1
EXIT_CODE=$?
set -e
```

脚本先用 `set +e` 关闭 errexit，以捕获 `cargo nextest run` 的真实退出码到 `EXIT_CODE`（避免 `-e` 在命令失败时直接终止导致无法进入后续报告分支），随后恢复 `set -e`。

### 3.2 双重失败判定

脚本末尾存在两条独立的硬失败出口，确保"零容忍"：

**出口 A — 测试用例失败**（ci-cd.yml:1367-1370）：
```bash
if [ "$FAILED" -gt "0" ]; then
  echo "❌ 分片 ... 发现 $FAILED 个测试失败，CI 失败"
  exit 1
fi
```

**出口 B — 进程异常退出**（ci-cd.yml:1371-1374）：
```bash
if [ "$EXIT_CODE" -ne "0" ]; then
  echo "❌ 分片 ... 进程异常退出（EXIT_CODE=$EXIT_CODE）"
  exit 1
fi
```

`EXIT_CODE != 0` 覆盖了"无 FAIL 行但进程整体异常退出"的场景（例如 archive 损坏、nextest 内部 panic、信号终止等）。两个出口任一命中即 `exit 1`，step 失败。

### 3.3 区分"编译失败"与"用例失败"的报告分支

脚本对 `EXIT_CODE != 0 && FAILED == 0` 这一无 FAIL 行的异常场景做了专门标记（ci-cd.yml:1330-1337、1357-1358）：

```bash
if [ "$EXIT_CODE" -ne "0" ] && [ "$FAILED" -eq "0" ]; then
  echo "## ❌ 编译失败或进程异常退出（EXIT_CODE=$EXIT_CODE）"
  # 输出 error[...] 行与最后 50 行原始日志
fi
```

这恰好是"假绿"最易出现的灰区——nextest 整体失败但未产出标准 `FAIL` 行。脚本在此分支仍 `exit 1`，不会放行。

---

## 四、其他相关 EXIT_CODE 验证点

对 ci-cd.yml 全文 `EXIT_CODE` 出现 44 处，覆盖构建、审计、E2E 等多个 job，均采用"`set +e` 捕获 → 判定 → `exit $EXIT_CODE`/`exit 1`"的同构模式。例如：

- ci-build（构建 job）：ci-cd.yml:333-379，`EXIT_CODE=${PIPESTATUS[0]}` ... `exit $EXIT_CODE`
- ci-audit（审计 job）：ci-cd.yml:1132-1168，`EXIT_CODE=$?` ... `exit $EXIT_CODE`
- E2E 测试：ci-cd.yml:2508-2517，`EXIT_CODE=$?` ... `exit $EXIT_CODE`

模式一致，门禁修复在所有相关 job 中通用化落地，而非仅限测试链路。

---

## 五、修复机制总结

| 检查点 | 状态 | 证据 |
|--------|------|------|
| `ci-build-test-artifacts` 编译失败会 exit 1 | 已落实 | 顶层 `defaults.run.shell: bash` 等价 `bash -eo pipefail`；`cargo nextest archive` 在该 shell 下，失败即脚本终止、step 非 0 退出（ci-cd.yml:74-76, 1223-1226） |
| `ci-test-rust` 存在 EXIT_CODE 检查 | 已落实 | `EXIT_CODE=$?` 捕获 + 末尾 `if [ "$EXIT_CODE" -ne "0" ]; then exit 1` 硬出口（ci-cd.yml:1309, 1371-1374） |
| 测试编译失败时报告 failure（消除假绿） | 已落实 | 预编译失败 → archive 未产出 → `ci-test-rust` 因 `needs` 不触发；即便 archive 损坏被分片拉取，`EXIT_CODE != 0 && FAILED == 0` 分支仍 `exit 1` 并输出诊断日志（ci-cd.yml:1247, 1330-1337, 1357-1358, 1371-1374） |
| 区分编译失败与用例失败的报告 | 已落实 | 三分支报告：编译异常 / 全通过 / 用例失败，各自独立 Markdown 与 Step Summary（ci-cd.yml:1330-1364） |

---

## 六、验证方式

无破坏性命令执行。本次审计仅读取 ci-cd.yml 静态配置，对照 GitHub Actions shell 调用约定与 bash `-e`/`pipefail` 语义推导失败传播链路。未修改任何代码文件，未创建 PR，未推送。
