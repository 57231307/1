#!/usr/bin/env python3
"""在 cargo tauri android init 生成的工程上注入权限、打印插件与启动图标。

调用方 .github/workflows/android.yml 的 注入安卓权限与打印插件 步。
入参 无参数，工作目录为 mobile-app。
做什么 1) 向 AndroidManifest.xml 补齐联网/相机/经典蓝牙/文件读写权限与可选用硬件声明
      2) 把 android/BluetoothPrintPlugin.kt 与 android/SystemPrintPlugin.kt 复制进
         生成工程的 com.bingxi.erp 包目录
      3) 把 生成应用图标 步产出的 src-tauri/icons/android 五档 mipmap 复制进生成工程的
         res 目录，覆盖模板默认图标；图标源缺失或清单图标锚点漂移即判红，不静默跳过
      4) 逐项断言注入结果，模板变化时判红而不是静默跳过。
存哪里 只改 CI 现场生成的 src-tauri/gen/android，不改版本库里的模板。
"""

import hashlib
import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "src-tauri/gen/android/app/src/main/AndroidManifest.xml"
PACKAGE_DIR = ROOT / "src-tauri/gen/android/app/src/main/java/com/bingxi/erp"
ICON_SOURCE_DIR = ROOT / "src-tauri/icons/android"
RES_DIR = ROOT / "src-tauri/gen/android/app/src/main/res"

# 五档密度三件套，与 cargo tauri icon 的安卓输出同名同目，缺一即断链判红。
ICON_DENSITIES = ["mipmap-hdpi", "mipmap-mdpi", "mipmap-xhdpi", "mipmap-xxhdpi", "mipmap-xxxhdpi"]
ICON_FILES = ["ic_launcher.png", "ic_launcher_round.png", "ic_launcher_foreground.png"]

# 源文件, 文件名, 必含的类定义, 必含的命令，缺任一项说明模板与壳侧代码已脱节。
PLUGINS = [
    (
        ROOT / "android/BluetoothPrintPlugin.kt",
        "BluetoothPrintPlugin.kt",
        "class BluetoothPrintPlugin",
        ["fun requestCameraPermission", "fun printText"],
    ),
    (
        ROOT / "android/SystemPrintPlugin.kt",
        "SystemPrintPlugin.kt",
        "class SystemPrintPlugin",
        ["fun printHtml", "fun printCurrentPage"],
    ),
]

# 权限逐条给用途，避免以后有人把某条当冗余删掉。
# 元组依次为 权限名, maxSdkVersion, 额外属性, 用途说明。
PERMISSIONS = [
    # WebView 载入远端 ERP 页面与接口请求
    ("android.permission.INTERNET", None, [], "联网"),
    # Android 11 及以下经典蓝牙为安装期权限
    ("android.permission.BLUETOOTH", "30", [], "低版本经典蓝牙"),
    # Android 11 及以下配对与连接经典蓝牙打印机
    ("android.permission.BLUETOOTH_ADMIN", "30", [], "低版本蓝牙管理"),
    # Android 12 起列出已配对设备与建立 SPP 连接需要运行时权限
    ("android.permission.BLUETOOTH_CONNECT", None, [], "蓝牙连接"),
    # 扫码对话框 getUserMedia 预览相机
    ("android.permission.CAMERA", None, [], "扫码相机"),
    # Android 12 及以下读取用户选择的导入文件
    ("android.permission.READ_EXTERNAL_STORAGE", "32", [], "低版本读取文件"),
    # Android 9 及以下保存导出的表格文件
    ("android.permission.WRITE_EXTERNAL_STORAGE", "28", [], "低版本写入文件"),
]

# 无对应硬件的手机也应能安装，缺失能力由页面降级提示承担。
FEATURES = [
    ("android.hardware.bluetooth", "蓝牙打印"),
    ("android.hardware.camera", "扫码相机"),
]


def fail(message: str) -> None:
    print("::error::" + message, file=sys.stderr)
    sys.exit(1)


def permission_line(name: str, max_sdk: str | None, attrs: list[str]) -> str:
    parts = [f'android:name="{name}"']
    if max_sdk is not None:
        parts.append(f'android:maxSdkVersion="{max_sdk}"')
    parts.extend(attrs)
    return "<uses-permission " + " ".join(parts) + " />"


def feature_line(name: str) -> str:
    return (
        f'<uses-feature android:name="{name}" '
        'android:required="false" />'
    )


def patch_manifest() -> None:
    if not MANIFEST.is_file():
        fail(f"未找到 AndroidManifest.xml：{MANIFEST}，cargo tauri android init 是否成功")
    text = MANIFEST.read_text(encoding="utf-8")
    if "<application" not in text:
        fail("AndroidManifest.xml 缺 <application 标签，Tauri 模板已变，需人工校准")
    if "<manifest" not in text:
        fail("AndroidManifest.xml 缺 <manifest 标签，Tauri 模板已变，需人工校准")

    added = []
    for name, max_sdk, attrs, _purpose in PERMISSIONS:
        if re.search(r'android:name\s*=\s*"' + re.escape(name) + r'"', text):
            continue
        line = permission_line(name, max_sdk, attrs)
        # 统一插在 <application 之前，保留模板原有的 manifest 内容
        text = text.replace("<application", line + "\n\n    <application", 1)
        added.append(name)

    # 已存在但缺 maxSdkVersion 的 BLUETOOTH 也要补齐，否则 Android 12 上会要求不存在的权限
    if 'android.permission.BLUETOOTH"' in text and "maxSdkVersion" not in text.split(
        'android.permission.BLUETOOTH"', 1
    )[1].split(">", 1)[0]:
        text = re.sub(
            r'(<uses-permission android:name="android.permission.BLUETOOTH")',
            r'\1 android:maxSdkVersion="30"',
            text,
            count=1,
        )
        added.append("BLUETOOTH maxSdkVersion")

    for name, _purpose in FEATURES:
        if re.search(r'android:name\s*=\s*"' + re.escape(name) + r'"', text):
            continue
        text = text.replace("<application", feature_line(name) + "\n\n    <application", 1)
        added.append(name)

    MANIFEST.write_text(text, encoding="utf-8")

    for name, max_sdk, _attrs, _purpose in PERMISSIONS:
        if not re.search(r'android:name\s*=\s*"' + re.escape(name) + r'"', text):
            fail(f"清单注入后仍缺权限 {name}")
        if max_sdk is not None and not re.search(
            r'android:name\s*=\s*"' + re.escape(name) + r'"[^>]*maxSdkVersion="'
            + re.escape(max_sdk)
            + r'"',
            text,
        ):
            fail(f"权限 {name} 的 maxSdkVersion 不是 {max_sdk}")
    for name, _purpose in FEATURES:
        if not re.search(r'android:name\s*=\s*"' + re.escape(name) + r'"', text):
            fail(f"清单注入后仍缺硬件声明 {name}")

    print("清单权限注入完成，新增：" + (", ".join(added) if added else "无（已齐备）"))


def copy_plugins() -> None:
    for source_path, file_name, class_marker, command_markers in PLUGINS:
        if not source_path.is_file():
            fail(f"未找到插件源文件：{source_path}")
        source = source_path.read_text(encoding="utf-8")
        if class_marker not in source:
            fail(f"{file_name} 缺 {class_marker} 类定义")
        for marker in command_markers:
            if marker not in source:
                fail(f"{file_name} 缺 {marker}，前端对应命令会调用失败")
        if not PACKAGE_DIR.is_dir():
            fail(
                f"生成的工程缺包目录 {PACKAGE_DIR}，"
                "Tauri 模板包名与 com.bingxi.erp 不一致，需人工校准"
            )
        destination = PACKAGE_DIR / file_name
        shutil.copyfile(source_path, destination)
        copied = destination.read_text(encoding="utf-8")
        if class_marker not in copied:
            fail(f"{file_name} 复制后校验失败")
        print(f"打印插件已复制到 {destination}")


def inject_icons() -> None:
    if not RES_DIR.is_dir():
        fail(f"生成的工程缺 res 目录：{RES_DIR}，cargo tauri android init 是否成功")
    text = MANIFEST.read_text(encoding="utf-8")
    if 'android:icon="@mipmap/ic_launcher"' not in text:
        fail(
            "AndroidManifest.xml 的 android:icon 不再是 @mipmap/ic_launcher，"
            "Tauri 模板图标锚点已变，需人工校准注入目标"
        )
    if not ICON_SOURCE_DIR.is_dir():
        fail(
            f"缺图标生成产物目录 {ICON_SOURCE_DIR}，"
            "生成应用图标步未运行、未先于本步、或输出目录不是 src-tauri/icons"
        )
    digest_for_log = ""
    for density in ICON_DENSITIES:
        source_dir = ICON_SOURCE_DIR / density
        target_dir = RES_DIR / density
        target_dir.mkdir(parents=True, exist_ok=True)
        for file_name in ICON_FILES:
            source_path = source_dir / file_name
            if not source_path.is_file():
                fail(
                    f"缺图标产物 {source_path}，生成应用图标步产物不完整，"
                    "该步必须运行 cargo tauri icon app-icon.png -o src-tauri/icons 且先于构建"
                )
            target_path = target_dir / file_name
            shutil.copyfile(source_path, target_path)
            source_bytes = source_path.read_bytes()
            target_bytes = target_path.read_bytes()
            if source_bytes != target_bytes:
                fail(f"图标复制后字节不一致：{target_path}")
            if density == "mipmap-xxxhdpi" and file_name == "ic_launcher.png":
                digest_for_log = hashlib.sha256(target_bytes).hexdigest()
    print(f"五档 mipmap 共 {len(ICON_DENSITIES) * len(ICON_FILES)} 枚图标已注入生成工程，模板默认图已被覆盖")
    print(f"xxxhdpi ic_launcher.png sha256 {digest_for_log}")


def main() -> None:
    patch_manifest()
    copy_plugins()
    inject_icons()


if __name__ == "__main__":
    main()
