#!/usr/bin/env python3
"""逐条核对 APK 清单里的权限、硬件特性与明文流量。

调用方 .github/workflows/android.yml 的严格冒烟·3 与 release APK 校验两步，两步都先跑一次
      --self-test 再断言真产物。
入参 1) APK 路径 2) aapt2 路径，或单参 --self-test（不需要 APK 与 aapt2）。
做什么 aapt2 dump xmltree 解析 AndroidManifest.xml，按 REQUIRED_PERMISSIONS 断言七条权限：
      期望值为整数时要求该属性存在且解析后相等，期望值为 None 时要求该属性不存在；
      两条硬件特性 required=false，usesCleartextTraffic 为真。不符项全部收集完再退出，
      一条红不遮蔽后续判据。
存哪里 只读 APK，不写文件。
"""

import re
import subprocess
import sys

REQUIRED_PERMISSIONS = {
    "android.permission.INTERNET": None,
    "android.permission.CAMERA": None,
    "android.permission.BLUETOOTH": 30,
    "android.permission.BLUETOOTH_ADMIN": 30,
    "android.permission.BLUETOOTH_CONNECT": None,
    "android.permission.READ_EXTERNAL_STORAGE": 32,
    "android.permission.WRITE_EXTERNAL_STORAGE": 28,
}

REQUIRED_FEATURES = ("android.hardware.bluetooth", "android.hardware.camera")


def fail(message: str) -> None:
    print("::error::" + message, file=sys.stderr)
    sys.exit(1)


def parse_elements(tree: str) -> list[dict]:
    """按缩进无关的 E 行切分元素，属性行归属最近的元素。"""
    elements: list[dict] = []
    for raw in tree.splitlines():
        line = raw.strip()
        if line.startswith("E: "):
            elements.append({"header": line, "attrs": []})
        elif line.startswith("A: ") and elements:
            elements[-1]["attrs"].append(line)
    return elements


def attr_value(block: dict, attr: str) -> str | None:
    pattern = re.compile(r":" + re.escape(attr) + r"\(0x[0-9a-fA-F]+\)=(.*)$")
    for line in block["attrs"]:
        matched = pattern.search(line)
        if matched:
            return matched.group(1)
    return None


def attr_string(block: dict, attr: str) -> str | None:
    value = attr_value(block, attr)
    if value is None:
        return None
    quoted = re.search(r'"([^"]+)"', value)
    return quoted.group(1) if quoted else None


def parse_int(value: str | None) -> int | None:
    """把 aapt2 dump 的属性原始值解析成整数，读不出来返回 None。

    aapt2 对同一属性有多种落法：整型在新版 build-tools 直接给十进制裸值（`=30`），
    旧版给 `(type 0x10)0x1e`；布尔给 `=false` 或 `(type 0x12)0x0`，真值为 `0xffffffff`；
    值后面还可能跟一段 `(Raw: ...)` 回显。四种落值形态都要认。调用方用"原始值在不在"
    区分"清单没写"与"写了但读不懂"，后者必须判红，不许退化成 None 冒充没设置。
    """
    if value is None:
        return None
    # aapt2 可能在值后再跟一段 (Raw: ...) 回显，只剥这段尾巴；不能按空白切词，
    # 否则 `(type 0x10)0x1e` 自身含空格会被切成 `(type` 而误判成读不懂。
    text = re.sub(r"\s*\(Raw:.*$", "", value).strip()
    if not text:
        return None
    matched = re.search(r"\(type 0x[0-9a-fA-F]+\)(0x[0-9a-fA-F]+)", text)
    if matched:
        text = matched.group(1)
    lowered = text.lower()
    if lowered == "true":
        return 1
    if lowered == "false":
        return 0
    try:
        return int(text, 16) if lowered.startswith("0x") else int(text)
    except ValueError:
        return None


def dump_block(block: dict, limit: int = 8) -> str:
    """把元素的原始属性行拼成一段可读文本，供判红时点在日志里留证。"""
    lines = [block["header"]] + block["attrs"][:limit]
    return "\n      ".join(line.strip() for line in lines)


def collect_permissions(elements: list[dict]) -> dict[str, list[dict]]:
    """按权限名归集 uses-permission 元素，同名多条全部保留。"""
    collected: dict[str, list[dict]] = {}
    for block in elements:
        if not block["header"].startswith("E: uses-permission "):
            continue
        name = attr_string(block, "name")
        if name:
            collected.setdefault(name, []).append(block)
    return collected


def evaluate(elements: list[dict]) -> list[str]:
    """对已解析的清单元素跑完全部判据，返回不符项文案列表，空列表即全通过。

    入参 parse_elements 的产物，调用方 main 与 self_test。判据一次跑完再交回调用方决定
    退出时机：首个不匹配就 sys.exit 会让其后的权限、硬件特性与明文流量在本轮永不被评估。
    同名权限重复声明同样按不符项返回，重复条目使"取哪一条比对"失去意义，必须点名原始条目。
    """
    problems: list[str] = []
    collected = collect_permissions(elements)
    for name, blocks in sorted(collected.items()):
        raws = [attr_value(b, "maxSdkVersion") for b in blocks]
        print(f"APK 清单权限 {name} 声明 {len(blocks)} 条 maxSdkVersion 原始值={raws}")

    for name, blocks in sorted((k, v) for k, v in collected.items() if len(v) > 1):
        detail = "\n".join(f"      {dump_block(b)}" for b in blocks)
        problems.append(
            "同一权限在 APK 清单里重复声明，低版本专用权限被重复的无 maxSdkVersion "
            f"条目覆盖后会在 Android 12 上继续索取：\n  {name}\n{detail}"
        )

    for name, expected in REQUIRED_PERMISSIONS.items():
        blocks = collected.get(name)
        if not blocks:
            problems.append(f"{name} 清单里没有该权限条目")
            continue
        raw = attr_value(blocks[0], "maxSdkVersion")
        evidence = dump_block(blocks[0])
        if expected is None:
            if raw is None:
                print(f"权限通过 {name} 未声明 maxSdkVersion")
            else:
                problems.append(
                    f"{name} 不应带 maxSdkVersion（Android 12 起为运行时权限，封顶会把该权限"
                    f"在 12 以上摘掉），实际原始值 {raw}，APK 清单原始条目：\n      {evidence}"
                )
            continue
        actual = parse_int(raw)
        if actual is None:
            problems.append(
                f"{name} 的 maxSdkVersion 原始值 {raw} 各落值形态都解析不出来，"
                f"校验脚本需补形态而不是放行，APK 清单原始条目：\n      {evidence}"
            )
        elif actual != expected:
            problems.append(
                f"{name} 的 maxSdkVersion 应为 {expected}，实际 {actual}，"
                f"APK 清单原始条目：\n      {evidence}"
            )
        else:
            print(f"权限通过 {name} maxSdkVersion={actual}")

    features: dict[str, tuple[str | None, int | None]] = {}
    for block in elements:
        if not block["header"].startswith("E: uses-feature "):
            continue
        name = attr_string(block, "name")
        if name:
            raw = attr_value(block, "required")
            features[name] = (raw, parse_int(raw))
    for name in REQUIRED_FEATURES:
        if name not in features:
            problems.append(f"硬件声明 {name} 清单里不存在")
            continue
        raw, required = features[name]
        if required is None:
            problems.append(f"硬件声明 {name} 的 required 原始值 {raw} 解析不出布尔")
        elif required != 0:
            problems.append(
                f"硬件声明 {name} 应存在且 required=false，实际原始值 {raw} 解析值 {required}"
            )
        else:
            print(f"硬件声明通过 {name} required=false")

    cleartext = None
    for block in elements:
        if block["header"].startswith("E: application "):
            cleartext = attr_value(block, "usesCleartextTraffic")
            break
    if cleartext is None:
        problems.append("APK 未声明明文流量开关")
    elif parse_int(cleartext) in (None, 0):
        problems.append("APK 未允许明文流量，实际原始值 " + cleartext)
    else:
        print("明文流量通过 usesCleartextTraffic=true")

    return problems


def self_test() -> None:
    """离线校验解析层与判定层，不需要 APK 与 aapt2。

    调用方 main 的 --self-test 分支，由 android.yml 的 APK 清单校验步与 release APK 校验步
    在断言真产物之前各调一次。入参无。夹具是 aapt2 dump xmltree 的属性行形态，含 CI 上真出
    现过的十进制裸值与带 (Raw:) 回显尾两种，外加 type 前缀、裸十六进制与布尔落值。
    """
    ns = "http://schemas.android.com/apk/res/android"

    def perm_line(name: str, max_sdk: str | None) -> str:
        lines = [
            "E: uses-permission (line=18)",
            f'  A: {ns}:name(0x01010003)="{name}"',
        ]
        if max_sdk is not None:
            lines.append(f"  A: {ns}:maxSdkVersion(0x01010271)={max_sdk}")
        return "\n".join(lines)

    def feature_line(name: str, required: str) -> str:
        return "\n".join(
            [
                "E: uses-feature (line=30)",
                f'  A: {ns}:name(0x01010003)="{name}"',
                f"  A: {ns}:required(0x0101000c)={required}",
            ]
        )

    def app_line(cleartext: str) -> str:
        return "\n".join(
            [
                "E: application (line=40)",
                f"  A: {ns}:usesCleartextTraffic(0x0101026d)={cleartext}",
            ]
        )

    def manifest(
        internet: str | None = None,
        bluetooth: str = "30",
        bluetooth_admin: str = "(type 0x10)0x1e (Raw: 30)",
        read_storage: str = "0x20",
        write_storage: str | None = "28 (Raw: 28)",
        bluetooth_feature: str = "false",
        camera_feature: str = "(type 0x12)0x0",
        cleartext: str = "true",
    ) -> str:
        parts = [
            perm_line("android.permission.INTERNET", internet),
            perm_line("android.permission.CAMERA", None),
            perm_line("android.permission.BLUETOOTH", bluetooth),
            perm_line("android.permission.BLUETOOTH_ADMIN", bluetooth_admin),
            perm_line("android.permission.BLUETOOTH_CONNECT", None),
            perm_line("android.permission.READ_EXTERNAL_STORAGE", read_storage),
        ]
        if write_storage is not None:
            parts.append(perm_line("android.permission.WRITE_EXTERNAL_STORAGE", write_storage))
        parts.append(feature_line("android.hardware.bluetooth", bluetooth_feature))
        parts.append(feature_line("android.hardware.camera", camera_feature))
        parts.append(app_line(cleartext))
        return "\n".join(parts)

    checks: list[tuple[str, bool]] = [
        ("十进制裸值 30 解析为 30", parse_int("30") == 30),
        ("带 (Raw:) 回显尾仍解析为 30", parse_int("30 (Raw: 30)") == 30),
        ("type 0x10 十六进制 0x1e 解析为 30", parse_int("(type 0x10)0x1e") == 30),
        (
            "type 前缀与 (Raw:) 回显尾同现仍解析为 30",
            parse_int("(type 0x10)0x1e (Raw: 30)") == 30,
        ),
        ("裸 0x20 解析为 32", parse_int("0x20") == 32),
        ("布尔 false 解析为 0", parse_int("false") == 0),
        (
            "布尔真值 0xffffffff 解析为非零",
            parse_int("(type 0x12)0xffffffff") == 0xFFFFFFFF,
        ),
        ("读不懂的值解析为 None 而不是冒充没设置", parse_int("abc") is None),
        ("七条权限齐备且封顶值正确的清单全通过", evaluate(parse_elements(manifest())) == []),
        (
            "不该带封顶的 INTERNET 带了值被判红",
            any(
                "INTERNET 不应带 maxSdkVersion" in p
                for p in evaluate(parse_elements(manifest(internet="30")))
            ),
        ),
        (
            "封顶值 29 与期望 30 不符被判红",
            any(
                "应为 30，实际 29" in p
                for p in evaluate(parse_elements(manifest(bluetooth="29")))
            ),
        ),
        (
            "封顶值写成读不懂的形态被判红而不是放行",
            any(
                "都解析不出来" in p
                for p in evaluate(parse_elements(manifest(bluetooth="abc")))
            ),
        ),
        (
            "缺失 WRITE_EXTERNAL_STORAGE 被点名",
            any(
                "WRITE_EXTERNAL_STORAGE 清单里没有该权限条目" in p
                for p in evaluate(parse_elements(manifest(write_storage=None)))
            ),
        ),
        (
            "硬件特性 required 为真被判红",
            any(
                "android.hardware.bluetooth" in p
                for p in evaluate(parse_elements(manifest(bluetooth_feature="(type 0x12)0xffffffff")))
            ),
        ),
        (
            "明文流量为假被判红",
            any(
                "未允许明文流量" in p
                for p in evaluate(parse_elements(manifest(cleartext="false")))
            ),
        ),
    ]

    broken = [title for title, ok in checks if not ok]
    if broken:
        fail("校验脚本自检未通过 " + str(len(broken)) + " 条：" + ", ".join(broken))
    print("校验脚本自检通过 " + str(len(checks)) + " 条，覆盖四种落值形态与三类判据")


def main() -> None:
    if len(sys.argv) == 2 and sys.argv[1] == "--self-test":
        self_test()
        return
    if len(sys.argv) != 3:
        fail("用法 assert-apk-permissions.py APK 路径 aapt2 路径，或 --self-test")
    apk, aapt2 = sys.argv[1], sys.argv[2]

    proc = subprocess.run(
        [aapt2, "dump", "xmltree", "--file", "AndroidManifest.xml", apk],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        fail("aapt2 解析清单失败：" + proc.stderr.strip())

    elements = parse_elements(proc.stdout)
    if not elements:
        fail("aapt2 未解析出任何清单元素，输出格式可能已变")

    problems = evaluate(elements)
    if problems:
        fail(
            "APK 清单校验不符，共 "
            + str(len(problems))
            + " 条：\n  "
            + "\n  ".join(problems)
        )


if __name__ == "__main__":
    main()
