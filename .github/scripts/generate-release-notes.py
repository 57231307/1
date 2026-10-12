#!/usr/bin/env python3
"""从 git commit 历史生成详细的发布说明。

解析 conventional commit 的 subject 和 body 中的 bullet points，
按类型分类生成结构化的 release notes。

用法: python3 generate-release-notes.py [PREV_TAG] [VERSION] [TAG_NAME]
"""

import subprocess
import sys
import re
import os
from collections import OrderedDict
from datetime import datetime, timezone

TYPE_MAP = {
    "feat": "feat",
    "fix": "fix",
    "refactor": "refactor",
    "perf": "refactor",
    "style": "refactor",
    "docs": "docs",
    "test": "test",
    "chore": "chore",
    "build": "chore",
    "ci": "chore",
    "remove": "remove",
    "breaking": "breaking",
    "verify": "other",
}

SCOPE_OVERRIDE = {
    ("fix", "tests"): "test",
    ("fix", "test"): "test",
    ("fix", "ci"): "chore",
    ("chore", "remove"): "remove",
    ("chore", "delete"): "remove",
}


def get_commits(prev_tag, head_ref="HEAD"):
    if prev_tag:
        args = ["git", "log", f"{prev_tag}..{head_ref}"]
    else:
        args = ["git", "log", head_ref, "--max-count=200"]
    args += ["--no-merges", "--format=%H%x00%s%x00%b%x00"]

    result = subprocess.run(
        args,
        capture_output=True, text=True, encoding="utf-8",
    )

    parts = result.stdout.split("\x00")
    commits = []
    for i in range(0, len(parts) - 2, 3):
        h = parts[i].strip()
        s = parts[i + 1].strip()
        b = parts[i + 2].strip()
        if h and s:
            commits.append({"hash": h[:8], "subject": s, "body": b})
    return commits


AREA_RULES = [
    ("backend/src/", "后端源码 (backend/src)"),
    ("backend/tests/", "后端测试 (backend/tests)"),
    ("backend/migration/", "数据库迁移 (backend/migration)"),
    ("backend/benches/", "性能基准 (backend/benches)"),
    ("frontend/src/", "前端源码 (frontend/src)"),
    ("frontend/e2e/", "前端 E2E (frontend/e2e)"),
    ("frontend/tests/", "前端单测 (frontend/tests)"),
    (".github/", "CI/CD (.github)"),
    (".monkeycode/", "内部文档 (.monkeycode)"),
]


def classify_area(path):
    for prefix, name in AREA_RULES:
        if path.startswith(prefix):
            return name
    if path.endswith(".md"):
        return "文档 (*.md)"
    if path.startswith("backend/"):
        return "后端其它 (backend/*)"
    if path.startswith("frontend/"):
        return "前端其它 (frontend/*)"
    return "其它"


def get_diff_stats(prev_tag, head_ref="HEAD"):
    """计算相对上一版本 tag 的真实代码差异（按区域分桶 + 热点文件）。

    仅依赖 git diff，不依赖 commit message，因此即使 PR 被 squash 合并、
    commit 历史被压成一条，仍能如实反映本次版本相对上一版本的变更量与覆盖面。
    返回 None 表示无对比基线或 diff 不可解析。"""
    if not prev_tag:
        return None
    result = subprocess.run(
        ["git", "diff", "--numstat", f"{prev_tag}..{head_ref}"],
        capture_output=True, text=True, encoding="utf-8",
    )
    if result.returncode != 0 or not result.stdout.strip():
        return None

    total_files = 0
    ins = 0
    dele = 0
    areas = OrderedDict()
    files = []
    for line in result.stdout.splitlines():
        parts = line.split("\t")
        if len(parts) != 3:
            continue
        a, d, path = parts
        ai = 0 if a == "-" else int(a)
        di = 0 if d == "-" else int(d)
        total_files += 1
        ins += ai
        dele += di
        area = classify_area(path)
        rec = areas.setdefault(area, [0, 0, 0])
        rec[0] += 1
        rec[1] += ai
        rec[2] += di
        files.append((path, ai, di))

    files.sort(key=lambda x: (x[1] + x[2]), reverse=True)
    return {"files": total_files, "ins": ins, "del": dele,
            "areas": areas, "top": files[:15]}


def format_diff_section(prev_tag, stats):
    lines = ["## 📈 与上一版本对比", ""]
    if not prev_tag:
        lines += ["_首个版本，无对比基线。_", ""]
        return lines
    if not stats:
        lines += ["_无法计算 diff（对比基线不可解析）。_", ""]
        return lines
    lines += [
        f"相对 `{prev_tag}` 的真实代码差异（`git diff {prev_tag}..HEAD`），"
        "不受 squash 合并影响：", "",
        f"- 合计变更 **{stats['files']}** 个文件，**+{stats['ins']}** 行 / "
        f"**-{stats['del']}** 行", "",
        "| 区域 | 文件数 | 新增行 | 删除行 |",
        "|------|-------:|-------:|-------:|",
    ]
    named = [name for _, name in AREA_RULES]
    for name in named:
        if name in stats["areas"]:
            f, i, d = stats["areas"][name]
            lines.append(f"| {name} | {f} | +{i} | -{d} |")
    for name, (f, i, d) in stats["areas"].items():
        if name not in named:
            lines.append(f"| {name} | {f} | +{i} | -{d} |")
    lines.append("")
    if stats["top"]:
        lines += ["变更最集中的文件（按增删行数）：", ""]
        for path, ai, di in stats["top"]:
            lines.append(f"- `{path}` (+{ai} / -{di})")
        lines.append("")
    return lines


def _area_breakdown(files):
    agg = {}
    for f in files:
        a = classify_area(f)
        agg[a] = agg.get(a, 0) + 1
    named = [name for _, name in AREA_RULES]
    ordered = OrderedDict()
    for name in named:
        if name in agg:
            ordered[name] = agg[name]
    for name, c in sorted(((k, v) for k, v in agg.items() if k not in named),
                          key=lambda x: -x[1]):
        ordered[name] = c
    return ordered


def get_change_matrix(prev_tag, head_ref="HEAD"):
    """按 git diff --name-status 把相对上一版本的文件分为 新增/修改/删除/重命名。

    数据源是文件差异而非 commit message，因此不受 squash 合并影响：
    A=新增、M/T=修改、D=删除、R=重命名/移动。返回 None 表示无对比基线或不可解析。"""
    if not prev_tag:
        return None
    result = subprocess.run(
        ["git", "diff", "--name-status", "-M", f"{prev_tag}..{head_ref}"],
        capture_output=True, text=True, encoding="utf-8",
    )
    if result.returncode != 0 or not result.stdout.strip():
        return None
    added, modified, deleted, renamed = [], [], [], []
    for line in result.stdout.splitlines():
        if not line.strip():
            continue
        parts = line.split("\t")
        tag = parts[0]
        if tag.startswith("R") and len(parts) >= 3:
            renamed.append((parts[1], parts[2]))
        elif tag == "A":
            added.append(parts[1])
        elif tag == "D":
            deleted.append(parts[1])
        elif tag in ("M", "T"):
            modified.append(parts[1])
        elif tag == "C" and len(parts) >= 3:
            added.append(parts[2])
    return {"added": added, "modified": modified,
            "deleted": deleted, "renamed": renamed}


def format_change_categories(prev_tag, matrix):
    """渲染 新增/修改/删除/变更 四类（相对上一版本）+ 文件级统计。"""
    if not prev_tag:
        return ["## 🆕 与上一版本对比", "", "_首个版本，无对比基线。_", ""]
    if not matrix:
        return ["## 🆕 与上一版本对比", "", "_无法计算文件级 diff（对比基线不可解析）。_", ""]
    add = matrix["added"]
    mod = matrix["modified"]
    dele = matrix["deleted"]
    ren = matrix["renamed"]
    L = []

    L += [f"## 🆕 新增（相对 {prev_tag}）", "",
          f"本版本相对上一版本新增 **{len(add)}** 个文件/模块。", "",
          "按区域分布：", "", "| 区域 | 新增文件数 |", "|------|------:|"]
    for a, c in _area_breakdown(add).items():
        L.append(f"| {a} | {c} |")
    L += ["", f"<details><summary>展开全部 {len(add)} 个新增文件</summary>", ""]
    for f in add:
        L.append(f"- `{f}`")
    L += ["", "</details>", ""]

    L += [f"## ✏️ 修改（相对 {prev_tag}）", "",
          f"修改 **{len(mod)}** 个文件。", "", "按区域分布：", "",
          "| 区域 | 修改文件数 |", "|------|------:|"]
    for a, c in _area_breakdown(mod).items():
        L.append(f"| {a} | {c} |")
    L += ["", f"<details><summary>展开全部 {len(mod)} 个修改文件</summary>", ""]
    for f in mod:
        L.append(f"- `{f}`")
    L += ["", "</details>", ""]

    L += [f"## 🗑️ 删除（相对 {prev_tag}）", "",
          f"删除 **{len(dele)}** 个文件。", ""]
    if dele:
        for f in dele:
            L.append(f"- `{f}`")
    else:
        L.append("_无_")
    L += [""]

    L += [f"## 🔄 变更 / 重命名（相对 {prev_tag}）", "",
          f"重命名/移动 **{len(ren)}** 项。", ""]
    if ren:
        for a, b in ren:
            L.append(f"- `{a}` -> `{b}`")
    else:
        L.append("_无文件重命名/移动。_")
    L += ["", "---", "", "## 📊 文件级统计", "",
          "| 类别 | 数量 |", "|------|------:|",
          f"| 新增（A） | {len(add)} |",
          f"| 修改（M） | {len(mod)} |",
          f"| 删除（D） | {len(dele)} |",
          f"| 变更/重命名（R） | {len(ren)} |",
          f"| **合计** | **{len(add) + len(mod) + len(dele) + len(ren)}** |", ""]
    return L


def parse_bullets(body):
    """解析 commit body 中的 conventional commit bullet points。

    识别形如 `- feat(scope): 描述` / `* fix: 描述` 的列表项，返回
    [{"type": ..., "scope": ..., "desc": ...}, ...]。非 bullet 行或不符合
    conventional commit 前缀的行被忽略。"""
    if not body:
        return []
    pattern = re.compile(r'^[\*\-]\s+([a-z]+)(?:\(([^)]+)\))?!?:\s+(.+)$')
    bullets = []
    for line in body.splitlines():
        m = pattern.match(line.strip())
        if m:
            bullets.append({
                "type": m.group(1),
                "scope": m.group(2) or "",
                "desc": m.group(3).strip(),
            })
    return bullets


def parse_commit_message(subject, body):
    """解析单个 commit 的 subject 和 body，提取变更项。

    支持两种格式：
    1. conventional commit body 中的 bullet points（- feat(xxx): 描述）
    2. squash merge commit body 中的自由文本段落（按关键词分类）
    """
    items = []

    # 先尝试解析 conventional commit bullet points
    bullets = parse_bullets(body)
    if bullets:
        for b in bullets:
            items.append({
                "type": map_type(b["type"], b["scope"]),
                "scope": b["scope"],
                "desc": b["desc"],
            })
        return items
    
    # 如果没有 bullet points，尝试从 subject 和 body 提取信息
    # 解析 squash merge commit body 中的多行描述
    lines = body.split("\n") if body else []
    current_section = None
    section_map = {
        "前端": "feat",
        "后端": "feat",
        "E2E": "test",
        "CI": "chore",
        "修复": "fix",
        "新增": "feat",
        "删除": "remove",
        "升级": "feat",
        "文档": "docs",
        "测试": "test",
    }
    
    for line in lines:
        line = line.strip()
        if not line or line.startswith("Co-authored-by") or line.startswith("Signed-off-by"):
            continue
        
        # 检测分类标题行（如 "前端：", "后端：", "修复：" 等）
        section_match = re.match(r'^(前端|后端|E2E|CI|修复|新增|删除|升级|文档|测试|Rust|Playwright|其他)[：:]\s*(.*)$', line)
        if section_match:
            current_section = section_map.get(section_match.group(1), "other")
            rest = section_match.group(2).strip()
            if rest:
                items.append({
                    "type": current_section,
                    "scope": section_match.group(1),
                    "desc": rest,
                })
            continue
        
        # 检测以 - 或 * 开头的列表项
        bullet_match = re.match(r'^[\*\-]\s+(.+)$', line)
        if bullet_match:
            desc = bullet_match.group(1).strip()
            # 尝试从描述中推断类型
            inferred_type = "other"
            for keyword, t in section_map.items():
                if keyword.lower() in desc.lower():
                    inferred_type = t
                    break
            items.append({
                "type": inferred_type,
                "scope": "",
                "desc": desc,
            })
            continue
        
        # 普通文本行作为描述
        if len(line) > 10 and not line.startswith("#"):
            inferred_type = "other"
            for keyword, t in section_map.items():
                if keyword.lower() in line.lower():
                    inferred_type = t
                    break
            items.append({
                "type": inferred_type,
                "scope": "",
                "desc": line,
            })
    
    # 如果 body 没有提取到任何内容，用 subject 本身
    if not items:
        ctype = get_commit_type(subject)
        mtype = TYPE_MAP.get(ctype, "other")
        items.append({
            "type": mtype,
            "scope": "",
            "desc": subject,
        })
    
    return items


def map_type(btype, scope):
    override = SCOPE_OVERRIDE.get((btype, scope))
    if override:
        return override
    return TYPE_MAP.get(btype, "other")


def get_commit_type(subject):
    m = re.match(r'^([a-z]+)', subject)
    return m.group(1) if m else "other"


def format_release_notes(version, tag_name, prev_tag, commits, stats=None, matrix=None):
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M:%S %Z")
    commit_range = f"{prev_tag}..{tag_name}" if prev_tag else f"初始版本（最近 {len(commits)} 个提交）"
    commit_count = len(commits)

    sections = OrderedDict([
        ("feat", []),
        ("refactor", []),
        ("remove", []),
        ("fix", []),
        ("breaking", []),
        ("docs", []),
        ("test", []),
        ("chore", []),
        ("other", []),
    ])
    summary_items = []
    seen = set()

    for c in commits:
        items = parse_commit_message(c["subject"], c["body"])
        
        for item in items:
            mtype = item["type"]
            key = f"{mtype}: {item['desc']}"
            if key not in seen:
                seen.add(key)
                sections[mtype].append(f"* {item['desc']}")
            
            if mtype not in ("chore", "docs", "test", "other"):
                if item["desc"] not in seen:
                    summary_items.append(item["desc"])
                    seen.add(item["desc"])

    summary = "；".join(summary_items[:3]) if summary_items else "详见下方变更分类"

    lines = []
    lines.append(f"# Bingxi Management Platform {version}")
    lines.append("")
    lines.append(f"**发布时间**: {now}")
    lines.append(f"**上一版本**: {prev_tag or '无（首个版本）'}")
    lines.append(f"**Commit 范围**: {commit_range}（共 {commit_count} 个提交）")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("## 📋 版本概述")
    lines.append("")
    lines.append(f"本次版本主要更新：{summary}")
    lines.append("")
    lines.append("---")
    lines.append("")

    lines += format_diff_section(prev_tag, stats)
    lines.append("---")
    lines.append("")

    lines += format_change_categories(prev_tag, matrix)
    lines.append("---")
    lines.append("")

    lines.append("## 📝 完整 Commit 列表")
    lines.append("")
    for c in commits:
        lines.append(f"- `{c['hash']}` {c['subject']}")
    lines.append("")
    lines.append("---")
    lines.append("")

    lines.append("## 🚀 快速部署")
    lines.append("")
    lines.append("```bash")
    lines.append("# 解压发布包")
    lines.append(f"tar -xzf release-{version}.tar.gz")
    lines.append("cd bingxi-erp")
    lines.append("")
    lines.append("# 部署后端")
    lines.append("cp backend/server /opt/bingxi-erp/backend/")
    lines.append("systemctl restart bingxi-backend")
    lines.append("")
    lines.append("# 部署前端")
    lines.append("cp -r frontend/dist/* /var/www/html/")
    lines.append("```")
    lines.append("")
    lines.append("## 🛠️ 技术栈")
    lines.append("")
    lines.append("- **后端**: Rust 1.94+ + Axum 0.8 + SeaORM 2.0 + PostgreSQL 15+")
    lines.append("- **前端**: Vue 3.5 + TypeScript 5.9 + Element Plus 2.14 + Vite 8")
    lines.append("- **测试**: Playwright 1.40 (E2E) + nextest (Rust) + Vitest 4 (前端单元)")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("*此发布说明由 CI 自动生成（`generate-release-notes.py`）："
                 "更新/修复条目解析自 commit 历史，"
                 "「与上一版本对比」章节由 `git diff PREV_TAG..HEAD` 计算，"
                 "不受 squash 合并影响。*")
    lines.append("")

    return "\n".join(lines)


if __name__ == "__main__":
    prev_tag = sys.argv[1] if len(sys.argv) > 1 else ""
    version = sys.argv[2] if len(sys.argv) > 2 else ""
    tag_name = sys.argv[3] if len(sys.argv) > 3 else version
    head_ref = sys.argv[4] if len(sys.argv) > 4 else "HEAD"

    commits = get_commits(prev_tag, head_ref)
    stats = get_diff_stats(prev_tag, head_ref)
    matrix = get_change_matrix(prev_tag, head_ref)
    notes = format_release_notes(version, tag_name, prev_tag, commits, stats, matrix)

    output_path = os.environ.get("RELEASE_NOTES_PATH", "release_notes.md")
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(notes)
    print(f"✅ Release notes 已生成: {output_path}")
    print(f"提交数: {len(commits)}")