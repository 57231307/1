#!/usr/bin/env bash
# check-rustfmt-content.sh — 内容级 rustfmt 终检门禁
#
# 功能：对 backend 工作区全部 .rs 文件做**内容级** rustfmt 校验（真实 diff 计数 +
#       探针自检），作为 `cargo fmt --all --check` 退出码之外的第二道真实判定。
# 调用方：CI job `ci-fmt-rust` 的 Rust 格式检查步骤（.github/workflows/ci-cd.yml，
#       紧随 cargo fmt 判定之后）；本地开发者在仓库任意目录 `bash
#       backend/scripts/check-rustfmt-content.sh` 直接运行。
# 入参：无（可选 --keep-tmp 保留临时镜像目录用于排障）。
# 传给谁：rustfmt（stable，--edition 取 backend/Cargo.toml，与 cargo fmt 同口径）。
# 产出什么：终端报告——待检文件数、探针自检结果、逐文件点名真实 diff；不写任何文件。
# 存到哪里：不落盘（临时镜像目录结束即删除）。
#
# 方法学口径（每一条都对应历史踩坑，禁止"顺手简化"）：
# 1. 本机 stable rustfmt 不支持 `--stdin-filename`，用它得到的是假清单 ⇒ 正确做法是把
#    文件**去 CR**（sed 's/\r$//'）后以**相对路径**交给 rustfmt --check。绝对路径形态
#    经实测会让 rustfmt 零输出 rc=0（根本没校验任何东西＝空操作假绿通道），故本脚本
#    统一 cd 到临时镜像根目录、只喂相对路径，rustfmt 从文件目录向上自动发现
#    backend/rustfmt.toml（已复制到镜像根），无需 --config-path 绝对路径形态。
# 2. 待检文件集由脚本自行枚举（find backend -name '*.rs'，排除 target），
#    **不依赖 git 工作树是否干净**；工作树不干净时如实报告 dirty 文件清单，
#    校验仍按工作树当前内容执行（与 CI checkout 后行为一致）。
# 3. 探针自检：注入一段已知未格式的 .rs 样例，走与普通文件**完全相同的调用形态**，
#    必须报出 "Diff in"；报不出说明本步已退化成空操作，"0 diff"毫无意义，直接判红
#    （fail-closed，防假绿）。
# 4. 终检是内容级判定：以真实 "Diff in" 计数为准，且实际校验文件数必须 > 0
#    （数量为 0 即判红，防空操作）；rustfmt 输出中无法归类的行一律判红。
# 5. skip_children / #[rustfmt::skip] 口径处理：逐文件校验比模块树递归**更严**，
#    若配置或源码里出现 skip 声明，两者口径冲突，本脚本拒绝执行并判红（口径不明
#    不放行），要求人工核对后显式改脚本，而不是静默给出一个可能与 CI 打架的清单。
#
# 退出码：0=通过；1=存在真实格式 diff；2=rustfmt 不可用；3=口径不明（edition 取不到/
#         存在 skip 声明）；4=探针自检失败（本步已成空操作）；5=待检文件枚举为空；
#         6=rustfmt 输出出现无法归类的行。
set -u

KEEP_TMP=0
for _arg in "$@"; do
    case "$_arg" in
        --keep-tmp) KEEP_TMP=1 ;;
        *)
            echo "[rustfmt-content-gate] 未知参数: $_arg（仅支持 --keep-tmp）" >&2
            exit 3
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BACKEND_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd "$BACKEND_DIR/.." && pwd)"
PREFIX="[rustfmt-content-gate]"

command -v rustfmt >/dev/null 2>&1 || {
    echo "$PREFIX ✗ rustfmt 不在 PATH（CI 需 setup-rust components: rustfmt；本地需 rustup component add rustfmt）→ 未验证，不判绿。" >&2
    exit 2
}

RUSTFMT_CFG="$BACKEND_DIR/rustfmt.toml"
CARGO_TOML="$BACKEND_DIR/Cargo.toml"
[ -f "$RUSTFMT_CFG" ] || { echo "$PREFIX ✗ 缺 rustfmt.toml（$RUSTFMT_CFG）→ 口径不明，不判绿。" >&2; exit 3; }
[ -f "$CARGO_TOML" ] || { echo "$PREFIX ✗ 缺 Cargo.toml（$CARGO_TOML）→ edition 口径取不到，不判绿。" >&2; exit 3; }

# edition 与 cargo fmt 同源：取 workspace 根 Cargo.toml 的 edition 字段
EDITION="$(grep -m1 '^edition' "$CARGO_TOML" | sed -E 's/.*=[[:space:]]*"([^"]*)".*/\1/')"
[ -n "$EDITION" ] || { echo "$PREFIX ✗ Cargo.toml 里读不到 edition → --edition 口径不明，不判绿。" >&2; exit 3; }

# skip 口径冲突检测（见头部第 5 条）：配置级 skip_children 或源码级 #[rustfmt::skip]
# 一旦出现，逐文件校验与模块树递归校验的待检集合不再等价，脚本拒绝给结论。
if grep -Eq '^[[:space:]]*skip_children[[:space:]]*=[[:space:]]*true' "$RUSTFMT_CFG"; then
    echo "$PREFIX ✗ rustfmt.toml 声明了 skip_children=true：逐文件内容级校验会覆盖 cargo fmt 跳过的子树，口径冲突 → 需人工核对后显式调整本脚本，不判绿。" >&2
    exit 3
fi
SKIP_ATTRS="$(find "$BACKEND_DIR" -name '*.rs' -type f -not -path '*/target/*' -print0 2>/dev/null \
    | xargs -0 grep -l 'rustfmt::skip' 2>/dev/null | head -20 || true)"
if [ -n "$SKIP_ATTRS" ]; then
    echo "$PREFIX ✗ 源码出现 #[rustfmt::skip] 声明，逐文件校验与模块树口径冲突 → 需人工核对后显式调整本脚本，不判绿："
    printf '%s\n' "$SKIP_ATTRS"
    exit 3
fi

# ① 自枚举待检文件集（只信文件系统，不信 git 状态）
WORK_LIST="$(mktemp)"
NULL_LIST="$(mktemp)"
RELLIST0="$(mktemp)"
TMP=""
cleanup() {
    if [ "$KEEP_TMP" = "1" ] && [ -n "$TMP" ]; then
        echo "$PREFIX --keep-tmp：镜像目录保留于 $TMP"
    else
        [ -n "$TMP" ] && rm -rf "$TMP"
    fi
    rm -f "$WORK_LIST" "$NULL_LIST" "$RELLIST0"
}
trap cleanup EXIT

find "$BACKEND_DIR" -name '*.rs' -type f -not -path '*/target/*' -print0 2>/dev/null > "$NULL_LIST"
RS_COUNT="$(tr -dc '\0' < "$NULL_LIST" | wc -c | tr -d ' ')"
if [ "${RS_COUNT:-0}" -eq 0 ]; then
    echo "$PREFIX ✗ 枚举到 0 个 .rs 文件（backend=$BACKEND_DIR）→ 枚举本身失败，防空操作直接判红。" >&2
    exit 5
fi

# 工作树脏与否只如实报告，不作为校验依据（校验对象永远是文件系统当前内容）
if command -v git >/dev/null 2>&1 && [ -d "$REPO_ROOT/.git" ]; then
    DIRTY_RS="$(git -C "$REPO_ROOT" status --porcelain -- 'backend' 2>/dev/null | grep -c '\.rs$' || true)"
    if [ "${DIRTY_RS:-0}" -gt 0 ]; then
        echo "$PREFIX ⚠ 工作树不干净：$DIRTY_RS 个 backend .rs 文件存在未提交改动，本次按工作树当前内容校验并如实报告，文件清单："
        git -C "$REPO_ROOT" status --porcelain -- 'backend' 2>/dev/null | grep '\.rs$' | sed "s#^#$PREFIX   #"
    fi
fi

# 构建去 CR 临时镜像：保留 backend 内相对目录结构，rustfmt.toml 复制到镜像根，
# 保证 rustfmt 从输入文件向上发现配置；所有 rustfmt 调用 cd 到镜像根、只喂相对路径。
TMP="$(mktemp -d)"
TMP_BASE="$(basename "$TMP")"
find "$BACKEND_DIR" -type d -not -path '*/target/*' -not -name 'target' -print0 > "$WORK_LIST"
while IFS= read -r -d '' d; do
    rel="${d#"$BACKEND_DIR"/}"
    [ "$rel" != "$d" ] && mkdir -p "$TMP/$rel"
done < "$WORK_LIST"
cp "$RUSTFMT_CFG" "$TMP/rustfmt.toml"

while IFS= read -r -d '' abs; do
    rel="${abs#"$BACKEND_DIR"/}"
    sed 's/\r$//' "$abs" > "$TMP/$rel"
    printf '%s\0' "$rel" >> "$RELLIST0"
done < "$NULL_LIST"

MIRROR_COUNT="$(tr -dc '\0' < "$RELLIST0" | wc -c | tr -d ' ')"
if [ "${MIRROR_COUNT:-0}" -ne "${RS_COUNT:-0}" ]; then
    echo "$PREFIX ✗ 镜像去 CR 后文件数 $MIRROR_COUNT ≠ 枚举数 $RS_COUNT → 采集不完整，不判绿。" >&2
    exit 5
fi

# ③ 探针自检：已知未格式化的样例必须被**同一调用形态**报出 "Diff in"，
#    报不出说明本步是空操作（绝对路径/配置未生效等失败形态），"0 diff"不可信，判红。
PROBE_REL="__fmt_gate_probe.rs"
printf 'fn main(){let   v:Vec<u8>=vec![1,2,3];\nlet s=v.iter().map(|a|*a+1).collect::<Vec<u8>>();\nprintln!("{}",s.len());}\n' > "$TMP/$PROBE_REL"
PROBE_OUT="$( cd "$TMP" && rustfmt --edition "$EDITION" --color never --check "$PROBE_REL" 2>&1 )"
PROBE_RC=$?
if [ "$PROBE_RC" -gt 1 ] || ! printf '%s' "$PROBE_OUT" | grep -qE '^Diff in'; then
    echo "$PREFIX ✗ 探针自检失败：已知未格式化样例未被报出 diff（rc=$PROBE_RC）→ 本步已退化为空操作，结论不可信，判红。探针原始输出："
    printf '%s\n' "$PROBE_OUT" | head -10
    rm -f "$TMP/$PROBE_REL"
    exit 4
fi
rm -f "$TMP/$PROBE_REL"
echo "$PREFIX ✓ 探针自检通过（注入的未格式化样例被正确报为未格式化，本步具备检测力）"

# 主批校验：每批 60 个相对路径（规避 Windows 命令行长度上限 os error 206 形态的整体失败）
FMT_RAW="$( cd "$TMP" && < "$RELLIST0" xargs -0 -r -n 60 rustfmt --edition "$EDITION" --color never --check 2>&1 )"
FMT_RC=$?
# Windows 实测（stable rustfmt 1.8.0）输出带 CR 行尾、且默认注入 ANSI 色码（即使管道
# 非 tty）：两者都会让"顶格 -/+ Diff 正文"归类型判定失真（实测 244 行误判未知）。
# 统一去 CR、剥 ANSI 转义序列后再归类（剥的都是呈现层噪声，不动 diff 语义内容）。
FMT_RAW="$(printf '%s' "$FMT_RAW" | tr -d '\r' | sed -E 's/\x1b\[[0-9;]*[A-Za-z]//g')"

FMTDIFFS="$(printf '%s\n' "$FMT_RAW" | grep -cE '^Diff in' || true)"
# 归类：Diff 块（"Diff in ..." 起、到空行/非缩进行为止）与行尾噪声之外的任何输出行
# 都是未知形态（rustfmt 崩溃、文件打不开等），fail-closed。去 CR 之后理论上不应再有
# "Incorrect newline style"，保留兼容仅为极端场景，且不豁免任何 diff。
# 归类（锚定式，兼容新旧两种 rustfmt diff 形态）："Diff in" 头行、以 '-'/'+' 起始的
# 增删行、以空白起始的上下文/续行、空行都属于 diff 正文；其余任何顶格非空白行都是
# 未知形态（rustfmt 崩溃、error/warning、文件打不开），fail-closed。
FMT_UNKNOWN="$(printf '%s\n' "$FMT_RAW" | awk '
  /^Diff in / { next }
  /^[-+]/     { next }
  /^[ \t]/    { next }
  /^$/        { next }
  /[^ \t]/    { n++ }
  END { print n + 0 }
')"
echo "$PREFIX 已实际校验 $RS_COUNT 个文件（rustfmt --edition $EDITION --check，rc=$FMT_RC），真实 diff=$FMTDIFFS 处，未知输出行=$FMT_UNKNOWN"

if [ "${FMT_UNKNOWN:-0}" -gt 0 ]; then
    echo "$PREFIX ✗ rustfmt 输出存在无法归类的行 → 结论不可信，判红。原始输出前 30 行："
    printf '%s\n' "$FMT_RAW" | head -30
    exit 6
fi
if [ "${FMTDIFFS:-0}" -gt 0 ]; then
    echo "$PREFIX ✗ 存在真实格式 diff，未格式化文件（backend/ 相对路径）："
    # rustfmt 报的是输入文件的已解析绝对路径（Windows 下为 \\?\...\Temp\<tmpdir>\... 且
    # 行号形态有 " at line N:" 与 ":N:" 两种），统一剥掉镜像临时目录前缀与行号尾缀，
    # 还原成 backend/ 相对路径点名；剥不动时原样输出（宁可难看，不可点名失真）。
    printf '%s\n' "$FMT_RAW" | grep -E '^Diff in' \
        | sed -E -e 's#^Diff in ##' -e 's# at line [0-9]+:?$##' -e 's#:[0-9]+:?$##' \
        | tr '\\' '/' \
        | sed -E "s#^.*/${TMP_BASE}/#backend/#" \
        | sort -u | sed "s#^#$PREFIX   #"
    echo "$PREFIX 修复方式（与 CI cargo fmt 口径一致）：对上述文件执行 rustfmt --edition $EDITION，或整仓 cd backend && cargo fmt --all 后复跑本脚本自证。"
    exit 1
fi
if [ "$FMT_RC" -ne 0 ]; then
    echo "$PREFIX ✗ rustfmt 退出码非 0（rc=$FMT_RC）但既无 diff 也无未知输出行 → 无法归因，保守判红。" >&2
    exit 6
fi

echo "$PREFIX ✓ rustfmt --check 无真实 diff（已实际校验 $RS_COUNT 个文件，且经探针自证具备检测能力）"
exit 0
