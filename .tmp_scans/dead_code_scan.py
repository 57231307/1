#!/usr/bin/env python3
"""全项目死代码深度扫描"""
import re
import os
import sys
import json
from pathlib import Path

BACKEND_SRC = Path("/workspace/backend/src")
BACKEND_TESTS = Path("/workspace/backend/tests")
BACKEND_MIGRATION = Path("/workspace/backend/migration/src")
FRONTEND_SRC = Path("/workspace/frontend/src")

ALL_RS_FILES = list(BACKEND_SRC.rglob("*.rs")) + list(BACKEND_TESTS.rglob("*.rs")) + list(BACKEND_MIGRATION.rglob("*.rs"))
ALL_VUE_TS = list(FRONTEND_SRC.rglob("*.ts")) + list(FRONTEND_SRC.rglob("*.tsx")) + list(FRONTEND_SRC.rglob("*.vue"))

print(f"后端 .rs 文件数: {len(ALL_RS_FILES)}")
print(f"前端 .ts/.tsx/.vue 文件数: {len(ALL_VUE_TS)}")


def read_all_rs_concat():
    out = {}
    for f in ALL_RS_FILES:
        try:
            out[f] = f.read_text(encoding="utf-8")
        except Exception:
            pass
    return out


def read_all_frontend_concat():
    out = {}
    for f in ALL_VUE_TS:
        try:
            out[f] = f.read_text(encoding="utf-8")
        except Exception:
            pass
    return out


def extract_allow_dead_code_items(rs_contents):
    items = []
    pattern = re.compile(
        r'#\[allow\(dead_code[^)]*\)\]\s*(?://[^\n]*\n\s*)*'
        r'(pub(?:\([^)]*\))?\s+(?:async\s+)?(?:fn|struct|enum|trait|const|static|type)\s+(\w+))',
        re.MULTILINE,
    )
    for f, content in rs_contents.items():
        for m in pattern.finditer(content):
            decl = m.group(1)
            name = m.group(2)
            items.append({
                "file": str(f.relative_to("/workspace")),
                "name": name,
                "decl": decl,
            })
    return items


def count_references_in_rs(name, exclude_file, rs_contents):
    pattern = re.compile(r'\b' + re.escape(name) + r'\b')
    count = 0
    for f, content in rs_contents.items():
        if f == exclude_file:
            continue
        count += len(pattern.findall(content))
    return count


def extract_zero_ref_pub_items(rs_contents):
    pattern = re.compile(
        r'^(\s*)(pub(?:\([^)]*\))?\s+(?:async\s+)?(?:fn|struct|enum|trait|const|static)\s+(\w+))',
        re.MULTILINE,
    )
    items = []
    for f, content in rs_contents.items():
        rel = str(f.relative_to("/workspace"))
        if "/models/" in rel or "/migration/" in rel or "/tests/" in rel:
            continue
        for m in pattern.finditer(content):
            indent = m.group(1)
            decl = m.group(2)
            name = m.group(3)
            start = m.start()
            before = content[max(0, start-300):start]
            if re.search(r'#\[allow\(dead_code[^)]*\)\]', before):
                continue
            # 跳过 #[cfg(test)] 块
            if '#[cfg(test)]' in before[-200:]:
                continue
            ref_count = count_references_in_rs(name, f, rs_contents)
            if ref_count == 0:
                items.append({
                    "file": rel,
                    "name": name,
                    "decl": decl.strip(),
                })
    return items


def extract_unused_uses(rs_contents):
    items = []
    for f, content in rs_contents.items():
        rel = str(f.relative_to("/workspace"))
        if "/models/" in rel or "/migration/" in rel:
            continue
        for m in re.finditer(r'^\s*use\s+([\w:]+)(?:::\{[^}]*\})?(?::\s*\*)?;', content, re.MULTILINE):
            full_path = m.group(1)
            last = full_path.split('::')[-1]
            if last == '{' or last.startswith('*'):
                continue
            pattern = re.compile(r'\b' + re.escape(last) + r'\b')
            after = content[m.end():]
            if not pattern.search(after):
                items.append({
                    "file": rel,
                    "line": content[:m.start()].count('\n') + 1,
                    "use": full_path,
                })
    return items


def extract_frontend_unused_exports(fe_contents):
    items = []
    pattern = re.compile(
        r'^export\s+(?:const|function|class|interface|type|enum)\s+(\w+)',
        re.MULTILINE,
    )
    exports = []
    for f, content in fe_contents.items():
        for m in pattern.finditer(content):
            name = m.group(1)
            exports.append((f, name))
    for f, name in exports:
        pattern = re.compile(r'\b' + re.escape(name) + r'\b')
        ref_count = 0
        for f2, content2 in fe_contents.items():
            if f2 == f:
                continue
            ref_count += len(pattern.findall(content2))
        if ref_count == 0:
            items.append({
                "file": str(f.relative_to("/workspace")),
                "name": name,
            })
    return items


print("Loading files...")
rs_contents = read_all_rs_concat()
fe_contents = read_all_frontend_concat()
print(f"已加载: {len(rs_contents)} .rs + {len(fe_contents)} frontend")

print("\n=== Phase 1: #[allow(dead_code)] 项 ===")
allow_items = extract_allow_dead_code_items(rs_contents)
print(f"共提取 {len(allow_items)} 项")
for item in allow_items:
    item_path = Path("/workspace") / item["file"]
    ref_count = count_references_in_rs(item["name"], item_path, rs_contents)
    item["ref_count"] = ref_count

allow_active = [x for x in allow_items if x["ref_count"] > 0]
allow_dead = [x for x in allow_items if x["ref_count"] == 0]
print(f"  实际有引用 (allow 注释冗余): {len(allow_active)}")
print(f"  真死代码 (应删项或保留抑制): {len(allow_dead)}")

print("\n=== Phase 2: pub 零引用项 (非 models/migration/tests) ===")
zero_ref_items = extract_zero_ref_pub_items(rs_contents)
seen = set()
uniq_zero = []
for it in zero_ref_items:
    k = it["file"] + ":" + it["name"]
    if k not in seen:
        seen.add(k)
        uniq_zero.append(it)
print(f"去重后: {len(uniq_zero)} 项")

print("\n=== Phase 3: 未使用 use ===")
unused_uses = extract_unused_uses(rs_contents)
print(f"共 {len(unused_uses)} 项")

print("\n=== Phase 4: 前端未引用 export ===")
fe_unused = extract_frontend_unused_exports(fe_contents)
print(f"共 {len(fe_unused)} 项")

report = {
    "summary": {
        "backend_rs_files": len(ALL_RS_FILES),
        "frontend_files": len(ALL_VUE_TS),
        "allow_dead_code_total": len(allow_items),
        "allow_active_with_refs": len(allow_active),
        "allow_dead_no_refs": len(allow_dead),
        "zero_ref_pub_items": len(uniq_zero),
        "unused_use_statements": len(unused_uses),
        "frontend_unused_exports": len(fe_unused),
    },
    "allow_with_refs": allow_items,
    "allow_dead": allow_dead,
    "allow_active": allow_active,
    "zero_ref_pub_items": uniq_zero,
    "unused_use_statements": unused_uses,
    "frontend_unused_exports": fe_unused,
}

with open("/workspace/.tmp_scans/dead_code_full_report.json", "w", encoding="utf-8") as f:
    json.dump(report, f, ensure_ascii=False, indent=2)
print("\n报告保存到 /workspace/.tmp_scans/dead_code_full_report.json")
print(f"\n=== 总览 ===")
for k, v in report["summary"].items():
    print(f"  {k}: {v}")
