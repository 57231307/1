#!/usr/bin/env python3
"""逐条核对 APK 清单里的权限、硬件特性与明文流量。

调用方 .github/workflows/android.yml 的严格冒烟·3 与 release APK 校验两步。
入参 1) APK 路径 2) aapt2 路径。
做什么 aapt2 dump xmltree 解析 AndroidManifest.xml，断言七条权限存在且 maxSdkVersion 精确，
      两条硬件特性 required=false，usesCleartextTraffic 为 true；任一不符以非零码退出。
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


def attr_int(block: dict, attr: str) -> int | None:
    value = attr_value(block, attr)
    if value is None:
        return None
    matched = re.search(r"\(type 0x[0-9a-fA-F]+\)(0x[0-9a-fA-F]+)", value)
    return int(matched.group(1), 16) if matched else None


def main() -> None:
    if len(sys.argv) != 3:
        fail("用法 assert-apk-permissions.py APK 路径 aapt2 路径")
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

    permissions: dict[str, int | None] = {}
    for block in elements:
        if not block["header"].startswith("E: uses-permission "):
            continue
        name = attr_string(block, "name")
        if name:
            permissions[name] = attr_int(block, "maxSdkVersion")

    missing = sorted(set(REQUIRED_PERMISSIONS) - set(permissions))
    if missing:
        fail("APK 缺权限 " + ", ".join(missing))
    for name, expected in REQUIRED_PERMISSIONS.items():
        actual = permissions[name]
        if actual != expected:
            fail(f"{name} 的 maxSdkVersion 应为 {expected}，实际 {actual}")
        print(f"权限通过 {name} maxSdkVersion={actual}")

    features: dict[str, int | None] = {}
    for block in elements:
        if not block["header"].startswith("E: uses-feature "):
            continue
        name = attr_string(block, "name")
        if name:
            features[name] = attr_int(block, "required")
    for name in REQUIRED_FEATURES:
        if features.get(name) != 0:
            fail(f"硬件声明 {name} 应存在且 required=false，实际 {features.get(name)}")
        print(f"硬件声明通过 {name} required=false")

    cleartext = None
    for block in elements:
        if block["header"].startswith("E: application "):
            cleartext = attr_value(block, "usesCleartextTraffic")
            break
    if cleartext is None:
        fail("APK 未声明明文流量开关")
    if not re.search(r"=true|0xffffffff", cleartext):
        fail("APK 未允许明文流量，实际 " + cleartext)
    print("明文流量通过 usesCleartextTraffic=true")


if __name__ == "__main__":
    main()
