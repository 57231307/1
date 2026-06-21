#!/usr/bin/env python3
"""扫描所有 .vue 父页面：template 引用但 script 缺 import 的组件"""
import re
import os
from pathlib import Path

SRC = Path("/workspace/frontend/src")
ALL_VUE = list(SRC.rglob("*.vue"))

print(f"扫描 {len(ALL_VUE)} 个 .vue 文件...")

problems = []
for vf in ALL_VUE:
    content = vf.read_text(encoding="utf-8")
    rel = str(vf.relative_to(SRC))
    # 提取 template 中所有标签
    tmpl_match = re.search(r'<template[^>]*>(.*?)</template>', content, re.DOTALL)
    if not tmpl_match:
        continue
    template = tmpl_match.group(1)
    # 提取 script setup 块
    script_match = re.search(r'<script\s+setup[^>]*>(.*?)</script>', content, re.DOTALL)
    if not script_match:
        continue
    script = script_match.group(1)

    # 提取 template 中的自定义组件（首字母大写）
    # 排除 html 原生标签
    html_tags = set("div,span,p,h1,h2,h3,h4,h5,h6,a,img,table,tr,td,th,thead,tbody,tfoot,ul,ol,li,"
                    "form,input,button,select,option,textarea,label,br,hr,section,article,header,"
                    "footer,main,nav,aside,figure,figcaption,video,audio,source,canvas,svg,path,"
                    "circle,rect,line,polyline,polygon,text,g,defs,linearGradient,stop,iframe,"
                    "style,script,link,meta,title,head,body,html,doctype,pre,code,blockquote,"
                    "em,strong,b,i,u,s,sub,sup,small,mark,del,ins,abbr,cite,q,dfn,kbd,samp,var,"
                    "time,ruby,rt,rp,bdi,bdo,wbr,br,area,base,col,embed,object,param,track,"
                    "fieldset,legend,optgroup,output,datalist,keygen,menu,menuitem,template,"
                    "noscript,address,details,summary,dialog,slot".split(","))

    # 提取 <Xxx 形式（不区分大小写，但只匹配首字母大写的）
    tags_in_template = set()
    for m in re.finditer(r'<([A-Z][A-Za-z0-9]+)', template):
        tags_in_template.add(m.group(1))

    # 提取 import 中的组件
    imported = set()
    for m in re.finditer(r"import\s+(\w+)\s+from\s+['\"][^'\"]*\.vue['\"]", script):
        imported.add(m.group(1))
    # 处理 import * as X
    for m in re.finditer(r"import\s+\*\s+as\s+(\w+)\s+from", script):
        star_name = m.group(1)
        # 找出该 namespace 后的所有 .xxx 引用
        for m2 in re.finditer(r'\b' + star_name + r'\.(\w+)', script + template):
            imported.add(m2.group(1))
    # 处理 import { X, Y, Z }
    for m in re.finditer(r"import\s*\{([^}]+)\}\s*from", script):
        for x in m.group(1).split(','):
            x = x.strip().split(' as ')[-1].strip()
            if x:
                imported.add(x)

    # 找出 template 引用但 import 缺失
    missing = tags_in_template - imported - html_tags
    # 过滤 Element Plus 组件 (ElXxx)
    missing = {m for m in missing if not m.startswith('El')}
    # 过滤 router-link 等
    missing = {m for m in missing if m not in ('RouterLink', 'RouterView')}

    if missing:
        problems.append({
            "file": rel,
            "missing_imports": sorted(missing),
        })

# 按缺失数排序
problems.sort(key=lambda x: -len(x['missing_imports']))

print(f"\n=== 父页面 template 引用但 import 缺失 (共 {len(problems)} 文件) ===")
for p in problems:
    if p['missing_imports']:
        print(f"  {p['file']}: {p['missing_imports']}")
