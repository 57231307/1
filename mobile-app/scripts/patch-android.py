#!/usr/bin/env python3
"""在 cargo tauri android init 生成的工程上注入权限与蓝牙打印插件。

调用方 .github/workflows/android.yml 的 注入安卓权限与蓝牙打印插件 步。
入参 无参数，工作目录为 mobile-app。
做什么 1) 向 AndroidManifest.xml 补齐联网/相机/经典蓝牙/文件读写权限与可选用硬件声明
      2) 把 android/BluetoothPrintPlugin.kt 复制进生成工程的 com.bingxi.erp 包目录
      3) 逐项断言注入结果，模板变化时判红而不是静默跳过。
存哪里 只改 CI 现场生成的 src-tauri/gen/android，不改版本库里的模板。
"""

import re
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "src-tauri/gen/android/app/src/main/AndroidManifest.xml"
PLUGIN_SRC = ROOT / "android/BluetoothPrintPlugin.kt"
PLUGIN_DST = (
    ROOT
    / "src-tauri/gen/android/app/src/main/java/com/bingxi/erp/BluetoothPrintPlugin.kt"
)

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


def copy_plugin() -> None:
    if not PLUGIN_SRC.is_file():
        fail(f"未找到插件源文件：{PLUGIN_SRC}")
    source = PLUGIN_SRC.read_text(encoding="utf-8")
    if "class BluetoothPrintPlugin" not in source:
        fail("BluetoothPrintPlugin.kt 缺 BluetoothPrintPlugin 类定义")
    if "fun requestCameraPermission" not in source:
        fail("BluetoothPrintPlugin.kt 缺相机权限申请命令，前端扫码前会调用失败")
    if not PLUGIN_DST.parent.is_dir():
        fail(
            f"生成的工程缺包目录 {PLUGIN_DST.parent}，"
            "Tauri 模板包名与 com.bingxi.erp 不一致，需人工校准"
        )
    shutil.copyfile(PLUGIN_SRC, PLUGIN_DST)
    copied = PLUGIN_DST.read_text(encoding="utf-8")
    if "class BluetoothPrintPlugin" not in copied or "fun requestCameraPermission" not in copied:
        fail("插件复制后校验失败")
    print(f"蓝牙打印与权限插件已复制到 {PLUGIN_DST}")


def main() -> None:
    patch_manifest()
    copy_plugin()


if __name__ == "__main__":
    main()
