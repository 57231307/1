#!/usr/bin/env python3
"""验证脚本：检查 cfg(test) 检测和测试函数排除"""
import os
import re
import sys
sys.path.insert(0, "/workspace/.audit-reports")
from scan_code import LineCounter, scan_file, BACKEND_SRC

# 收集所有函数
all_funcs = []
test_funcs = []
non_test_funcs = []

for root, dirs, files in os.walk(BACKEND_SRC):
    for fname in files:
        if fname.endswith(".rs"):
            fp = os.path.join(root, fname)
            funcs, _ = scan_file(fp)
            for f in funcs:
                if f[3]:
                    test_funcs.append((f[0], f[1], f[2], fp))
                else:
                    non_test_funcs.append((f[0], f[1], f[2], fp))

print(f"总函数数: {len(all_funcs) + len(test_funcs) + len(non_test_funcs)}")
print(f"测试函数数 (被排除): {len(test_funcs)}")
print(f"非测试函数数: {len(non_test_funcs)}")
print()

# 检查 builtin_transition_rules 是否在 >100 清单中
print("验证 builtin_transition_rules 是否被正确识别为非测试函数:")
for fname, sl, length, fp in non_test_funcs:
    if fname == "builtin_transition_rules":
        print(f"  找到: {fname} at {fp}:{sl} length={length}")
print()

# 检查可能漏判的 #[cfg(test)] 函数
# 搜索所有包含 #[cfg(test)] 的行
print("检查所有 #[cfg(test)] 出现位置 (采样前20个):")
count = 0
for root, dirs, files in os.walk(BACKEND_SRC):
    for fname in files:
        if fname.endswith(".rs"):
            fp = os.path.join(root, fname)
            try:
                with open(fp, "r", encoding="utf-8", errors="replace") as f:
                    for i, line in enumerate(f, 1):
                        if "#[cfg(test)]" in line:
                            count += 1
                            if count <= 20:
                                rel = os.path.relpath(fp, "/workspace")
                                print(f"  {rel}:{i}: {line.rstrip()[:80]}")
            except Exception:
                pass
print(f"  总计 #[cfg(test)] 出现次数: {count}")
