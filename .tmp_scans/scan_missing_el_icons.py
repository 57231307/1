#!/usr/bin/env python3
"""扫描所有 .vue 父页面：template 引用但 script 缺 import 的 ElXxx (含 Icon)"""
import re
from pathlib import Path

SRC = Path("/workspace/frontend/src")
ALL_VUE = list(SRC.rglob("*.vue"))

print(f"扫描 {len(ALL_VUE)} 个 .vue 文件...")

# Element Plus 自动导入能识别的组件（非 Icon）
# 如果是 ElXxx 但 vite.config.ts 配置了 ElementPlusResolver，会自动注册
# 但 Icons（ElIcon / 各 ElIcon 名）需要单独 import @element-plus/icons-vue
# 此脚本假定所有 ElXxx 都需手动 import（保守策略）

problems = []
for vf in ALL_VUE:
    content = vf.read_text(encoding="utf-8")
    rel = str(vf.relative_to(SRC))
    tmpl_match = re.search(r'<template[^>]*>(.*?)</template>', content, re.DOTALL)
    if not tmpl_match:
        continue
    template = tmpl_match.group(1)
    script_match = re.search(r'<script\s+setup[^>]*>(.*?)</script>', content, re.DOTALL)
    if not script_match:
        continue
    script = script_match.group(1)

    # 提取 template 中的 ElXxx 组件
    tags_in_template = set()
    for m in re.finditer(r'<(El[A-Z][A-Za-z0-9]+)', template):
        tags_in_template.add(m.group(1))

    if not tags_in_template:
        continue

    # 提取 import 中的 ElXxx
    imported = set()
    for m in re.finditer(r"import\s+(\w+)\s+from\s+['\"][^'\"]*\.vue['\"]", script):
        imported.add(m.group(1))
    for m in re.finditer(r"import\s*\{([^}]+)\}\s*from", script):
        for x in m.group(1).split(','):
            x = x.strip().split(' as ')[-1].strip()
            if x:
                imported.add(x)
    # 提取 element-plus/icons-vue 的 import
    for m in re.finditer(r"import\s*\{([^}]+)\}\s*from\s+['\"]@element-plus/icons-vue['\"]", script):
        for x in m.group(1).split(','):
            x = x.strip().split(' as ')[-1].strip()
            if x:
                imported.add(x)

    missing = tags_in_template - imported
    if missing:
        problems.append({
            "file": rel,
            "missing_imports": sorted(missing),
        })

problems.sort(key=lambda x: -len(x['missing_imports']))

print(f"\n=== template 引用 ElXxx 但 import 缺失 (共 {len(problems)} 文件) ===")
for p in problems:
    print(f"  {p['file']}: {p['missing_imports']}")
