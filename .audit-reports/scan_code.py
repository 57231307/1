#!/usr/bin/env python3
"""D08/D09/D10 复审脚本：扫描 Rust 函数长度和文件长度"""
import os
import re
import sys
from pathlib import Path

BACKEND_SRC = "/workspace/backend/src"


class LineCounter:
    """基于括号深度追踪的函数长度统计"""

    def __init__(self, lines):
        self.lines = lines
        self.n = len(lines)

    def scan(self):
        """返回 [(func_name, start_line, length, is_test), ...]"""
        text = "".join(self.lines)
        masked = self._mask_strings_and_comments(text)
        masked_lines = masked.split("\n")

        results = []
        brace_stack = []
        depth = 0
        pending_attrs = []
        in_cfg_test_context = False

        i = 0
        n_lines = len(masked_lines)
        while i < n_lines:
            line = masked_lines[i]
            stripped = line.strip()

            if stripped.startswith("#["):
                if "cfg(test)" in stripped:
                    pending_attrs.append("cfg(test)")
                elif re.match(r"#\[\s*test\s*\]", stripped):
                    pending_attrs.append("test")
                i += 1
                continue

            mod_match = re.match(r"\s*mod\s+(\w+)\s*\{", line)
            if mod_match:
                mod_name = mod_match.group(1)
                is_cfg_test = "cfg(test)" in pending_attrs or in_cfg_test_context
                brace_pos = line.find("{")
                brace_stack.append(("mod", is_cfg_test, depth, mod_name, i + 1))
                after = line[brace_pos + 1:]
                depth += 1 + after.count("{") - after.count("}")
                if is_cfg_test:
                    in_cfg_test_context = True
                pending_attrs = []
                i += 1
                continue

            fn_match = re.match(
                r"\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:unsafe\s+)?(?:const\s+)?fn\s+(\w+)",
                line,
            )
            if fn_match and "{" in line:
                func_name = fn_match.group(1)
                is_test = "test" in pending_attrs or in_cfg_test_context
                brace_pos = line.find("{")
                start_line = i + 1
                start_depth = depth
                after = line[brace_pos + 1:]
                depth += 1 + after.count("{") - after.count("}")
                func_end_line = i + 1
                j = i
                while j + 1 < n_lines and depth > start_depth:
                    j += 1
                    ln = masked_lines[j]
                    depth += ln.count("{") - ln.count("}")
                    func_end_line = j + 1
                length = func_end_line - start_line + 1
                results.append((func_name, start_line, length, is_test))
                pending_attrs = []
                i = j + 1 if j > i else i + 1
                continue

            old_depth = depth
            depth += line.count("{") - line.count("}")

            while brace_stack and depth < brace_stack[-1][2] + 1:
                kind, is_cfg_test, sd, name, sl = brace_stack.pop()
                if kind == "mod" and is_cfg_test:
                    in_cfg_test_context = any(e[1] for e in brace_stack)

            pending_attrs = []
            i += 1

        return results

    def _mask_strings_and_comments(self, text):
        """把字符串和注释内容替换为空格（保留换行符）"""
        result = []
        i = 0
        n = len(text)
        in_string = False
        in_char = False
        in_line_comment = False
        in_block_comment = False
        in_raw_string = False
        raw_string_hashes = 0

        while i < n:
            c = text[i]

            if in_line_comment:
                if c == "\n":
                    in_line_comment = False
                    result.append(c)
                else:
                    result.append(" " if c != "\t" else "\t")
                i += 1
                continue

            if in_block_comment:
                if c == "*" and i + 1 < n and text[i + 1] == "/":
                    in_block_comment = False
                    result.append(" ")
                    result.append(" ")
                    i += 2
                    continue
                else:
                    if c == "\n":
                        result.append(c)
                    else:
                        result.append(" " if c != "\t" else "\t")
                    i += 1
                    continue

            if in_string:
                if c == "\\":
                    result.append(" ")
                    if i + 1 < n:
                        result.append(" ")
                        i += 2
                    else:
                        i += 1
                    continue
                elif c == '"':
                    in_string = False
                    result.append(" ")
                    i += 1
                    continue
                else:
                    if c == "\n":
                        result.append(c)
                    else:
                        result.append(" " if c != "\t" else "\t")
                    i += 1
                    continue

            if in_char:
                if c == "\\":
                    result.append(" ")
                    if i + 1 < n:
                        result.append(" ")
                        i += 2
                    else:
                        i += 1
                    continue
                elif c == "'":
                    in_char = False
                    result.append(" ")
                    i += 1
                    continue
                else:
                    if c == "\n":
                        result.append(c)
                    else:
                        result.append(" " if c != "\t" else "\t")
                    i += 1
                    continue

            if in_raw_string:
                if c == '"':
                    matched = True
                    for k in range(raw_string_hashes):
                        if i + 1 + k >= n or text[i + 1 + k] != "#":
                            matched = False
                            break
                    if matched:
                        in_raw_string = False
                        result.append(" ")
                        for k in range(raw_string_hashes):
                            result.append(" ")
                        i += 1 + raw_string_hashes
                        raw_string_hashes = 0
                        continue
                    else:
                        result.append(" ")
                        i += 1
                        continue
                else:
                    if c == "\n":
                        result.append(c)
                    else:
                        result.append(" " if c != "\t" else "\t")
                    i += 1
                    continue

            if c == "/" and i + 1 < n and text[i + 1] == "/":
                in_line_comment = True
                result.append(" ")
                result.append(" ")
                i += 2
                continue
            elif c == "/" and i + 1 < n and text[i + 1] == "*":
                in_block_comment = True
                result.append(" ")
                result.append(" ")
                i += 2
                continue
            elif c == '"':
                in_string = True
                result.append(" ")
                i += 1
                continue
            elif c == "'":
                if i + 2 < n and text[i + 2] == "'" and text[i + 1] != "\\":
                    in_char = True
                    result.append(" ")
                    i += 1
                    continue
                elif i + 3 < n and text[i + 1] == "\\" and text[i + 3] == "'":
                    in_char = True
                    result.append(" ")
                    i += 1
                    continue
                else:
                    result.append(c)
                    i += 1
                    continue
            elif c == "r" and i + 1 < n and text[i + 1] == "#":
                j = i + 1
                hashes = 0
                while j < n and text[j] == "#":
                    hashes += 1
                    j += 1
                if j < n and text[j] == '"':
                    in_raw_string = True
                    raw_string_hashes = hashes
                    result.append(" ")
                    for k in range(hashes + 1):
                        result.append(" ")
                    i = j + 1
                    continue
                else:
                    result.append(c)
                    i += 1
                    continue
            elif c == "r" and i + 1 < n and text[i + 1] == '"':
                in_raw_string = True
                raw_string_hashes = 0
                result.append(" ")
                result.append(" ")
                i += 2
                continue
            else:
                result.append(c)
                i += 1
                continue

        return "".join(result)


def scan_file(filepath):
    try:
        with open(filepath, "r", encoding="utf-8", errors="replace") as f:
            lines = f.readlines()
    except Exception:
        return [], 0
    counter = LineCounter(lines)
    funcs = counter.scan()
    return funcs, len(lines)


def main():
    rs_files = []
    for root, dirs, files in os.walk(BACKEND_SRC):
        for fname in files:
            if fname.endswith(".rs"):
                rs_files.append(os.path.join(root, fname))

    print(f"扫描目录: {BACKEND_SRC}")
    print(f"找到 .rs 文件数: {len(rs_files)}")
    print()

    all_funcs = []
    file_lengths = []

    for fp in rs_files:
        funcs, line_count = scan_file(fp)
        file_lengths.append((fp, line_count))
        for fname, sl, length, is_test in funcs:
            all_funcs.append((fname, sl, length, is_test, fp))

    non_test_funcs = [f for f in all_funcs if not f[3]]
    d08_funcs = [f for f in non_test_funcs if f[2] > 80]
    d08_funcs.sort(key=lambda x: -x[2])

    print("=" * 80)
    print("D08 复审: >80 行非测试函数")
    print("=" * 80)
    print(f">80 行非测试函数数量: {len(d08_funcs)}")
    print()
    print("前 10 名最长非测试函数:")
    print(f"{'排名':<4} {'函数名':<40} {'行数':<6} {'文件路径:行号'}")
    print("-" * 100)
    for idx, (fname, sl, length, is_test, fp) in enumerate(d08_funcs[:10], 1):
        rel_path = os.path.relpath(fp, "/workspace")
        print(f"{idx:<4} {fname:<40} {length:<6} {rel_path}:{sl}")
    print()

    d09_funcs = [f for f in non_test_funcs if f[2] > 100]
    d09_funcs.sort(key=lambda x: -x[2])

    print("=" * 80)
    print("D09 复审: >100 行非测试函数")
    print("=" * 80)
    print(f">100 行非测试函数数量: {len(d09_funcs)}")
    print()
    print("完整清单:")
    print(f"{'排名':<4} {'函数名':<40} {'行数':<6} {'文件路径:行号'}")
    print("-" * 100)
    for idx, (fname, sl, length, is_test, fp) in enumerate(d09_funcs, 1):
        rel_path = os.path.relpath(fp, "/workspace")
        print(f"{idx:<4} {fname:<40} {length:<6} {rel_path}:{sl}")
    print()

    file_lengths.sort(key=lambda x: -x[1])
    d10_files = [f for f in file_lengths if f[1] > 1000]

    print("=" * 80)
    print("D10 复审: >1000 行文件")
    print("=" * 80)
    print(f">1000 行文件数量: {len(d10_files)}")
    print()
    print("前 20 名最长 .rs 文件:")
    print(f"{'排名':<4} {'行数':<6} {'文件路径'}")
    print("-" * 100)
    for idx, (fp, lc) in enumerate(file_lengths[:20], 1):
        rel_path = os.path.relpath(fp, "/workspace")
        print(f"{idx:<4} {lc:<6} {rel_path}")
    print()

    print("=" * 80)
    print("复审结论")
    print("=" * 80)
    d08_done = len(d08_funcs) == 0
    d09_done = len(d09_funcs) == 0
    d10_done = len(d10_files) == 0
    print(f"D08 (>80 行非测试函数拆分): {'DONE' if d08_done else 'NOT_DONE'} (剩余 {len(d08_funcs)} 个)")
    print(f"D09 (>100 行非测试函数拆分): {'DONE' if d09_done else 'NOT_DONE'} (剩余 {len(d09_funcs)} 个)")
    print(f"D10 (>1000 行文件拆分): {'DONE' if d10_done else 'NOT_DONE'} (剩余 {len(d10_files)} 个)")
    print(f"三项全部完成: {'YES' if (d08_done and d09_done and d10_done) else 'NO'}")


if __name__ == "__main__":
    main()
