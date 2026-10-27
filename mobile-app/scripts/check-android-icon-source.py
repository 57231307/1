#!/usr/bin/env python3
"""安卓 launcher 图标源图与生成次序的静态锁。

调用方 .github/workflows/android.yml 的 严格冒烟·4 校验安卓图标源图与生成次序 步，
      在 生成应用图标 与 构建 debug APK 之前跑一次，先 --self-test 再断言真实仓。
入参 无参为对真实仓断言；--self-test 用内置合成夹具自证判据可判红；
      --repo-root <路径> 覆盖仓根（供变异自证指向临时副本，默认脚本上溯两级）。
做什么 1) 断言 APK launcher 图标的打包输入源图 mobile-app/app-icon.png 的 sha256
         等于钉住的新 logo 常量，源图被换成旧标或任何非钉住图即判红；
       2) 解析 .github/workflows/android.yml 的步序，按 run 命令行内容定位
         cargo tauri icon、cargo tauri android init、cargo tauri android build 三步，
         断言 icon 在 init 之后且在 build 之前。icon 排在 init 之前时 gen/android 尚未生成，
         mipmap 无处落地，APK 会回落 Tauri 默认图标，故次序即缺陷判据。
为什么钉源图哈希而非生成物 gen/android 由 CI 现场生成不入库，桌面 icons 也被图标步重生成，
      唯一稳定可钉的打包输入是 app-icon.png；次序判据落在 workflow 文本面，与源图判据互补。
存哪里 只读文件，不写。
"""

import hashlib
import re
import sys
from pathlib import Path

# 新 logo（1024 不透明 master）sha256，branding 提交换入，与首页透明底显示资产同枚 artwork。
NEW_LOGO_SHA256 = "afc5cc2aafac07c62389d7d64baa72bf29d9c2f528d7caf3dcca754c35070384"
ICON_SOURCE_REL = "mobile-app/app-icon.png"
WORKFLOW_REL = ".github/workflows/android.yml"

ICON_CMD = "cargo tauri icon"
INIT_CMD = "cargo tauri android init"
BUILD_CMD = "cargo tauri android build"


def fail(message: str) -> None:
    print("::error::" + message, file=sys.stderr)
    sys.exit(1)


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def parse_steps(text: str) -> list[tuple[str, str]]:
    """把 workflow 文本切成 (步名, 该步 run 全文) 列表，顺序即文件内出现顺序。

    只认 `- name:` 开步与 `run: |` 或 `run: <单行>` 起的命令体，命令体收到下一个顶层
    `- ` 步或 job 边界为止。判据靠 run 内容里的 cargo 命令定位三步，不靠步名，
    步名被改写也不影响判据。
    """
    steps: list[tuple[str, str]] = []
    current_name = ""
    current_run: list[str] = []
    in_run = False
    run_line_indent = 0

    def flush() -> None:
        if current_name or current_run:
            steps.append((current_name, "\n".join(current_run)))

    for raw in text.splitlines():
        stripped = raw.strip()
        name_match = re.match(r"^-\s+name:\s*(.*)$", stripped)
        if name_match:
            flush()
            current_name = name_match.group(1).strip().strip("'\"")
            current_run = []
            in_run = False
            continue
        # run 正文必须比 run: 关键字行缩进更深；一旦某非空行缩进回到 run: 同级或更浅
        # （下一个 step 键、env、uses 等），即结束采集，避免把兄弟键吞进 run。
        if in_run and stripped and not raw.startswith(" " * (run_line_indent + 1)):
            in_run = False
        run_match = re.match(r"^run:\s*(.*)$", stripped)
        if run_match:
            in_run = True
            run_line_indent = len(raw) - len(raw.lstrip())
            inline = run_match.group(1).strip()
            if inline and inline != "|":
                current_run.append(inline)
            continue
        if in_run:
            current_run.append(raw)
    flush()
    return steps


def find_step(steps: list[tuple[str, str]], needle: str) -> int:
    """返回 run 内容首次包含 needle 的步下标，找不到返回 -1。"""
    for index, (_name, run_blob) in enumerate(steps):
        if needle in run_blob:
            return index
    return -1


def check_order(steps: list[tuple[str, str]]) -> list[str]:
    """对已解析步序跑全部次序判据，返回不符项列表，空列表即通过。"""
    problems: list[str] = []
    icon_idx = find_step(steps, ICON_CMD)
    init_idx = find_step(steps, INIT_CMD)
    build_idx = find_step(steps, BUILD_CMD)
    for label, idx in (("生成图标 " + ICON_CMD, icon_idx),
                       ("初始化工程 " + INIT_CMD, init_idx),
                       ("构建 APK " + BUILD_CMD, build_idx)):
        if idx < 0:
            problems.append(f"workflow 里找不到含 {label} 的步，判据无法成立")
    if min(icon_idx, init_idx, build_idx) >= 0:
        if not init_idx < icon_idx:
            problems.append(
                f"图标步（下标 {icon_idx}）未在 init 步（下标 {init_idx}）之后，"
                "gen/android 尚未生成，mipmap 无处落地，APK 会回落默认图标")
        if not icon_idx < build_idx:
            problems.append(
                f"图标步（下标 {icon_idx}）未在 build 步（下标 {build_idx}）之前，"
                "生成的 mipmap 不会被本次构建消费")
    return problems


def check_source(root: Path) -> list[str]:
    """对真实源图跑哈希钉判据，返回不符项列表。"""
    problems: list[str] = []
    icon_path = root / ICON_SOURCE_REL
    if not icon_path.is_file():
        problems.append(f"launcher 图标源图不存在：{icon_path}")
        return problems
    actual = sha256_file(icon_path)
    if actual != NEW_LOGO_SHA256:
        problems.append(
            f"launcher 图标源图 {ICON_SOURCE_REL} 哈希 {actual} 不等于钉住的新 logo "
            f"{NEW_LOGO_SHA256}，源图被换成旧标或非钉图，APK 图标会错")
    else:
        print(f"图标源图通过 {ICON_SOURCE_REL} sha256={actual}")
    return problems


def check_real(root: Path) -> None:
    workflow = root / WORKFLOW_REL
    if not workflow.is_file():
        fail(f"未找到 workflow：{workflow}")
    steps = parse_steps(workflow.read_text(encoding="utf-8"))
    problems = check_source(root) + check_order(steps)
    if problems:
        fail("安卓图标静态锁不符，共 " + str(len(problems)) + " 条：\n  "
             + "\n  ".join(problems))
    icon_idx = find_step(steps, ICON_CMD)
    init_idx = find_step(steps, INIT_CMD)
    build_idx = find_step(steps, BUILD_CMD)
    print(f"安卓图标静态锁通过 源图哈希钉住 且 次序 init({init_idx})<icon({icon_idx})<build({build_idx})")


def self_test() -> None:
    """内置合成夹具自证两条判据都能判红，不需要真实仓文件。

    调用方 main 的 --self-test 分支。次序夹具给三种排布：正确 init-icon-build、
    错误 icon-init-build（本支修复前形态）、错误 init-build-icon；源图夹具用临时文件
    验哈希不符能被抓。任一夹具判据失灵即自检红，防静态锁退化成恒真断言。
    """
    good = (
        "- name: A\n  run: cargo tauri android init\n"
        "- name: B\n  run: cargo tauri icon app-icon.png\n"
        "- name: C\n  run: cargo tauri android build --debug --apk\n"
    )
    icon_before_init = (
        "- name: B\n  run: cargo tauri icon app-icon.png\n"
        "- name: A\n  run: cargo tauri android init\n"
        "- name: C\n  run: cargo tauri android build --debug --apk\n"
    )
    icon_after_build = (
        "- name: A\n  run: cargo tauri android init\n"
        "- name: C\n  run: cargo tauri android build --debug --apk\n"
        "- name: B\n  run: cargo tauri icon app-icon.png\n"
    )
    missing_build = (
        "- name: A\n  run: cargo tauri android init\n"
        "- name: B\n  run: cargo tauri icon app-icon.png\n"
    )
    checks: list[tuple[str, bool]] = [
        ("正确次序 init-icon-build 判通过", check_order(parse_steps(good)) == []),
        ("icon 排在 init 之前判红", check_order(parse_steps(icon_before_init)) != []),
        ("icon 排在 build 之后判红", check_order(parse_steps(icon_after_build)) != []),
        ("缺 build 步判红", check_order(parse_steps(missing_build)) != []),
        ("多行 run 块仍能按命令内容定位步",
         check_order(parse_steps(
             "- name: A\n  run: |\n    set -euo pipefail\n    cargo tauri android init\n"
             "- name: B\n  run: |\n    cargo tauri icon app-icon.png\n"
             "- name: C\n  run: |\n    cargo tauri android build --debug --apk\n")) == []),
    ]

    import tempfile
    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        (root / "mobile-app").mkdir()
        icon = root / ICON_SOURCE_REL
        icon.write_bytes(b"not the real logo bytes")
        checks.append(("源图哈希不符判红", check_source(root) != []))
        icon.write_bytes(b"")
        checks.append(("源图为空判红", check_source(root) != []))

    broken = [title for title, ok in checks if not ok]
    if broken:
        fail("静态锁自检未通过 " + str(len(broken)) + " 条：" + ", ".join(broken))
    print(f"静态锁自检通过 {len(checks)} 条，覆盖次序三种错排与源图两种不符")


def main() -> None:
    args = sys.argv[1:]
    if args == ["--self-test"]:
        self_test()
        return
    root = Path(args[args.index("--repo-root") + 1]) if "--repo-root" in args \
        else Path(__file__).resolve().parent.parent.parent
    check_real(root)


if __name__ == "__main__":
    main()
