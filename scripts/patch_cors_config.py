#!/usr/bin/env python3
"""
修补生产服务器 config.yaml，添加 CorsConfig 缺失字段。

操作:
  - 解析 yaml 文本（不依赖 PyYAML，用最简的状态机方式）
  - 在 cors 段下补全 4 个缺失字段
  - 保留原有 allowed_origins（不覆盖运维已配置的）
  - 原子写入（先写临时文件再 rename）

用法:
  python3 patch_cors_config.py /opt/bingxi-erp/backend/config.yaml
"""
import sys
import os
import re


CORS_BLOCK = """  allow_credentials: true
  allowed_methods:
    - "GET"
    - "POST"
    - "PUT"
    - "DELETE"
    - "OPTIONS"
  allowed_headers:
    - "Content-Type"
    - "Authorization"
    - "X-Requested-With"
    - "Cookie"
    - "Accept"
  max_age_secs: 3600
"""


def patch(path: str) -> None:
    if not os.path.exists(path):
        print(f"ERROR: 文件不存在: {path}", file=sys.stderr)
        sys.exit(1)

    with open(path, "r", encoding="utf-8") as f:
        text = f.read()

    # 1. 找 cors 段起止位置
    cors_match = re.search(r"^(cors:)\s*$", text, flags=re.MULTILINE)
    if not cors_match:
        # 没有 cors 段，追加到文件末尾
        text = text.rstrip() + "\n\ncors:\n" + CORS_BLOCK
        added = ["cors 段(整体)"]
    else:
        start = cors_match.end()
        # 找 cors 段之后的下一个顶级 key (行首不以空格开头且以 xxx: 形式)
        next_top = re.search(r"^[a-z_][a-z0-9_]*:.*$", text[start:], flags=re.MULTILINE)
        if next_top:
            end = start + next_top.start()
        else:
            end = len(text)
        cors_block = text[start:end]

        # 2. 检查缺失字段
        missing = []
        for key in [
            "allow_credentials",
            "allowed_methods",
            "allowed_headers",
            "max_age_secs",
        ]:
            if not re.search(rf"^\s+{key}:", cors_block, flags=re.MULTILINE):
                missing.append(key)

        if missing:
            # 在 cors 段末尾追加缺失字段
            new_cors = cors_block.rstrip() + "\n" + CORS_BLOCK
            text = text[:start] + new_cors + text[end:]
            added = missing
        else:
            added = []

    # 原子写入
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        f.write(text)
    os.replace(tmp, path)

    if added:
        print(f"✓ 已补全: {added}")
    else:
        print("✓ cors 段已完整，无需修改")
    print(f"✓ 文件已写入: {path}")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        print("用法: python3 patch_cors_config.py <config.yaml 路径>", file=sys.stderr)
        sys.exit(1)
    patch(sys.argv[1])
