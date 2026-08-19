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


def get_commits(prev_tag):
    if prev_tag:
        rev_range = f"{prev_tag}..HEAD"
    else:
        rev_range = "--max-count=200"

    fmt = "--format=%H%x00%s%x00%b%x00"
    result = subprocess.run(
        ["git", "log", rev_range, fmt, "--no-merges"],
        capture_output=True, text=True,
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


def parse_bullets(body):
    bullets = []
    for line in body.split("\n"):
        line = line.strip()
        if not line or line.startswith("Co-authored-by") or line.startswith("Signed-off-by"):
            continue
        m = re.match(r'^[\*\-]\s+([a-z]+)(?:\(([^)]*)\))?:\s*(.*)$', line)
        if m:
            bullets.append({
                "type": m.group(1),
                "scope": (m.group(2) or "").strip(),
                "desc": m.group(3).strip(),
            })
    return bullets


def map_type(btype, scope):
    override = SCOPE_OVERRIDE.get((btype, scope))
    if override:
        return override
    return TYPE_MAP.get(btype, "other")


def get_commit_type(subject):
    m = re.match(r'^([a-z]+)', subject)
    return m.group(1) if m else "other"


def format_release_notes(version, tag_name, prev_tag, commits):
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
        bullets = parse_bullets(c["body"])

        if bullets:
            for b in bullets:
                mtype = map_type(b["type"], b["scope"])
                key = f"{b['type']}: {b['desc']}"
                if key not in seen:
                    seen.add(key)
                    sections[mtype].append(f"* {b['desc']}")

            for b in bullets:
                mtype = map_type(b["type"], b["scope"])
                if mtype not in ("chore", "docs", "test", "other"):
                    if b["desc"] not in seen:
                        summary_items.append(b["desc"])
                        seen.add(b["desc"])
                    break
        else:
            ctype = get_commit_type(c["subject"])
            mtype = TYPE_MAP.get(ctype, "other")
            if mtype not in ("chore", "docs", "test", "other"):
                if c["subject"] not in seen:
                    summary_items.append(c["subject"])
                    seen.add(c["subject"])

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

    section_headers = OrderedDict([
        ("feat", "## 🆕 新增（feat）"),
        ("refactor", "## ✏️ 修改（refactor / perf / style）"),
        ("remove", "## 🗑️ 删除（remove / chore-delete）"),
        ("fix", "## 🐛 修复（fix）"),
        ("breaking", "## 🔄 变更（breaking / 其他变更）"),
    ])

    for key, header in section_headers.items():
        lines.append(header)
        lines.append("")
        items = sections.get(key, [])
        if items:
            seen_items = set()
            for item in items:
                if item not in seen_items:
                    seen_items.add(item)
                    lines.append(item)
        else:
            lines.append("_无_")
        lines.append("")

    total = sum(len(v) for v in sections.values())
    lines.append("---")
    lines.append("")
    lines.append("## 📊 统计")
    lines.append("")
    lines.append("| 类型 | 数量 |")
    lines.append("|------|------|")
    lines.append(f"| 新增（feat） | {len(sections['feat'])} |")
    lines.append(f"| 修复（fix） | {len(sections['fix'])} |")
    lines.append(f"| 修改（refactor/perf/style） | {len(sections['refactor'])} |")
    lines.append(f"| 删除（remove） | {len(sections['remove'])} |")
    lines.append(f"| 变更（breaking） | {len(sections['breaking'])} |")
    lines.append(f"| 文档（docs） | {len(sections['docs'])} |")
    lines.append(f"| 测试（test） | {len(sections['test'])} |")
    lines.append(f"| 构建/工具（chore） | {len(sections['chore'])} |")
    lines.append(f"| **合计** | **{total}** |")
    lines.append("")
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
    lines.append("- **后端**: Rust + Axum + SeaORM")
    lines.append("- **前端**: Vue 3 + TypeScript + Element Plus")
    lines.append("- **数据库**: PostgreSQL 14+")
    lines.append("")
    lines.append("---")
    lines.append("")
    lines.append("*此发布说明由 CI 自动生成，依据 `.github/RELEASE_TEMPLATE.md` 模板格式化。*")
    lines.append("")

    return "\n".join(lines)


if __name__ == "__main__":
    prev_tag = sys.argv[1] if len(sys.argv) > 1 else ""
    version = sys.argv[2] if len(sys.argv) > 2 else ""
    tag_name = sys.argv[3] if len(sys.argv) > 3 else version

    commits = get_commits(prev_tag)
    notes = format_release_notes(version, tag_name, prev_tag, commits)

    output_path = os.environ.get("RELEASE_NOTES_PATH", "release_notes.md")
    with open(output_path, "w") as f:
        f.write(notes)
    print(f"✅ Release notes 已生成: {output_path}")
    print(f"提交数: {len(commits)}")