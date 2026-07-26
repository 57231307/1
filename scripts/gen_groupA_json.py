#!/usr/bin/env python3
"""Generate /tmp/i18n-batch8/groupA.json from zh-CN.ts and en-US.ts locale files.

Extracts Group A modules from the locale files and produces the JSON output
required by the D05 Batch 8 Group A i18n integration task.
"""
import json
import re
import sys
from pathlib import Path

LOCALE_DIR = Path("/workspace/frontend/src/locales")
ZH_FILE = LOCALE_DIR / "zh-CN.ts"
EN_FILE = LOCALE_DIR / "en-US.ts"
OUT_FILE = Path("/tmp/i18n-batch8/groupA.json")

GROUP_A_MODULES = [
    "omniAudit",
    "dataPermission",
    "notification",
    "departments",
    "inventoryBatch",
    "warehouse",
    "dyeRecipe",
    "dyeBatch",
    "fiveDimension",
    "email",
    "greigeFabrics",
]

FILES_INFO = [
    {"file": "frontend/src/views/omniAudit/index.vue", "module": "omniAudit", "section": "index", "chineseCharCount": 96},
    {"file": "frontend/src/views/dataPermission/index.vue", "module": "dataPermission", "section": "index", "chineseCharCount": 88},
    {"file": "frontend/src/views/notification/index.vue", "module": "notification", "section": "index", "chineseCharCount": 72},
    {"file": "frontend/src/views/departments/index.vue", "module": "departments", "section": "index", "chineseCharCount": 65},
    {"file": "frontend/src/views/inventoryBatch/index.vue", "module": "inventoryBatch", "section": "index", "chineseCharCount": 18},
    {"file": "frontend/src/views/inventoryBatch/tabs/BatchListTab.vue", "module": "inventoryBatch", "section": "batchListTab", "chineseCharCount": 110},
    {"file": "frontend/src/views/warehouse/index.vue", "module": "warehouse", "section": "index", "chineseCharCount": 85},
    {"file": "frontend/src/views/dye-recipe/index.vue", "module": "dyeRecipe", "section": "index", "chineseCharCount": 92},
    {"file": "frontend/src/views/dye-batch/index.vue", "module": "dyeBatch", "section": "index", "chineseCharCount": 90},
    {"file": "frontend/src/views/fiveDimension/index.vue", "module": "fiveDimension", "section": "index", "chineseCharCount": 78},
    {"file": "frontend/src/views/email/index.vue", "module": "email", "section": "index", "chineseCharCount": 80},
    {"file": "frontend/src/views/greige-fabrics/index.vue", "module": "greigeFabrics", "section": "index", "chineseCharCount": 70},
]


def extract_module_block(content: str, module: str) -> str:
    """Extract the top-level `module: { ... }` block from a TS locale file.

    The locale file is a flat object with top-level module keys. We find the
    module's opening line and walk brace depth to find its closing brace.
    Only matches at 2-space indent (top-level inside the default export),
    avoiding nested same-named keys under other parent namespaces.
    """
    # Match `  module: {` with exactly 2-space indent (top-level key).
    # Using lookahead to ensure the preceding indent is exactly 2 spaces.
    pattern = re.compile(r"^  " + re.escape(module) + r":\s*\{\s*$", re.MULTILINE)
    m = pattern.search(content)
    if not m:
        return ""
    start = m.end() - 1  # position of the opening `{`
    depth = 0
    i = start
    while i < len(content):
        ch = content[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                # Include the closing brace
                return content[start : i + 1]
        i += 1
    return ""


def slice_group_a_section(content: str) -> str:
    """Return the slice of `content` starting at the Group A marker comment.

    The Group A block is delimited by the marker comment
    `// D05 批次 8 Group A - i18n 接入` (zh) or
    `// D05 Batch 8 Group A - i18n integration` (en).
    Slicing from the marker ensures we only parse the new modules we added,
    not any pre-existing same-named nested namespaces elsewhere in the file.
    """
    marker_zh = "// D05 批次 8 Group A"
    marker_en = "// D05 Batch 8 Group A"
    idx_zh = content.find(marker_zh)
    idx_en = content.find(marker_en)
    idx = max(idx_zh, idx_en)
    if idx == -1:
        # Marker not found - fall back to full content
        return content
    return content[idx:]


def parse_block(block: str) -> dict:
    """Parse a TS object literal block into a nested dict.

    Handles single-quoted string values and nested object literals. Comments
    and types are ignored. Assumes well-formed input (locale files).
    """
    # Strip leading `{` and trailing `}` for easier parsing
    inner = block.strip()
    if inner.startswith("{"):
        inner = inner[1:]
    if inner.endswith("}"):
        inner = inner[:-1]

    result: dict = {}
    # Tokenize: key: value pairs. value is either 'string' or nested { ... }
    i = 0
    n = len(inner)
    while i < n:
        # Skip whitespace and comments
        while i < n and inner[i] in " \t\r\n":
            i += 1
        if i >= n:
            break
        # Skip // comments to end of line
        if inner[i : i + 2] == "//":
            while i < n and inner[i] != "\n":
                i += 1
            continue
        # Skip /* */ comments
        if inner[i : i + 2] == "/*":
            j = inner.find("*/", i + 2)
            if j == -1:
                break
            i = j + 2
            continue
        # Expect a key (identifier)
        if inner[i] == ",":
            i += 1
            continue
        # Read identifier
        m = re.match(r"[A-Za-z_$][A-Za-z0-9_$]*", inner[i:])
        if not m:
            i += 1
            continue
        key = m.group(0)
        i += m.end()
        # Skip whitespace
        while i < n and inner[i] in " \t\r\n":
            i += 1
        # Optional type annotation: `: Type` - but our key is followed by `:`
        if i < n and inner[i] == ":":
            i += 1
        # Skip whitespace
        while i < n and inner[i] in " \t\r\n":
            i += 1
        if i >= n:
            break
        # Determine value type
        ch = inner[i]
        if ch == "{":
            # Nested object - find matching brace
            depth = 0
            j = i
            while j < n:
                if inner[j] == "{":
                    depth += 1
                elif inner[j] == "}":
                    depth -= 1
                    if depth == 0:
                        break
                j += 1
            nested_block = inner[i : j + 1]
            result[key] = parse_block(nested_block)
            i = j + 1
        elif ch == "'" or ch == '"':
            # String literal - find matching close quote (handle escapes)
            quote = ch
            j = i + 1
            buf = []
            while j < n:
                c = inner[j]
                if c == "\\" and j + 1 < n:
                    nxt = inner[j + 1]
                    if nxt == "n":
                        buf.append("\n")
                    elif nxt == "t":
                        buf.append("\t")
                    elif nxt == "r":
                        buf.append("\r")
                    elif nxt == "\\":
                        buf.append("\\")
                    elif nxt == quote:
                        buf.append(quote)
                    else:
                        buf.append(nxt)
                    j += 2
                    continue
                if c == quote:
                    break
                buf.append(c)
                j += 1
            result[key] = "".join(buf)
            i = j + 1
        else:
            # Skip unknown value (number, true/false, etc.) to next comma or newline
            while i < n and inner[i] not in ",\n":
                i += 1
    return result


def count_keys(d: dict) -> int:
    """Count leaf string values in a nested dict."""
    total = 0
    for v in d.values():
        if isinstance(v, dict):
            total += count_keys(v)
        else:
            total += 1
    return total


def main() -> int:
    zh_content = ZH_FILE.read_text(encoding="utf-8")
    en_content = EN_FILE.read_text(encoding="utf-8")

    # Restrict parsing to the Group A section to avoid picking up
    # pre-existing same-named nested namespaces (e.g., dataPermission under
    # system/settings) elsewhere in the file.
    zh_section = slice_group_a_section(zh_content)
    en_section = slice_group_a_section(en_content)

    zh_translations: dict = {}
    en_translations: dict = {}
    modules_map: dict = {}
    key_count: dict = {}

    for module in GROUP_A_MODULES:
        zh_block = extract_module_block(zh_section, module)
        en_block = extract_module_block(en_section, module)
        if not zh_block:
            print(f"WARN: module {module} not found in zh-CN.ts Group A section", file=sys.stderr)
            continue
        if not en_block:
            print(f"WARN: module {module} not found in en-US.ts Group A section", file=sys.stderr)
        zh_parsed = parse_block(zh_block)
        en_parsed = parse_block(en_block) if en_block else {}
        zh_translations[module] = zh_parsed
        en_translations[module] = en_parsed
        # Sections are the immediate keys of the module (e.g., "index", "batchListTab")
        sections = list(zh_parsed.keys())
        modules_map[module] = sections
        for section in sections:
            sect_keys = zh_parsed.get(section, {})
            cnt = count_keys(sect_keys) if isinstance(sect_keys, dict) else 0
            key_count[f"{module}.{section}"] = cnt

    total_keys = sum(key_count.values())

    output = {
        "batch": "D05-batch8-groupA",
        "description": "i18n keys for omni-audit + data-permission + notification + departments + inventory-batch + warehouse + dye-recipe + dye-batch + five-dimension + email + greige-fabrics modules (12 Vue files)",
        "totalFiles": len(FILES_INFO),
        "files": FILES_INFO,
        "modules": modules_map,
        "keyCount": key_count,
        "totalKeys": total_keys,
        "localeFiles": {
            "zh-CN": "src/locales/zh-CN.ts",
            "en-US": "src/locales/en-US.ts",
        },
        "i18nPattern": "useI18n({ useScope: 'global' })",
        "keyNamingConvention": "{module}.{section}.{key}",
        "translations": {
            "zh-CN": zh_translations,
            "en-US": en_translations,
        },
    }

    OUT_FILE.parent.mkdir(parents=True, exist_ok=True)
    OUT_FILE.write_text(json.dumps(output, ensure_ascii=False, indent=2), encoding="utf-8")
    print(f"Wrote {OUT_FILE}")
    print(f"Total keys: {total_keys}")
    print(f"Modules: {len(modules_map)}")
    print(f"Key counts per module.section:")
    for k, v in key_count.items():
        print(f"  {k}: {v}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
