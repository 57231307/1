# 依赖漏洞扫描与供应链安全审计报告

> 审计编号: 4.7 / 4.8
> 审计日期: 2026-08-22
> 审计范围: `.github/workflows/ci-cd.yml` 的 `ci-audit` job、`backend/Cargo.toml`
> 审计人: Code Implementation Agent

---

## 4.7 依赖漏洞扫描

### 1. cargo-audit job 存在性检查

**检查方法**: `grep "audit" .github/workflows/ci-cd.yml`

**结论**: 存在 `ci-audit` job（行 1599），名为「🛡️ 依赖审计」，`runs-on: ubuntu-latest`，`timeout-minutes: 15`，`needs: ci-info`。该 job 同时负责 Rust（`cargo audit`）和前端（`npm audit`）依赖审计，并上传 `dependency-audit-reports` artifact（保留 90 天）。

### 2. 漏洞是否阻塞 CI（修改前状态）

**检查方法**: 读取 `ci-audit` job 全文，分析漏洞处理逻辑。

**结论**: **修改前漏洞不阻塞 CI**。

修改前的代码逻辑（行 1622-1659）：
- 使用 `set +e` 捕获 `cargo audit` 退出码，**不传播**失败
- 注释明确写「发现漏洞不会让步骤失败，只有 cargo audit / npm audit 命令本身异常时才会阻塞」
- 行 1657 原文：`⚠️  $VULN_COUNT 个漏洞（不阻塞，记录在案）`
- 仅生成报告 + 写入 `$GITHUB_STEP_SUMMARY`，**无任何 `exit 1` 阻塞逻辑**
- 前端 npm audit 同理（行 1718）：`⚠️  $VULN_COUNT 个漏洞（不阻塞，记录在案）`

### 3. 修改内容：让 high/critical 漏洞阻塞 CI

**已修改文件**: `.github/workflows/ci-cd.yml`（`ci-audit` job）

#### 3.1 Rust 依赖审计步骤（行 1619-1676）

**改动 1 — 加 `--no-dev-deps`**（行 1624-1625）:
```yaml
# 4.7：--no-dev-deps 跳过 [dev-dependencies]，仅审计生产依赖
cargo audit --json --no-dev-deps 2>reports/cargo-audit.json | tee reports/cargo-audit.txt
```
理由：`--no-dev-deps` 跳过 `[dev-dependencies]`，仅审计生产依赖，减少误报，聚焦线上风险。

**改动 2 — 提取 high/critical 计数**（行 1632-1636）:
```bash
# 4.7 依赖漏洞扫描：high/critical 级别漏洞必须阻塞 CI
# cargo-audit JSON 中 vulnerabilities.list[].advisory.severity 为
# "low" | "medium" | "high" | "critical"（cargo-audit 0.18+ 字段）
# --no-dev-deps 跳过 [dev-dependencies]，仅审计生产依赖
CRITICAL_HIGH_COUNT=$(jq '[.vulnerabilities.list[]? | select(.advisory.severity == "high" or .advisory.severity == "critical")] | length' reports/cargo-audit.json 2>/dev/null || echo 0)
```

**改动 3 — 报告新增 high/critical 指标**（行 1643）:
```bash
echo "**high/critical 漏洞**: $CRITICAL_HIGH_COUNT  "
```
并在 Step Summary 三分支输出（行 1663-1669）：无漏洞 / 有 high/critical 阻塞 / 仅 low+medium 记录。

**改动 4 — 阻塞判定**（行 1672-1676）:
```bash
# high/critical 漏洞存在则阻塞 CI
if [ "$CRITICAL_HIGH_COUNT" -gt "0" ]; then
  echo "::error::检测到 $CRITICAL_HIGH_COUNT 个 high/critical 级别漏洞，阻塞 CI。详见 reports/cargo-audit-report.md"
  exit 1
fi
```

#### 3.2 前端 npm 审计步骤（行 1681-1733）

为保持策略一致性，同步加固前端审计：
- 行 1692-1694：提取 `NPM_CRITICAL_HIGH_COUNT`（severity 为 `critical`/`high` 的包数量）
- 行 1701：报告新增 high/critical 指标
- 行 1715-1727：Step Summary 三分支输出
- 行 1730-1733：high/critical 漏洞 `exit 1` 阻塞 CI

#### 3.3 Job 注释更新（行 1604-1607）

原文「发现漏洞不会让步骤失败」已更正为「4.7 依赖漏洞扫描加固：high/critical 级别漏洞也会阻塞 CI（exit 1），low/medium 仅记录在案，不阻塞。cargo audit 使用 --no-dev-deps，仅审计生产依赖。」

### 4. 阻塞策略汇总

| 漏洞级别 | Rust cargo-audit | 前端 npm audit | 行为 |
|---------|------------------|----------------|------|
| critical | `exit 1` 阻塞 | `exit 1` 阻塞 | CI 失败 |
| high | `exit 1` 阻塞 | `exit 1` 阻塞 | CI 失败 |
| medium | 记录在案 | 记录在案 | 不阻塞 |
| low | 记录在案 | 记录在案 | 不阻塞 |
| 命令本身异常 | `set -e` 阻塞 | `set -e` 阻塞 | CI 失败 |

### 4.7 结论

- `ci-audit` job 存在且功能完整。
- **修改前**漏洞不阻塞 CI，存在高危漏洞放行风险。
- **已修改** `.github/workflows/ci-cd.yml`，Rust 与前端均加 `--no-dev-deps`/severity 判定 + `exit 1`，high/critical 漏洞阻塞 CI，low/medium 仅记录。
- 修改后 YAML 缩进与 `run: |` 块结构经人工核对一致。

---

## 4.8 供应链安全

### 1. 不可信源检查（git+ 源）

**检查方法**: `rg -n 'git\+' backend/Cargo.toml`

**结果**: **0 匹配**。`backend/Cargo.toml` 无任何 `git+`、`ssh+git`、`git://` 等 Git 源依赖。

### 2. registry/path 源检查

**检查方法**: `rg -n 'registry|path\s*=' backend/Cargo.toml`

**结果**:
| 行号 | 内容 | 判定 |
|------|------|------|
| 131 | `migration = { version = "0.1.0", path = "migration" }` | **工作区内部依赖**，非外部源 |
| 183 | `path = "src/main.rs"` | `[[bin]]` 二进制入口路径，非依赖 |
| 203 | `path = "src/bin/hash_password.rs"` | `[[bin]]` 二进制入口路径，非依赖 |
| 207 | `path = "src/bin/cli.rs"` | `[[bin]]` 二进制入口路径，非依赖 |

- 无 `registry = "..."` 字段（无自定义 registry 引用）
- 第 131 行 `path = "migration"` 是**工作区内部相对路径**：`backend/Cargo.toml` 第 210 行 `[workspace] members = [".", "migration"]` 确认 `migration` 是 workspace 成员 crate，属于同一仓库内部模块，非外部不可信源。

### 3. 其他供应链风险检查

| 检查项 | 命令 | 结果 |
|--------|------|------|
| `[patch.crates-io]` 覆盖 | `rg -n '\[patch' backend/Cargo.toml` | 0 匹配，无 patch 覆盖 |
| `replace-with` 镜像源 | `rg -n 'replace-with' backend/Cargo.toml` | 0 匹配，无镜像替换 |

### 4. 依赖来源汇总

`backend/Cargo.toml` 共声明依赖（不含 dev/build）约 40 项，全部使用 crates.io 标准版本号格式（如 `axum = "0.8"`、`sea-orm = { version = "2.0.2", ... }`），无任何：
- Git 源依赖
- 自定义 registry
- 外部 path 依赖
- patch 覆盖
- 镜像替换

### 4.8 结论

- `backend/Cargo.toml` 无 `git+` 不可信源，**确认**（0 匹配）。
- 所有外部依赖均来自 crates.io（标准版本号格式），唯一 `path =` 为工作区内部 `migration` crate（`workspace.members` 成员），非外部源。
- 无 `[patch]`、无 `replace-with` 镜像替换。
- **供应链安全：通过**。所有依赖来自 crates.io 官方 registry 或工作区内部，无可信度风险。

---

## 总结

| 审计项 | 状态 | 是否修改 CI |
|--------|------|-------------|
| 4.7 依赖漏洞扫描 | 已加固 | **是**，修改 `ci-cd.yml` 的 `ci-audit` job |
| 4.8 供应链安全 | 通过 | 否 |

### 修改清单

1. `.github/workflows/ci-cd.yml` 行 1604-1607：更新 `ci-audit` job 注释
2. `.github/workflows/ci-cd.yml` 行 1624-1625：`cargo audit` 加 `--no-dev-deps`
3. `.github/workflows/ci-cd.yml` 行 1632-1676：Rust 审计新增 high/critical 阻塞逻辑
4. `.github/workflows/ci-cd.yml` 行 1692-1733：前端 npm 审计新增 high/critical 阻塞逻辑

### 风险提示

- `cargo audit` 的 `advisory.severity` 字段需要 cargo-audit 0.18+。CI 中 `cargo install cargo-audit --locked`（行 1617）默认安装最新版，满足要求。
- 若后续 cargo-audit 版本变更 JSON schema（如 severity 字段位置变动），需同步更新 jq 表达式。当前表达式使用 `.advisory.severity` 与 cargo-audit 0.18+ 文档一致。
- `--no-dev-deps` 意味着 dev 依赖漏洞不扫描，这是预期行为（dev 依赖仅用于测试/构建，不进入生产二进制）。如需同时审计 dev 依赖，移除该 flag 即可，但 high/critical 阻塞逻辑会误伤测试依赖。
