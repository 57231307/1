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
    lines.append("- **后端**: Rust 1.94+ + Axum 0.8 + SeaORM 2.0 + PostgreSQL 15+")
    lines.append("- **前端**: Vue 3.5 + TypeScript 5.9 + Element Plus 2.14 + Vite 8")
    lines.append("- **测试**: Playwright 1.40 (E2E) + nextest (Rust) + Vitest 4 (前端单元)")
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