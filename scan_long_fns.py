#!/usr/bin/env python3
"""
扫描 Rust 源文件，找出超过 80 行的非测试函数。

算法要点：
- 基于字符级状态机，正确处理 // 行注释、/* */ 块注释、
  字符串字面量 "..."、字符字面量 '...'、原始字符串 r"..." / r#"..."#、
  原始字节字符串 br"..."、字节字符串 b"..."、字节字符 b'...'。
- 跟踪大括号深度定位函数体范围。
- 排除 #[test] / #[tokio::test] 标注的函数。
- 排除 mod tests / mod test 模块（任何深度）内的函数。
"""

import os
import re
import sys

SRC_DIR = "/workspace/backend/src"
THRESHOLD = 80


def strip_to_code(lines):
    """将源代码行转换为纯代码字符流，同时记录每个字符所属的原始行号 (1-based)。
    处理：行注释 //、块注释 /* */、字符串、字符、原始字符串等。
    返回 list of (char, line_no_0based)
    """
    n = len(lines)
    out = []
    line_no = 0  # 0-based
    state = "code"  # code, line_comment, block_comment, string, char, raw_string
    raw_hash_count = 0

    while line_no < n:
        line = lines[line_no]
        col = 0
        L = len(line)
        while col < L:
            c = line[col]
            nxt = line[col + 1] if col + 1 < L else ""

            if state == "code":
                if c == "/" and nxt == "/":
                    state = "line_comment"
                    col += 2
                    continue
                if c == "/" and nxt == "*":
                    state = "block_comment"
                    col += 2
                    continue
                # 原始字符串 r"..." / r#"..."#
                if c == "r" and nxt == '"':
                    state = "raw_string"
                    raw_hash_count = 0
                    col += 2
                    continue
                if c == "r" and nxt == "#":
                    j = col + 1
                    hc = 0
                    while j < L and line[j] == "#":
                        hc += 1
                        j += 1
                    if j < L and line[j] == '"':
                        state = "raw_string"
                        raw_hash_count = hc
                        col = j + 1
                        continue
                    out.append((c, line_no))
                    col += 1
                    continue
                # 原始字节字符串 br"..."
                if c == "b" and nxt == "r" and col + 2 < L and line[col + 2] == '"':
                    state = "raw_string"
                    raw_hash_count = 0
                    col += 3
                    continue
                if c == "b" and nxt == "r" and col + 2 < L and line[col + 2] == "#":
                    j = col + 3
                    hc = 0
                    while j < L and line[j] == "#":
                        hc += 1
                        j += 1
                    if j < L and line[j] == '"':
                        state = "raw_string"
                        raw_hash_count = hc
                        col = j + 1
                        continue
                    out.append((c, line_no))
                    col += 1
                    continue
                # 字节字符串 b"..."
                if c == "b" and nxt == '"':
                    state = "string"
                    col += 2
                    continue
                # 字节字符 b'...'
                if c == "b" and nxt == "'":
                    state = "char"
                    col += 2
                    continue
                # 字符串
                if c == '"':
                    state = "string"
                    col += 1
                    continue
                # 字符 / 生命周期
                if c == "'":
                    # 启发式：若本行后续 2 个字符内有闭合 '，当作字符字面量
                    k = col + 1
                    found_close = False
                    while k < L:
                        if line[k] == "\\":
                            k += 2
                            continue
                        if line[k] == "'":
                            found_close = True
                            break
                        if line[k] == '"' or line[k] == "/":
                            break
                        k += 1
                    if found_close:
                        out.append((c, line_no))
                        col += 1
                        state = "char"
                        continue
                    else:
                        # 生命周期标注，当普通字符
                        out.append((c, line_no))
                        col += 1
                        continue
                out.append((c, line_no))
                col += 1
                continue

            elif state == "line_comment":
                col = L
                continue

            elif state == "block_comment":
                if c == "*" and nxt == "/":
                    state = "code"
                    col += 2
                    continue
                col += 1
                continue

            elif state == "string":
                if c == "\\":
                    col += 2
                    continue
                if c == '"':
                    state = "code"
                    col += 1
                    continue
                col += 1
                continue

            elif state == "char":
                if c == "\\":
                    col += 2
                    continue
                if c == "'":
                    state = "code"
                    col += 1
                    continue
                col += 1
                continue

            elif state == "raw_string":
                if c == '"':
                    matched = True
                    for h in range(raw_hash_count):
                        if col + 1 + h >= L or line[col + 1 + h] != "#":
                            matched = False
                            break
                    if matched:
                        state = "code"
                        col += 1 + raw_hash_count
                        continue
                col += 1
                continue

        if state == "line_comment":
            state = "code"
        line_no += 1

    return out


def find_functions(lines):
    """从纯代码字符流中找出函数定义。"""
    code = strip_to_code(lines)
    chars = []
    char_lines = []
    for ch, ln in code:
        chars.append(ch)
        char_lines.append(ln)
    text = "".join(chars)
    N = len(text)

    results = []

    # 找所有 fn 关键字位置（词边界）
    fn_positions = []
    for m in re.finditer(r"\bfn\s+([A-Za-z_][A-Za-z0-9_]*)", text):
        fn_positions.append((m.start(), m.group(1)))

    # 扫描 brace 深度，并记录 mod xxx { 的范围
    mod_ranges = []
    mod_stack = []
    depth_at_pos = [0] * (N + 1)
    d = 0
    i = 0
    while i < N:
        c = text[i]
        if c == "{":
            # 检查前面是否有 mod <name>
            j = i - 1
            while j >= 0 and text[j] in " \t\r\n":
                j -= 1
            k = j
            while k >= 0 and (text[k].isalnum() or text[k] == "_"):
                k -= 1
            name = text[k + 1:j + 1]
            m = k
            while m >= 0 and text[m] in " \t\r\n":
                m -= 1
            if m >= 2 and text[m - 2:m + 1] == "mod" and (m - 2 == 0 or not (text[m - 3].isalnum() or text[m - 3] == "_")):
                mod_stack.append((name, d, i))
            d += 1
            depth_at_pos[i] = d
            i += 1
            continue
        elif c == "}":
            depth_at_pos[i] = d
            if mod_stack and mod_stack[-1][1] == d - 1:
                modname, open_d, open_pos = mod_stack.pop()
                mod_ranges.append((open_pos, i, modname, open_d))
            d -= 1
            if d < 0:
                d = 0
            i += 1
            continue
        else:
            depth_at_pos[i] = d
            i += 1
    depth_at_pos[N] = d

    def in_tests_mod(pos):
        for (s, e, name, _) in mod_ranges:
            if s < pos < e and name in ("tests", "test"):
                return True
        return False

    for fn_pos, fn_name in fn_positions:
        # 检查测试属性
        has_test_attr = False
        p = fn_pos - 1
        while p >= 0 and text[p] in " \t\r\n":
            p -= 1
        attrs = []
        while p >= 0 and text[p] == "]":
            depth_b = 1
            q = p - 1
            while q >= 0 and depth_b > 0:
                if text[q] == "]":
                    depth_b += 1
                elif text[q] == "[":
                    depth_b -= 1
                q -= 1
            if q >= 0 and text[q] == "#":
                attr_content = text[q + 2:p]
                attrs.append(attr_content)
                p = q - 1
                while p >= 0 and text[p] in " \t\r\n":
                    p -= 1
            else:
                break

        for attr in attrs:
            stripped = attr.strip()
            if stripped == "test":
                has_test_attr = True
                break
            if stripped.startswith("tokio::test"):
                has_test_attr = True
                break
            # 兜底：包含 test 标识符
            if re.search(r"\btest\b", attr):
                has_test_attr = True
                break

        # 找函数体
        body_open = -1
        search_from = fn_pos
        while search_from < N:
            ch = text[search_from]
            if ch == ";":
                body_open = -1
                break
            if ch == "{":
                body_open = search_from
                break
            if ch == "}":
                body_open = -1
                break
            search_from += 1

        if body_open == -1:
            continue

        target_depth = depth_at_pos[body_open]
        close_pos = -1
        for k in range(body_open + 1, N):
            if text[k] == "}" and depth_at_pos[k] == target_depth:
                close_pos = k
                break
        if close_pos == -1:
            continue

        start_line = char_lines[fn_pos] + 1
        end_line = char_lines[close_pos] + 1
        line_count = end_line - start_line + 1

        results.append({
            "name": fn_name,
            "start_line": start_line,
            "end_line": end_line,
            "line_count": line_count,
            "in_tests_mod": in_tests_mod(fn_pos),
            "has_test_attr": has_test_attr,
            "file": None,
        })

    return results


def scan_file(path):
    try:
        with open(path, "r", encoding="utf-8", errors="replace") as f:
            lines = f.read().split("\n")
    except Exception:
        return []
    funcs = find_functions(lines)
    for fn in funcs:
        fn["file"] = path
    return funcs


def main():
    all_funcs = []
    rs_files = []
    for root, dirs, files in os.walk(SRC_DIR):
        for fn in files:
            if fn.endswith(".rs"):
                rs_files.append(os.path.join(root, fn))

    rs_files.sort()
    for path in rs_files:
        all_funcs.extend(scan_file(path))

    long_fns = [
        f for f in all_funcs
        if f["line_count"] > THRESHOLD
        and not f["has_test_attr"]
        and not f["in_tests_mod"]
    ]

    long_fns.sort(key=lambda x: (-x["line_count"], x["file"], x["start_line"]))

    print("=" * 100)
    print(f"扫描目录: {SRC_DIR}")
    print(f"扫描 .rs 文件数: {len(rs_files)}")
    print(f"解析出的函数总数: {len(all_funcs)}")
    print(f"超过 {THRESHOLD} 行的非测试函数数量: {len(long_fns)}")
    print("=" * 100)
    print()
    print(f"{'#':<4}{'行数':<7}{'函数名':<40}{'文件路径:行号'}")
    print("-" * 110)
    for idx, f in enumerate(long_fns, 1):
        rel = os.path.relpath(f["file"], "/workspace")
        print(f"{idx:<4}{f['line_count']:<7}{f['name']:<40}{rel}:{f['start_line']}-{f['end_line']}")

    return 0


if __name__ == "__main__":
    sys.exit(main())
