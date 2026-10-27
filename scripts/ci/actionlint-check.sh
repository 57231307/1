#!/usr/bin/env bash
# ==============================================================================
# scripts/ci/actionlint-check.sh
#
# 用途：CI 静态门。用固定版本 actionlint 校验 .github 下全部 workflow 与
#       composite action，把发现按严重度分流后套棘轮基线判定，防"再发"而不是
#       要求一次性清零历史 style/info 债。
#
# 判定语义（写死在此，回报同口径）：
#   1. actionlint 原生发现（非 shellcheck 的规则，如注入面 / 元数据缺项 / 表达式语法）
#      恒为零，任何一条即判红，不进基线。
#   2. shellcheck 发现的 error 级恒为零，任何一条即判红，不进基线。
#      （SC2170 属 error 级，必须真修，不允许基线化、不允许 disable 注释掩盖。）
#   3. shellcheck 发现的 warning / style / info 级走棘轮基线，只准减不准增：
#        - 当前发现里出现基线未登记的键  => 判红（新增须显式登记并复审）。
#        - 基线里的键在当前发现里不存在  => 判红（防过期锁：历史债被清掉后要
#          及时刷新基线，否则过期条目会掩盖将来在同位置的再发）。
#   4. 键 = <workflow 文件>|<shellcheck 在该 run 脚本体内的相对行:列>|<SC 码>。
#      用脚本体内相对行列而非 workflow 外层行号，是因为步骤在 workflow 文件里
#      上下移动会平移外层行号、但不改脚本体坐标，这样基线不会因排版移动而误过期。
#      重复键（同一文件内多个步骤命中同一 SC 码于同一相对行列）按重数并存，
#      comm 在非 -u 排序下天然做多集合差，故不丢"数量只减不增"的信息。
#
# 返回：
#   0 = 全部通过（error 恒零且非 error 与基线逐条精确对齐）
#   1 = 判红（error 级 / 原生级 / 新增未登记 / 过期锁 / 工具链缺失 / 解析地板被触发）
#
# 用法：
#   bash scripts/ci/actionlint-check.sh                # 默认 run 模式（CI）
#   ACTIONLINT_CHECK_MODE=self-test bash scripts/ci/actionlint-check.sh   # 自证检测力
#   ACTIONLINT_CHECK_MODE=compare ACTIONLINT_OUTPUT_FILE=<f> bash ...     # 读入既有发现文本比对
#
# 前置（run 模式）：runner 有 curl/sha256sum/tar；actionlint 不存在时按固定版本下载并做
#   sha256 完整性核对，下载或校验不过当场判红，绝不退化成"取不到就跳过检查"。
# ==============================================================================
set -uo pipefail

cd "$(dirname "$0")/../.."

MODE="${ACTIONLINT_CHECK_MODE:-run}"
ACTIONLINT_VERSION="${ACTIONLINT_VERSION:-1.7.12}"
ACTIONLINT_BIN="${ACTIONLINT_BIN:-./actionlint}"
BASELINE_FILE="${ACTIONLINT_BASELINE:-.github/actionlint-shellcheck-baseline.txt}"
FINDINGS_FILE="${ACTIONLINT_OUTPUT_FILE:-}"

# 统一排序语言环境，保证 runner（en_US）与本机（可能中文 locale）对 comm 判据一致。
export LC_ALL=C

# ------------------------------------------------------------------ 比较核函数
# 输入：$1=发现文本文件  $2=基线文件
# 输出：分流判定，返回码即本步期望的 CI 判定（0 绿 / 1 红）。
# 解析口径与自测共用同一段逻辑，确保自测真跑到生产判据而不是另写一套。
compare_findings() {
    local findings="$1"
    local baseline="$2"
    local cur_keys err_txt native_txt
    cur_keys="$(mktemp)"; err_txt="$(mktemp)"; native_txt="$(mktemp)"

    awk -v cur="$cur_keys" -v err="$err_txt" -v nat="$native_txt" '
    {
        line = $0
        # 只处理 "<path>.yml:<L>:<C>: ..." 形态的 actionlint 问题行
        if (line !~ /\.ya?ml:[0-9]+:[0-9]+: /) next
        if (line ~ / shellcheck reported issue in this script: /) {
            fidx = index(line, ":")
            file = substr(line, 1, fidx - 1)
            if (match(line, /SC[0-9]+:[a-z]+:[0-9]+:[0-9]+/)) {
                tok = substr(line, RSTART, RLENGTH)
                n = split(tok, a, ":")
                code = a[1]; sev = a[2]; il = a[3]; ic = a[4]
                key = file "|" il ":" ic "|" code
                if (sev == "error") { print key >> err; close(err) }
                else { print key >> cur; close(cur) }
            } else {
                # shellcheck 行但取不到 SC 码：无法判定严重度，按硬错处理，绝不放过
                print line >> nat; close(nat)
            }
        } else {
            # 非 shellcheck 的 actionlint 原生发现（[expression]/[syntax]/[parser] 等），恒为零
            print line >> nat; close(nat)
        }
    }' "$findings"

    LC_ALL=C sort "$cur_keys" > "${cur_keys}.sorted"
    # 基线：去掉注释行与空行后排序（写入 mktemp，不在仓库根留游离文件）
    BASE_SORTED="$(mktemp)"
    grep -vE '^[[:space:]]*(#|$)' "$baseline" | LC_ALL=C sort > "$BASE_SORTED"

    local err_count native_count new_count stale_count
    err_count="$(grep -c . "$err_txt" || true)"
    native_count="$(grep -c . "$native_txt" || true)"
    new_count="$(comm -23 "${cur_keys}.sorted" "$BASE_SORTED" | grep -c . || true)"
    stale_count="$(comm -13 "${cur_keys}.sorted" "$BASE_SORTED" | grep -c . || true)"

    local total_problem_lines parsed_lines
    total_problem_lines="$(grep -cE '\.ya?ml:[0-9]+:[0-9]+: ' "$findings" || true)"
    parsed_lines="$(( err_count + native_count + $(grep -c . "${cur_keys}.sorted" || true) ))"

    echo "actionlint 问题行总数: $total_problem_lines"
    echo "解析归类: 原生级=$native_count shellcheck-error=$err_count shellcheck非error(当前)=$(grep -c . "${cur_keys}.sorted" || true)"

    # 解析地板：分类计数必须等于问题行总数，否则说明解析器漏判（防"空转假绿"）。
    if [ "$parsed_lines" -ne "$total_problem_lines" ]; then
        echo "::error::actionlint-check 解析地板被触发：问题行 $total_problem_lines 但归类合计 $parsed_lines，判定器可能退化，拒绝判绿"
        rm -f "$cur_keys" "${cur_keys}.sorted" "$err_txt" "$native_txt" "$BASE_SORTED"
        return 1
    fi

    if [ "$native_count" -gt 0 ]; then
        echo "::error::发现 $native_count 条 actionlint 原生发现（非 shellcheck），此项恒零，请修复"
        echo "--- 原生发现 ---"; cat "$native_txt"
    fi
    if [ "$err_count" -gt 0 ]; then
        echo "::error::发现 $err_count 条 shellcheck error 级发现，此项恒零，必须真修不允许基线化/disable"
        echo "--- error 级键 ---"; cat "$err_txt"
    fi
    if [ "$new_count" -gt 0 ]; then
        echo "::error::发现 $new_count 条基线未登记的 shellcheck 非 error 键（新增未登记，只准减不准增）"
        echo "--- 新增键 ---"; comm -23 "${cur_keys}.sorted" "$BASE_SORTED"
    fi
    if [ "$stale_count" -gt 0 ]; then
        echo "::error::基线有 $stale_count 个键在当前源码已不存在（过期锁，请刷新基线以恢复对再发的拦截）"
        echo "--- 过期键 ---"; comm -13 "${cur_keys}.sorted" "$BASE_SORTED"
    fi

    rm -f "$cur_keys" "${cur_keys}.sorted" "$err_txt" "$native_txt" "$BASE_SORTED"

    if [ "$native_count" -gt 0 ] || [ "$err_count" -gt 0 ] || [ "$new_count" -gt 0 ] || [ "$stale_count" -gt 0 ]; then
        return 1
    fi
    return 0
}

# ------------------------------------------------------------------ 自证检测力
self_test() {
    local fails=0
    local tmpdir; tmpdir="$(mktemp -d)"
    # 夹具基线：两条已登记的非 error 键
    printf '%s\n' \
        '.github/workflows/ci-cd.yml|10:5|SC2086' \
        '.github/workflows/ci-cd.yml|20:1|SC2129' \
        > "$tmpdir/base.txt"

    # 期望绿：当前发现与基线逐条精确对齐，无 error 无原生
    cat > "$tmpdir/green.txt" <<'EOF'
.github/workflows/ci-cd.yml:100:9: shellcheck reported issue in this script: SC2086:info:10:5: Double quote to prevent globbing and word splitting [shellcheck]
.github/workflows/ci-cd.yml:200:9: shellcheck reported issue in this script: SC2129:style:20:1: Consider using { cmd1; cmd2; } >> file instead of individual redirects [shellcheck]
EOF
    # 期望红：新增一条基线没有的 style 发现
    cat > "$tmpdir/newstyle.txt" <<'EOF'
.github/workflows/ci-cd.yml:100:9: shellcheck reported issue in this script: SC2086:info:10:5: Double quote to prevent globbing and word splitting [shellcheck]
.github/workflows/ci-cd.yml:200:9: shellcheck reported issue in this script: SC2129:style:20:1: Consider using { cmd1; cmd2; } >> file instead of individual redirects [shellcheck]
.github/workflows/ci-cd.yml:300:9: shellcheck reported issue in this script: SC2002:style:5:3: Useless cat. [shellcheck]
EOF
    # 期望红：error 级发现（必须恒零，即使基线里有同名键也判红，因为它不进基线）
    cat > "$tmpdir/error.txt" <<'EOF'
.github/workflows/dead-code-audit.yml:154:9: shellcheck reported issue in this script: SC2170:error:9:6: Invalid number for -eq [shellcheck]
.github/workflows/ci-cd.yml:100:9: shellcheck reported issue in this script: SC2086:info:10:5: Double quote to prevent globbing and word splitting [shellcheck]
.github/workflows/ci-cd.yml:200:9: shellcheck reported issue in this script: SC2129:style:20:1: Consider using { cmd1; cmd2; } >> file instead of individual redirects [shellcheck]
EOF
    # 期望红：过期锁（基线里一条键在当前已消失）
    cat > "$tmpdir/stale.txt" <<'EOF'
.github/workflows/ci-cd.yml:100:9: shellcheck reported issue in this script: SC2086:info:10:5: Double quote to prevent globbing and word splitting [shellcheck]
EOF
    # 期望红：原生级发现（恒零）
    cat > "$tmpdir/native.txt" <<'EOF'
.github/workflows/ci-cd.yml:12:5: ${ { github.head_ref } } is not allowed in run scripts because it can be a script injection [shellcheck]
.github/workflows/ci-cd.yml:100:9: shellcheck reported issue in this script: SC2086:info:10:5: Double quote to prevent globbing and word splitting [shellcheck]
.github/workflows/ci-cd.yml:200:9: shellcheck reported issue in this script: SC2129:style:20:1: Consider using { cmd1; cmd2; } >> file instead of individual redirects [shellcheck]
EOF
    # 期望红：多重数上升（同一键当前出现 2 次，基线只 1 次）
    printf '%s\n' '.github/workflows/ci-cd.yml|10:5|SC2086' > "$tmpdir/base_dup.txt"
    cat > "$tmpdir/dup.txt" <<'EOF'
.github/workflows/ci-cd.yml:100:9: shellcheck reported issue in this script: SC2086:info:10:5: Double quote [shellcheck]
.github/workflows/ci-cd.yml:150:9: shellcheck reported issue in this script: SC2086:info:10:5: Double quote [shellcheck]
EOF

    check_expect() {
        local name="$1" want="$2" f="$3" base="$4"
        local got=0
        compare_findings "$f" "$base" >/dev/null 2>&1 || got=1
        if [ "$got" -eq "$want" ]; then
            echo "STEP self-test:$name EXIT=0 (as expected, exit=$want)"
        else
            echo "STEP self-test:$name EXIT=1 (expected exit=$want but got $got)"
            fails=$((fails + 1))
        fi
    }

    echo "=== actionlint-check self-test 检测力地板 ==="
    check_expect green_exact_match    0 "$tmpdir/green.txt"    "$tmpdir/base.txt"
    check_expect new_style_detected   1 "$tmpdir/newstyle.txt" "$tmpdir/base.txt"
    check_expect error_hard_zero      1 "$tmpdir/error.txt"    "$tmpdir/base.txt"
    check_expect stale_lock_detected  1 "$tmpdir/stale.txt"    "$tmpdir/base.txt"
    check_expect native_hard_zero     1 "$tmpdir/native.txt"   "$tmpdir/base.txt"
    check_expect dup_multiset_up      1 "$tmpdir/dup.txt"      "$tmpdir/base_dup.txt"

    # 解析地板自证：一行 shellcheck 但 SC 码残缺 => 问题行=1 归类>=1 且判红（不空转）
    cat > "$tmpdir/malformed.txt" <<'EOF'
.github/workflows/ci-cd.yml:100:9: shellcheck reported issue in this script: NOCODE:weird [shellcheck]
EOF
    check_expect parser_floor         1 "$tmpdir/malformed.txt" "$tmpdir/base.txt"

    rm -rf "$tmpdir"
    if [ "$fails" -gt 0 ]; then
        echo "SUMMARY self-test failing=$fails"
        return 1
    fi
    echo "SUMMARY self-test failing=0"
    echo "actionlint-check 判定器自证通过：error恒零/原生恒零/新增检测/过期锁检测/多重集上升/解析地板 全部有检测力"
    return 0
}

# ------------------------------------------------------------------ 取发现文本（run 模式）
ensure_actionlint() {
    if [ -x "$ACTIONLINT_BIN" ]; then
        return 0
    fi
    local VER="$ACTIONLINT_VERSION"
    local BASE="https://github.com/rhysd/actionlint/releases/download/v${VER}"
    local TGZ="actionlint_${VER}_linux_amd64.tar.gz"
    curl -sSfL --retry 3 -o "$TGZ" "$BASE/$TGZ" \
        || { echo "::error::actionlint 下载失败（固定版本 $VER），拒绝跳过检查"; return 1; }
    curl -sSfL --retry 3 -o actionlint-checksums.txt "$BASE/actionlint_${VER}_checksums.txt" \
        || { echo "::error::actionlint 校验清单下载失败，拒绝跳过检查"; return 1; }
    sha256sum --ignore-missing -c actionlint-checksums.txt \
        || { echo "::error::actionlint 产物 sha256 完整性校验未通过"; return 1; }
    tar -xzf "$TGZ"
    chmod +x ./actionlint
    return 0
}

case "$MODE" in
    self-test)
        self_test
        exit $?
        ;;
    compare)
        if [ -z "$FINDINGS_FILE" ] || [ ! -f "$FINDINGS_FILE" ]; then
            echo "::error::compare 模式需要 ACTIONLINT_OUTPUT_FILE 指向已存在的发现文本"
            exit 1
        fi
        ;;
    run)
        if [ ! -f "$BASELINE_FILE" ]; then
            echo "::error::缺少基线文件 $BASELINE_FILE"
            exit 1
        fi
        if ! ensure_actionlint; then
            exit 1
        fi
        FINDINGS_FILE="$(mktemp)"
        "$ACTIONLINT_BIN" > "$FINDINGS_FILE" 2>&1
        rc=$?
        # actionlint 自身退出码：0 无发现、1 有发现、>1 内部错误。>1 直接判红，绝不当"零发现"放过。
        if [ "$rc" -gt 1 ]; then
            echo "::error::actionlint 运行异常（退出码 $rc），拒绝判绿"
            cat "$FINDINGS_FILE"
            rm -f "$FINDINGS_FILE"
            exit 1
        fi
        echo "actionlint（$ACTIONLINT_VERSION）已运行，发现文件行数：$(grep -c . "$FINDINGS_FILE" || true)"
        ;;
    *)
        echo "::error::未知 ACTIONLINT_CHECK_MODE=$MODE（应为 run|compare|self-test）"
        exit 1
        ;;
esac

if [ ! -f "$BASELINE_FILE" ]; then
    echo "::error::缺少基线文件 $BASELINE_FILE"
    exit 1
fi

if compare_findings "$FINDINGS_FILE" "$BASELINE_FILE"; then
    echo "actionlint 门通过：error 恒零、原生恒零、非 error 与基线逐条精确对齐"
    echo "GATE_GREEN"
    exit 0
else
    echo "actionlint 门判红（详见上方分类清单）"
    exit 1
fi
