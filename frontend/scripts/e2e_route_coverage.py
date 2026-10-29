#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""路由 → e2e 用例 覆盖率判定器原型（R6 配套，只本地用，不进 CI 硬门）。

只读工具：不写仓库、不改 git、不跑浏览器。纯标准库，无第三方依赖。

用法：
    python e2e_route_coverage.py <仓库绝对路径> [--list-limit 20] [--sites]

分母（口径来源 repo1/frontend/scripts/route-snapshot.txt）：
    快照由 frontend/scripts/route-snapshot.mjs 生成，每行 `METHOD\\tPATH\\tHANDLER\\tROUTES_FILE`，
    头部 `# layers=N endpoints=M`。快照里的路径参数已被
    frontend/scripts/check-api-paths.mjs:37 normalizePath() 归一为 `*`
    （axum 源码里的 `{id}` / `${id}` / `:id` 全部变 `*`），不是 `{param}` 形态。
    判定单位取 (METHOD, PATH) 对 —— 与快照自身的 key 一致：同一路径的 GET 与 DELETE
    是两个契约，只按路径去重会把「DELETE 从没测过」算成已覆盖。
    另外并报「按路径去重」的第二视图，便于与历史数字对照。

分子（frontend/e2e 下的字面请求路径）：
    只认「真正把请求发出去」的调用位点，不认 UI 路由串（page.goto / router.push），
    也不认 mock 拦截（context.route / page.route / route.fulfill）：
      R1 apiCall / apiCallRaw / apiCallExpectFail / apiCall*：arg1=METHOD 字面量，arg2=路径；
      R2 page.request.get|post|put|patch|delete|fetch(url, {method:'X'})；
      R3 request.get|post|... （APIRequestContext，同 R2）；
      R4 helper 间接：自动识别「把某个入参喂给 R1/R2/R3 路径位」的顶层函数
         （safeGet/safePostAction/createEntity/verifyEndpointHealthy/…），
         在其调用位按参数位置取路径字面量 —— 上一轮 depth 体检证实这条不补就会把
         「其实覆盖了」误判成零覆盖，故为强制项；
      R5 显式注册表：e2e/flow/51-endpoint-health-scan.spec.ts 的 `const ROUTES: string[]`
         与 e2e/traversal/modules.config.ts 的 `listApi: '...'`（与 check-api-paths.mjs:405
         E2E_PROBE_REGISTRIES 同一清单，不另起口径），按 GET 计入。
      R6a 对象矩阵（`{ path: '...', method: 'PUT' }` 形态）：method 只认**本条目自己的顶层
         成员**，条目边界由花括号配平取最内层，多行条目与带 `{id}` 的路径串都不误判；
         窗口若靠上一条目的 `{ path:` 字面量定位，会让无 method 的条目继承邻居方法（造幽灵方法），
         多行条目还会把更大范围并进本条目。条目自己不写 method 时，缺省方法按**数组名**查派生表
         （审批矩阵缺省 POST、敏感导出数组缺省 GET，两处各有消费点实证），
         绝不按文件名一律套 POST；数组名以 `_ENDPOINTS` 结尾却不在表里时进
         registry_no_method 响亮计数，既不编方法也不丢位点。
    路径表达式解析：字符串/模板字面量、同文件 const 字符串常量代入、
    API_BASE/API_PREFIX 常量代入、`a + b` 拼接、`${expr}` → `*`、纯数字段 → `*`、去 query。

三档结论：
    A 零覆盖：分母条目在 e2e 任何请求位点都没出现过（含方法不符的也算路径未命中）。
    B 浅覆盖：出现过，但只证「可达/状态面」，拿不出字段断言或回读。判据（位点级，逐条可点验）：
       b1 由 verifyEndpointHealthy / verifyDownloadEndpointHealthy 发出（helpers.ts:3092/3165，
          返回值 `void` 且体内无 `expect(`，注释原文「无返回值也视为健康」）；
       b2 由 R5 注册表发出（51 健康扫描把非崩溃码只进 warns 不参与断言；
          list-api-probe.ts 头注释明写复用 verifyEndpointHealthy 的纯状态判据）；
       b3 结果未被绑定且同域作用域内没有针对它的断言/抛错；
       b4 写方法（POST/PUT/PATCH/DELETE）在同域作用域内没有后续 GET 回读。
    C 深覆盖：至少一个位点拿得出证据 —— 返回值被绑定且作用域内对该变量有 `expect(`/`throw` 断言；
       或写方法后同作用域出现同路径或父路径的 GET 回读；或 apiCallExpectFail 的结果被断言；
       或经由体内含 `expect(` 的 helper 发出。
    B/C 的取值偏保守：拿不出证据就记 B。B 是「待抽查」清单，不是判死结论，
    每条 B 都给了 file:line，可逐条点验。

不静默原则：解析不出路径的位点、命中不了快照的 e2e 字面量（幽灵/漂移）、mock 位点、
端点矩阵里缺方法的条目（registry_no_method），全部计数并列出，不当成「没有」。
分母残缺同样是「没有」的反面，一律响亮报错不报零：快照文件找不到、e2e 目录找不到、
快照解析出 0 条端点行、头行声明的 endpoints 与实际行数不符（截断）。这四种情况下
任何「A 零覆盖=0」都是输入缺失伪装的达标，不是用例覆盖出来的零缺口。

用法附带检测力自证：
    python e2e_route_coverage.py --self-test <可写工作目录>
夹具仓库铺在该目录下（绝对路径不写进源码），打印夹具判据、变异自证与假零防御三段。
"""
from __future__ import annotations

import io
import os
import re
import sys
from collections import defaultdict

API_BASE_PREFIX = "/api/v1/erp"
SENTINEL = "\u0001SEG\u0001"
# e2e 里代表「主机/前缀」的常量名：值一律折叠，前缀按 API_BASE_PREFIX 补
HOST_CONSTS = {
    "API_BASE",
    "API_BASE_URL",
    "BASE_URL",
    "E2E_API_BASE",
    "ORIGIN",
    "API_ORIGIN",
    "SERVER_URL",
    "BASE",
}
# 这些名字的值就是 /api/v1/erp 前缀（e2e 里多处 `const API_PREFIX = '/api/v1/erp'`）
PREFIX_CONSTS = {"API_PREFIX", "ERP_PREFIX", "PREFIX"}

REQUEST_METHODS = ("GET", "POST", "PUT", "PATCH", "DELETE")
WRITE_METHODS = ("POST", "PUT", "PATCH", "DELETE")

API_CALL_RE = re.compile(
    r"\b(apiCall[A-Za-z0-9_]*)(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\s*\(", re.A)
# 位点识别用：只锚定"名字后面接泛型或左括号"，泛型交 _call_paren 程序化配平
API_NAME_RE = re.compile(r"\b(apiCall[A-Za-z0-9_]*)\s*(?=<|\()", re.A)
REQ_NAME_RE = re.compile(
    r"\b(page|context|request)\.(get|post|put|patch|delete|fetch)\s*(?=<|\()", re.A | re.I)
PAGE_REQ_RE = re.compile(
    r"\b(?:page|context|request)\.(get|post|put|patch|delete|fetch)"
    r"(?:<[^<>]*(?:<[^<>]*>[^<>]*)*>)?\s*\(", re.A | re.I)
# 缩进捕获只能吃水平空白：`^(\\s*)` 会把声明之前的整行空行/换行一起吃进来，
# 于是 indent != "" 让所有顶层函数都被登记成局部函数，跨文件解析不到 helper，
# 间接位点在调用文件整片丢失（spec 调 helpers 导出函数即此形态）。
FUNC_DECL_RE = re.compile(
    r"^([ \t]*)(?:export\s+)?(?:async\s+)?function\s+([A-Za-z_]\w*)\s*(?:<[^()\n]*>)?\s*\(", re.A | re.M)
TEST_DECL_RE = re.compile(r"^\s*(?:test|it)(?:\.\w+)*\s*\(\s*(?:'|\"|`)", re.A | re.M)
DESCRIBE_RE = re.compile(r"^\s*test\.describe", re.A)
EXPECT_RE = re.compile(r"\b(?:expect|assert)[A-Za-z0-9_.]*\s*\(", re.A)
ROUTES_DECL_RE = re.compile(r"const\s+ROUTES\s*:\s*string\[\]\s*=\s*\[", re.A)
LISTAPI_RE = re.compile(r"\blistApi\s*:\s*(['\"`])([^'\"`]*)\1", re.A)
MOCK_RE = re.compile(r"\b(?:context|page)\.route\s*\(|\broute\.fulfill\s*\(", re.A)
# 数据表字段驱动位点：对象成员里写成路径字面量的字段，DelCase.createApi 这类形态。
# 只收以斜杠开头的取值，label 与 listKey 这类非路径成员进不了表，不会污染展开。
MEMBER_PATH_RE = re.compile(r"(?<![\w.$])([A-Za-z_]\w*)\s*:\s*(['\"`])(/[^'\"`\n]*)\2", re.A)
# 实参本身就是内联对象字面量时，只认这些字段名当 API 路径，避免把 payload 里恰好以
# 斜杠开头的业务字段当成请求路径。
API_MEMBER_FIELDS = frozenset((
    "path", "api", "endpoint", "urlPath", "apiUrl",
    "createApi", "deleteApi", "getApi", "listApi", "updateApi", "readApi",
))
# 展开出的字面量以占位名喂给 to_canonical，前缀补齐与查询串剥离仍走原归一规则。
# 占位名必须是合法标识符，否则 to_canonical 走不到「按名字查常量表」那条分支。
MEMBER_PLACEHOLDER = "__MEMBER__"
# 对象矩阵数组名到缺省方法的派生表：只收消费点有实证的两张矩阵，别的一律不派生。
# 这不是豁免表（豁免表会把位点从判定里摘出去），而是给条目补它自己在运行时真正
# 使用的方法：条目显式写了 method 时本表不参与；表里没有的数组名既不编方法，
# 也不静默丢弃，而是进 registry_no_method 响亮计数交人工点名。
# APPROVE_ENDPOINTS 依据 traversal/41-approve-endpoints.spec.ts:46 `method ?? 'POST'`
#   与 endpoints.config.ts:146 头注释「method：审批 HTTP 方法，缺省 POST」。
# SENSITIVE_EXPORT_ENDPOINTS 的类型是 Array<{path;resource}>，压根没有 method 成员，
#   消费点 traversal/39-export-endpoints.spec.ts:36 用 page.request.get ⇒ 缺省 GET，
#   且这些路径在快照里以 GET 在册（crm/customers/export、products/export、
#   audit-logs/export 等），旧写法一律套 POST 才会造出假的方法不符。
REGISTRY_DEFAULT_METHOD = {
    "APPROVE_ENDPOINTS": "POST",
    "SENSITIVE_EXPORT_ENDPOINTS": "GET",
}
# 一个字段在同文件的路径字面量超过这个数就不展开，登记成未解析而不是硬吃进来。
MEMBER_FANOUT_LIMIT = 200
# `const/let NAME = 右值` 的声明头，右值原文交给 _loose_map 逐字符配平去取。
LOOSE_DECL_RE = re.compile(r"\b(?:const|let)\s+([A-Za-z_$][\w$]*)\s*=(?!=)\s*", re.A)

# 形态分桶：只为「哪些分母条目其实不该按 e2e 业务用例的尺子要求」提供可复演的划线依据。
# 不做剔除：A 档总数始终按全量分母给，剔除口径由读的人在报告里另行声明。
OUT_OF_SCOPE_BUCKETS = (
    ("健康/就绪/指标探针（无业务终态可断）", ("health", "liveness", "readiness", "metrics")),
    ("打印/导出/下载类（返回二进制或文件流）", ("/print", "/export", "/download")),
    ("模板与导入导出工件（文件体，非 JSON 信封）", ("/templates", "/data-import", "/imports")),
    ("上传/头像（multipart 文件上传）", ("/upload", "/avatar")),
    ("实时通道（websocket，非 request/response 语义）", ("/ws/",)),
    ("运维/容灾内部端点（非业务用户路径）", ("/admin/failover", "/slow-queries", "/api-gateway", "/system-update")),
    ("AI 推断类（输出不确定，只能断可达）", ("/ai/", "predict", "forecast", "anomal", "optimization")),
)


# 覆盖分母豁免表（调度 #148 终裁 A 片1 落档）：命中键从 A 档分母摘除，
# 但必须逐条响亮打印；规则当轮命中 0 条即报豁免过期，禁止静默失效。
# 精确全键匹配（归一口径 {id} 已在快照侧折成 *），不用通配防误摘邻腿
# （同前缀的 cancel 与 create 与 GET * 仍在 A 档，属 #139 正常残量）。
# 依据：二级大客户审批为有意特性——migration domain/business/m0083_add_customer_tier_column.rs
# 与 services/crm/customer_transfer_approval_service.rs 头注释自述（check_large_customer
# 恒判非大客户恒 max_level 恒 1，director/manager 腿正常 HTTP 生命周期不可达）；
# service 层契约锁已覆盖非终态挂链组合：contract_wave11_major_customer_second_level_reachable_test.rs
# 与 contract_wave14_d6_second_level_approval_test.rs（直插 DB 造数）。
COVERAGE_EXEMPT = {
    "POST /api/v1/erp/crm/transfer-approvals/*/director-approve":
        "正常生命周期 HTTP 不可达（max_level=2 分支不触发）；service 层契约锁覆盖：wave11 大客户二级可达 + wave14 d6 二级审批，直插 DB 造非终态挂链。",
    "POST /api/v1/erp/crm/transfer-approvals/*/manager-approve":
        "正常生命周期 HTTP 不可达（同上）；service 层契约锁覆盖：wave11 与 wave14 直插 DB 造非终态挂链组合。",
}


def read(path):
    return io.open(path, encoding="utf-8", errors="replace", newline=None).read()


# ---------------------------------------------------------------- 归一化
def normalize_path(p):
    """对齐 check-api-paths.mjs:37 normalizePath，另加两项 e2e 侧必需的归一。

    与 JS 版的差异（有意为之，均在报告里点名）：
      - JS 版 `:(\\/|[\\w])` 分支把 `:id` 展成 `*d`（残字符未被吞掉）。本仓快照与
        e2e 字面量都不含 `:param` 形态（快照 0 个冒号段；e2e 仅 1 处 `:id` 且落在
        注释里的 UI 路由），故不复制该缺陷，按语义整段归一为 `*`。
      - 追加「纯数字段 → `*`」：JS 版不需要（它扫的是 src/api 的模板串），
        而 e2e 里存在硬编码 id（`/purchase/orders/12/items`），不归一就会误判零覆盖。
    """
    s = p
    q = s.find("?")
    if q >= 0:
        s = s[:q]
    h = s.find("#")
    if h >= 0:
        s = s[:h]
    # 完整 URL（含协议）：从 /api/v1/erp 处切齐
    k = s.find(API_BASE_PREFIX)
    if k > 0:
        s = s[k:]
    s = re.sub(r"\$\{[^{}]*\}", "*", s)
    s = re.sub(r"\{[^{}]*\}", "*", s)
    s = re.sub(r"(?<=/):[A-Za-z_]\w*", "*", s)
    s = re.sub(r"(?<=/)\d+(?=/|$)", "*", s)
    s = re.sub(r"/{2,}", "/", s)
    if not s.startswith("/"):
        s = "/" + s
    s = re.sub(r"/+$", "", s)
    return s or "/"


def _inline_template_calls(text, consts):
    """把 `${NAME(args)}` 与裸 `NAME(args)` 换成 NAME 的模板取值（单行箭头常量）。

    `${defectBase(recordId!)}/process` 这类路径由同文件小函数拼出：不内联就整段
    变 `*`，得到 /*/process 的假幽灵；内联后骨架回来了，参数段留给归一化的
    `${...}`→`*` 规则处理。
    """
    call_args = r"(?:[^()]|\([^()]*\))*"
    for _ in range(2):
        changed = False
        for name, val in list(consts.items()):
            pat = r"\$\{\s*%s\s*\(\s*%s\s*\)\s*\}" % (re.escape(name), call_args)
            new = re.sub(pat, (val.replace("\\", "\\\\") or '""'), text)
            if new != text:
                text, changed = new, True
            pat2 = r"(?<![\w.$\"'`])%s\s*\(\s*%s\s*\)" % (re.escape(name), call_args)
            new = re.sub(pat2, (val.replace("\\", "\\\\") or '""'), text)
            if new != text:
                text, changed = new, True
        if not changed:
            break
    return text


def to_canonical(raw, consts):
    """把一个路径表达式求值成 /api/v1/erp 之后的归一路径。返回 (path|None, 未解析片段)."""
    if raw is None:
        return None, "empty"
    text = _inline_template_calls(raw.strip(), consts)
    if not text:
        return None, "empty"
    unresolved = []
    out = []
    i = 0
    n = len(text)
    while i < n:
        c = text[i]
        if c in "'\"`":
            quote = c
            i += 1
            buf = []
            while i < n:
                ch = text[i]
                if ch == "\\":
                    buf.append(text[i + 1] if i + 1 < n else "")
                    i += 2
                    continue
                if ch == quote:
                    i += 1
                    break
                if ch == "$" and i + 1 < n and text[i + 1] == "{":
                    j = i + 2
                    depth = 1
                    inner = []
                    while j < n and depth:
                        if text[j] == "{":
                            depth += 1
                        elif text[j] == "}":
                            depth -= 1
                            if depth == 0:
                                j += 1
                                break
                        inner.append(text[j])
                        j += 1
                    i = j
                    expr = "".join(inner).strip()
                    ident = re.match(r"^([A-Za-z_]\w*)$", expr)
                    sub = None
                    if ident:
                        name = ident.group(1)
                        if name in consts:
                            sub = consts[name]
                        elif name in HOST_CONSTS:
                            sub = ""
                        elif name in PREFIX_CONSTS:
                            sub = API_BASE_PREFIX
                    if sub is None:
                        buf.append("*")
                        if not ident:
                            unresolved.append(expr[:32])
                        else:
                            unresolved.append(ident.group(1))
                    else:
                        buf.append(sub)
                    continue
                buf.append(ch)
                i += 1
            out.append("".join(buf))
            continue
        if c == "+":
            i += 1
            continue
        m = re.match(r"[A-Za-z_]\w*", text[i:])
        if m:
            name = m.group(0)
            i += len(name)
            if name in consts:
                out.append(consts[name])
            elif name in HOST_CONSTS:
                out.append("")
            elif name in PREFIX_CONSTS:
                out.append(API_BASE_PREFIX)
            elif name in ("await", "async", "new", "String", "encodeURIComponent"):
                continue
            else:
                out.append("*")
                unresolved.append(name)
            continue
        if c in " \n\t":
            i += 1
            continue
        out.append(c)
        i += 1
    joined = "".join(out)
    if not joined or joined == "*":
        return None, ",".join(sorted(set(unresolved))) or raw[:40]
    if "/" not in joined.lstrip("*"):
        # 只剩变量占位，没有任何路径骨架：不可信，判未解析
        return None, raw[:40]
    path = normalize_path(joined)
    if path.startswith(API_BASE_PREFIX):
        pass
    elif re.match(r"^https?://", path):
        return None, raw[:40]
    else:
        path = API_BASE_PREFIX + (path if path.startswith("/") else "/" + path)
    return path, ",".join(sorted(set(unresolved)))


# ---------------------------------------------------------------- 词法辅助
def strip_comments(src):
    """逐字符去 // 与 /* */ 注释，长度与换行数保持不变（行号可对齐原文）。

    进入字符串的判据：引号前一个非空字符是值位置符，避免把正则/字符类里的引号
    当字符串起点（例如 toMatch(/['"]/)）。
    """
    out = []
    i = 0
    n = len(src)
    state = "code"
    quote = ""
    prev_significant = ""
    while i < n:
        c = src[i]
        nxt = src[i + 1] if i + 1 < n else ""
        if state == "code":
            if c == "/" and nxt == "/":
                state = "line"
                out.append("  ")
                i += 2
                continue
            if c == "/" and nxt == "*":
                state = "block"
                out.append("  ")
                i += 2
                continue
            if c in "'\"`" and (prev_significant in "(,=:[{+&|!?;\n\r\t" or prev_significant == ""):
                state = "str"
                quote = c
                out.append(c)
                prev_significant = c
                i += 1
                continue
            if not c.isspace():
                prev_significant = c
            out.append(c)
            i += 1
            continue
        if state == "line":
            if c == "\n":
                state = "code"
                out.append("\n")
            else:
                out.append(" ")
            i += 1
            continue
        if state == "block":
            if c == "*" and nxt == "/":
                state = "code"
                out.append("  ")
                i += 2
                continue
            out.append("\n" if c == "\n" else " ")
            i += 1
            continue
        # in string
        if c == "\\":
            out.append(c)
            if nxt:
                out.append(nxt)
            i += 2
            continue
        if c == quote:
            state = "code"
            out.append(c)
            prev_significant = c
            i += 1
            continue
        if c == "\n" and quote != "`":
            state = "code"  # 未闭合的单双引号：按行截断，回到代码态
            out.append("\n")
            i += 1
            continue
        if c == "$" and nxt == "{" and quote == "`":
            depth = 1
            out.append(c)
            out.append(nxt)
            i += 2
            while i < n and depth:
                if src[i] == "{":
                    depth += 1
                elif src[i] == "}":
                    depth -= 1
                out.append(src[i])
                i += 1
            continue
        out.append(c)
        i += 1
    return "".join(out)


def split_args(src, open_idx):
    """src[open_idx] == '('，取该调用的实参列表（顶层逗号切分）与右括号后一位。"""
    depth = 0
    i = open_idx
    buf = []
    args = []
    quote = ""
    end = len(src)
    while i < len(src):
        c = src[i]
        if quote:
            if c == "\\":
                buf.append(c)
                if i + 1 < len(src):
                    buf.append(src[i + 1])
                i += 2
                continue
            if c == quote:
                quote = ""
            buf.append(c)
            i += 1
            continue
        if c in "'\"`":
            quote = c
            buf.append(c)
            i += 1
            continue
        if c == "(":
            depth += 1
            if depth == 1:
                i += 1
                continue
        elif c == ")":
            depth -= 1
            if depth == 0:
                end = i + 1
                if "".join(buf).strip():
                    args.append("".join(buf).strip())
                return args, end
        elif c == "," and depth == 1:
            args.append("".join(buf).strip())
            buf = []
            i += 1
            continue
        buf.append(c)
        i += 1
    if buf and "".join(buf).strip():
        args.append("".join(buf).strip())
    return args, len(src)


def match_brace(src, open_idx):
    """src[open_idx] == '{'，返回配对右括号下标；失败返回 -1。忽略字符串与注释（已预处理）。"""
    depth = 0
    i = open_idx
    quote = ""
    while i < len(src):
        c = src[i]
        if quote:
            if c == "\\":
                i += 2
                continue
            if c == quote:
                quote = ""
            i += 1
            continue
        if c in "'\"`":
            quote = c
            i += 1
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def _fn_body_open(src, after):
    """签名右括号后一位 `after`，跳过返回类型里的泛型与花括号，找函数体的 `{`。

    `Promise<{ items: Dict[] }>` 这类返回类型自带花括号：直接 find("{") 会把类型
    注解当函数体，body/range 全错，进而 forwarder 判不出来、体内位点整片漏计。
    """
    i = after
    n = len(src)
    angle = 0
    while i < n:
        c = src[i]
        if c == "<":
            angle += 1
        elif c == ">":
            if angle > 0:
                angle -= 1
        elif c == "{" and angle == 0:
            return i
        elif c == ";" and angle == 0:
            return -1  # declare/abstract 签名，无函数体
        i += 1
    return -1


def line_of(src, idx):
    return src.count("\n", 0, idx) + 1


def brace_spans(src):
    """一遍扫描给出文本里每个花括号的配平区间（开, 闭），忽略字符串字面量。

    调用方 Analyzer._entry_span（对象矩阵条目定位）。注释已由 strip_comments 剥掉，
    字符串里的 `{id}` 靠 quote 态跳过，不会把路径串里的花括号当括号。
    """
    spans = []
    stack = []
    quote = ""
    i = 0
    n = len(src)
    while i < n:
        c = src[i]
        if quote:
            if c == "\\":
                i += 2
                continue
            if c == quote:
                quote = ""
            i += 1
            continue
        if c in "'\"`":
            quote = c
            i += 1
            continue
        if c == "{":
            stack.append(i)
        elif c == "}":
            if stack:
                spans.append((stack.pop(), i))
        i += 1
    return spans


def entry_members(src, op, cl):
    """取对象条目 {op..cl} 顶层（不含嵌套对象）的成员名到取值原文。

    调用方 R6a 对象矩阵判读，用来只认本条目自己的 method 成员。
    做法是按顶层逗号切段（字符串与嵌套括号内的逗号不算分隔），段首取键名。
    嵌套对象里的 method（例如 body 里恰好有名为 method 的业务字段）不计，
    否则子对象或相邻条目的方法会被冒充成本条目的方法。
    """
    cuts = []
    depth = 0
    quote = ""
    i = op + 1
    while i < cl:
        c = src[i]
        if quote:
            if c == "\\":
                i += 2
                continue
            if c == quote:
                quote = ""
            i += 1
            continue
        if c in "'\"`":
            quote = c
        elif c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
        elif depth == 0 and c == ",":
            cuts.append(i)
        i += 1
    cuts.append(cl)

    members = {}
    seg_start = op + 1
    for cut in cuts:
        seg = src[seg_start:cut]
        s = seg.strip()
        lead = seg_start + (len(seg) - len(seg.lstrip()))
        colon = None
        for k, ch in enumerate(s):
            if ch != ":":
                continue
            prev_colon = k > 0 and s[k - 1] == ":"
            next_colon = k + 1 < len(s) and s[k + 1] == ":"
            if not prev_colon and not next_colon:
                colon = k
                break
        if colon is not None:
            km = re.search(r"([A-Za-z_]\w*)\s*\??\s*$", s[:colon])
            if km:
                members[km.group(1)] = s[colon + 1:].strip()
        seg_start = cut + 1
    return members


def enclosing_const_name(src, pos):
    """找包住 pos 的 `const 名 …… = [` 声明里的数组名，找不到返回空串。

    只要求 名 与 `[` 之间不含等号，类型标注里的花括号与尖括号都不影响命中；
    调用方 R6a 用数组名去查该数组消费点的缺省方法。
    """
    best = ("", -1)
    for m in re.finditer(r"const\s+(\w+)[^=]*=\s*\[", src[:pos]):
        best = (m.group(1), m.end())
    return best[0] if best[1] > 0 else ""


def const_map(src):
    r"""同文件字符串字面量常量表，并把常量之间的内插传递展开。

    收录四类（缺任何一类都会造出假未解析或假幽灵）：
      1. `const/let/var NAME = '字面量'`：`let path = '/x?query'` 累加 query 的写法
         只收 const 就会漏（13-receipt-inventory-writeback）；
      2. 函数参数默认值 `params = '?page=1'`：签名里的默认值就是该位的真实取值；
      3. 单行箭头模板常量 `const defectBase = (id) => \`/x/${id}\``：调用位以
         `${defectBase(...)}` 出现，交给 to_canonical 的内联展开；
      4. 常量之间的内插传递（见下）——不传递展开会出假幽灵：`${API_BASE}` 残留在
         取值里，归一后变成 /*/*/x，对不上任何在册路由。
    """
    raw = {}
    for m in re.finditer(
            r"(?<![\w.$])([A-Za-z_]\w*)\s*=(?!=)\s*(['\"`])((?:\\.|(?!\2)[^\\\n])*)\2", src):
        raw[m.group(1)] = m.group(3)
    for m in re.finditer(
            r"(?<![\w.$])([A-Za-z_]\w*)\s*=\s*\(([^()]*)\)\s*=>\s*(['\"`])"
            r"((?:\\.|(?!\3)[^\\\n])*)\3", src):
        raw.setdefault(m.group(1), m.group(4))

    def subst(name):
        if name in raw:
            return raw[name]
        if name in HOST_CONSTS:
            return ""
        if name in PREFIX_CONSTS:
            return API_BASE_PREFIX
        return None

    for _ in range(3):
        changed = False
        for k in sorted(raw):
            v = raw[k]
            nv = re.sub(r"\$\{\s*([A-Za-z_]\w*)\s*\}",
                        lambda m: subst(m.group(1)) if subst(m.group(1)) is not None else m.group(0),
                        v)
            if nv != v:
                raw[k] = nv
                changed = True
        if not changed:
            break
    return raw


# ---------------------------------------------------------------- 快照（分母）
def load_snapshot(repo):
    cand = []
    for rel in (
        "frontend/scripts/route-snapshot.txt",
        "backend/route-snapshot.txt",
        "route-snapshot.txt",
    ):
        p = os.path.join(repo, *rel.split("/"))
        if os.path.isfile(p):
            cand.append(p)
    if not cand:
        raise SystemExit("找不到 route-snapshot.txt，请在仓库里指定：已试 " + ", ".join(
            r for r in ("frontend/scripts/route-snapshot.txt", "backend/route-snapshot.txt", "route-snapshot.txt")))
    path = cand[0]
    rows = []
    header = ""
    for line in read(path).split("\n"):
        if not line.strip():
            continue
        if line.startswith("#"):
            header = line.strip()
            continue
        parts = line.split("\t")
        if len(parts) < 2:
            continue
        method, pth = parts[0].strip(), parts[1].strip()
        handler = parts[2].strip() if len(parts) > 2 else ""
        mod = parts[3].strip() if len(parts) > 3 else ""
        rows.append({"method": method, "path": normalize_path(pth), "raw_path": pth,
                     "handler": handler, "routes_file": mod})
    # 假零防御，分母侧。调用方 load_snapshot 上游为 analyze 与主输出，报错文本直接进 stderr。
    # 空快照、或行数与头行声明不符时响亮报错，绝不返回零行让上层算出「A 零覆盖=0」。
    # 那种零是输入缺失造成的假绿，不是用例真覆盖出来的零缺口。
    if not rows:
        raise SystemExit("快照 %s 解析出 0 条端点行，判分母输入无效（每行应为 METHOD、PATH、HANDLER、ROUTES_FILE 四列制表符分隔），拒绝报零" % path)
    declared = re.search(r"endpoints=(\d+)", header)
    if declared and int(declared.group(1)) != len(rows):
        raise SystemExit("快照 %s 头行声明 endpoints=%s 与实际解析出的 %d 行不符，判分母输入不完整，拒绝报零" % (
            path, declared.group(1), len(rows)))
    return path, header, rows


# ---------------------------------------------------------------- e2e 站点（分子）
def e2e_files(repo):
    root = os.path.join(repo, "frontend", "e2e")
    if not os.path.isdir(root):
        raise SystemExit("找不到 e2e 目录: " + root)
    skip = {"node_modules", ".auth", "reporters", "dist", ".git"}
    out = []
    for dirp, dirs, files in os.walk(root):
        dirs[:] = sorted(d for d in dirs if d not in skip)
        for fn in sorted(files):
            if fn.endswith(".ts") and not fn.endswith(".d.ts"):
                out.append(os.path.join(dirp, fn))
    return sorted(out), root


def origin_of(rel):
    base = os.path.basename(rel)
    if base == "global-setup.ts":
        return "setup"
    if base.endswith(".spec.ts"):
        return "spec"
    if rel.startswith("e2e/fixtures/") or "/fixtures/" in rel:
        return "fixture"
    return "helper"


def build_function_index(src):
    """函数声明（顶层与函数内局部）：名字 -> 签名、体、字节区间、是否顶层。

    局部函数也要建索引：spec 里大量用例把请求封在本文件的 `async function erpGet(...)`
    里，只认顶层会把这类位点当成"路径来自未知变量"而漏计，进而虚报零覆盖。
    """
    fns = {}
    for m in FUNC_DECL_RE.finditer(src):
        indent, name = m.group(1), m.group(2)
        op = src.index("(", m.end() - 1)
        args, after = split_args(src, op)
        bi = _fn_body_open(src, after)
        if bi < 0:
            continue
        be = match_brace(src, bi)
        if be < 0:
            continue
        params = []
        for a in args:
            a = a.strip()
            if not a:
                continue
            params.append(re.split(r"[:?=(,]", a)[0].strip())
        fns.setdefault(name, {
            "line": line_of(src, m.start()),
            "params": params,
            "body": src[bi:be + 1],
            "ret": src[after:bi],
            "range": (bi, be),
            "top": indent == "",
        })
    return fns


def collect_scopes(src):
    """作用域区间表：test/it 块与函数块，用于「同域断言/回读」判定。"""
    scopes = []
    for m in re.finditer(r"\b(?:test|it)(?:\.\w+)*\s*\(", src):
        head = src[:m.start()].rstrip()
        if head.endswith("describe") or ".describe" in head[-40:]:
            continue
        op = src.index("(", m.start())
        _, after = split_args(src, op)
        bi = src.find("{", after - 1)
        if bi < 0:
            continue
        be = match_brace(src, bi)
        if be > bi:
            scopes.append((bi, be))
    for m in FUNC_DECL_RE.finditer(src):
        op = src.index("(", m.end() - 1)
        _, after = split_args(src, op)
        bi = _fn_body_open(src, after)
        if bi < 0:
            continue
        be = match_brace(src, bi)
        if be > bi:
            scopes.append((bi, be))
    scopes.sort(key=lambda s: (s[0], -(s[1] - s[0])))
    return scopes


def innermost_scope(scopes, idx, full):
    best = None
    for a, b in scopes:
        if a <= idx <= b:
            if best is None or (b - a) < (best[1] - best[0]):
                best = (a, b)
    return best or (0, full)


class Analyzer:
    """一次建仓：源码（去注释）、常量表、顶层函数表、helper 分类表；再采位点。"""

    def __init__(self, repo):
        self.repo = repo
        self.files, self.e2e_root = e2e_files(repo)
        self.srcs = {}
        self.consts = {}
        self.scopes = {}
        self.file_fns = {}
        self.fns = {}
        self.sites = []
        self._site_seen = set()
        self.unparsed = []
        self.registry_no_method = []
        self.forwarders = {}
        self.mocks = {}
        self.nested = {}
        self._pf_cache = {}
        self._member_cache = {}
        self._loose_cache = {}
        self._obj_spans = {}
        self._registries = {}
        for p in self.files:
            clean = strip_comments(read(p))
            rel = os.path.relpath(p, repo).replace("\\", "/")
            self.srcs[rel] = clean
            self.consts[rel] = const_map(clean)
            self.scopes[rel] = collect_scopes(clean)
            idx = build_function_index(clean)
            self.file_fns[rel] = idx
            for name, info in idx.items():
                rec = dict(info, file=rel)
                if info["top"]:
                    self.fns.setdefault(name, rec)
                else:
                    self.nested.setdefault(rel, {}).setdefault(name, rec)
        self.collect()

    # ---------------- helper 分类 ----------------
    def fn_info(self, name, rel=None):
        """先查顶层函数（可跨文件按名字解析），再查同文件的局部函数。"""
        info = self.fns.get(name)
        if info is not None:
            return name, info
        loc = self.nested.get(rel or "", {})
        if name in loc:
            return "%s#%s" % (rel, name), loc[name]
        return None, None
    @staticmethod
    def _driven_param(pexpr, consts, params):
        """入参到底是「整条路径」还是「只填其中一个段」？

        把每个入参换成哨兵后求归一路径：
          - 哨兵不在路径部分（只出现在 query，或整条被 query 截断）⇒ 不算路径来源；
          - 去掉哨兵段后仍有 ≥2 个字面段（如 /iam/roles/${roleId}/permissions）⇒ 路径骨架
            由 helper 自己写死，入参只填一个段 ⇒ 不算路径来源（这类位点应在 helper 体内
            直接计入，按调用位点展开反而会得到 `page`、`roleId` 这种无意义表达式）。
          - 其余（`${path}?page=1`、`${endpoint}/submit`、纯 `path`）⇒ 路径来源。
        """
        if not params:
            return None
        tmp = dict(consts)
        for p in params:
            if p:
                tmp[p] = SENTINEL
        val, _ = to_canonical(pexpr, tmp)
        if not val:
            # 求值失败时退回文本判断：整条表达式就是某个入参才算驱动
            for pi, p in enumerate(params):
                if p and re.fullmatch(re.escape(pexpr.strip()), p):
                    return pi
            return None
        segs = [s for s in val[len(API_BASE_PREFIX):].split("/") if s] if val.startswith(
            API_BASE_PREFIX) else [s for s in val.split("/") if s]
        if SENTINEL not in segs and not any(SENTINEL in s for s in segs):
            return None
        lit = [s for s in segs if SENTINEL not in s and s != "*"]
        if len(lit) >= 2:
            return None
        for pi, p in enumerate(params):
            if p and re.search(r"\b%s\b" % re.escape(p), pexpr):
                return pi
        return None

    def _is_pure_param_path(self, pexpr, consts, params):
        """路径是否**整条**由入参决定（`${path}`、`${endpoint}/submit` 之外无具名骨架）。

        只有这种位点才需要从调用位点展开、并在体内跳过；否则体内位点已足够表达契约。
        """
        tmp = dict(consts)
        for p in params:
            if p:
                tmp[p] = SENTINEL
        val, _ = to_canonical(pexpr, tmp)
        if not val:
            return bool(re.fullmatch(r"[A-Za-z_]\w*", pexpr.strip() or " "))
        segs = [s for s in val[len(API_BASE_PREFIX):].split("/") if s] if val.startswith(
            API_BASE_PREFIX) else [s for s in val.split("/") if s]
        lit = [s for s in segs if SENTINEL not in s and s != "*"]
        return not lit and any(SENTINEL in s for s in segs)

    @staticmethod
    def _call_paren(text, name_end):
        """名字之后跳过可选泛型实参再取 `(` 的下标；不平衡或没有 `(` 返回 -1。

        必须程序化扫尖括号：`apiCallRaw<Array<Record<string, unknown>>>(...)` 是三层嵌套，
        正则的 `<[^<>]*>` 只能吃两层，漏掉的这批位点会把真实覆盖判成零覆盖。
        """
        i = name_end
        n = len(text)
        while i < n and text[i] in " \t\n":
            i += 1
        if i < n and text[i] == "<":
            depth = 0
            while i < n:
                c = text[i]
                if c == "<":
                    depth += 1
                elif c == ">":
                    depth -= 1
                    if depth == 0:
                        i += 1
                        break
                elif c in "()":
                    return -1
                i += 1
            else:
                return -1
            while i < n and text[i] in " \t\n":
                i += 1
        return i if i < n and text[i] == "(" else -1

    def _raw_sites(self, text):
        """text 内的 R1/R2/R3 位点：[{kind, method, method_expr, path_expr, pos}]"""
        found = []
        for m in API_NAME_RE.finditer(text):
            name = m.group(1)
            if not name.startswith("apiCall") or name == "apiCallRawVersion":
                continue
            head = text[:m.start()].rstrip()
            # 声明位与类型位不是请求：`export async function apiCall<T = unknown>(` 的
            # 实参表会被当成位点，把类型注解误读成路径
            if head.endswith("function") or head.endswith("declare") or head.endswith("typeof"):
                continue
            op = self._call_paren(text, m.end())
            if op < 0:
                continue
            args, _ = split_args(text, op)
            if len(args) < 3:
                continue
            mm = re.match(r"^['\"](GET|POST|PUT|PATCH|DELETE)['\"]$", args[1])
            found.append({"kind": name, "method": mm.group(1) if mm else None,
                          "method_expr": None if mm else args[1], "path_expr": args[2],
                          "pos": m.start()})
        for m in REQ_NAME_RE.finditer(text):
            verb = m.group(2).lower()
            head = text[:m.start()].rstrip()
            if head.endswith("typeof") or head.endswith("function"):
                continue
            op = self._call_paren(text, m.end())
            if op < 0:
                continue
            args, _ = split_args(text, op)
            if not args:
                continue
            method = verb.upper() if verb != "fetch" else None
            method_expr = None
            if method is None and len(args) > 1:
                fm = re.search(r"method\s*:\s*(['\"](?:GET|POST|PUT|PATCH|DELETE)['\"]|[A-Za-z_]\w*)",
                               args[1])
                if fm:
                    tok = fm.group(1)
                    lit = re.match(r"^['\"](GET|POST|PUT|PATCH|DELETE)['\"]$", tok)
                    if lit:
                        method = lit.group(1)
                    else:
                        method_expr = tok  # 简写 `method,` / `method: m`：方法也来自变量
                method = method or "GET"
            if method is None:
                method = "GET"
            found.append({"kind": "request." + verb, "method": method, "method_expr": method_expr,
                          "path_expr": args[0], "pos": m.start()})
        # R3b 裸 fetch()：e2e 里确有绕过 page.request 的全局 fetch（helpers.ts:1896
        # `${API_BASE}/health`、:1918 `${API_BASE}${API_PREFIX}/init/initialize`、
        # flow/00-deploy-init.spec.ts:21）。不收这批会把真实发过请求的端点判成零覆盖。
        # 本循环必须与上面的 finditer 平级：嵌在里面时，一个 page.request 位点都没有的
        # 文件会连一枚 fetch 位点都采不到，且循环变量会被内层覆盖。
        for m in re.finditer(r"(?<![\w.$])fetch\s*\(", text):
            head = text[:m.start()].rstrip()
            if head.endswith(("function", "typeof", "await typeof")):
                continue
            op = text.index("(", m.start())
            args, _ = split_args(text, op)
            if not args:
                continue
            method = "GET"
            if len(args) > 1:
                fm = re.search(r"method\s*:\s*['\"](GET|POST|PUT|PATCH|DELETE)['\"]", args[1])
                if fm:
                    method = fm.group(1)
            found.append({"kind": "fetch", "method": method, "method_expr": None,
                          "path_expr": args[0], "pos": m.start()})
        return found

    @staticmethod
    def _expand_locals(expr, local_consts, rounds=3):
        """把体内 "const url = [模板串引用 path]" 这类中间变量展开一层层，
        直到能看出路径其实来自哪个入参（不展开就会漏判 forwarder，把 helper 的
        调用位点全丢，进而虚报零覆盖）。"""
        out = expr
        for _ in range(rounds):
            prev = out
            for k, v in local_consts.items():
                out = re.sub(r"\$\{\s*%s\s*\}" % re.escape(k), v, out)
                out = re.sub(r"(?<![\w.$])%s(?!\w)" % re.escape(k), "'%s'" % v, out)
            if out == prev:
                break
        return out

    @staticmethod
    def _body_asserts(body):
        """体内是否对响应「内容」下断言：expect/assert，或按字段缺失抛错。

        只按 HTTP 状态抛错（verifyEndpointHealthy 那套）不算：那只是可达性。
        状态面消息形态是「返回 404 / 服务器内部错误」，字段面消息形态是
        「返回数据缺少字段 / 缺少 items 数组」（helpers.ts:2971/2989 原文）。
        """
        if EXPECT_RE.search(body):
            return True
        return bool(re.search(r"throw new Error\([^;]*?(缺少|不是数组|字段|应为|不匹配)", body, re.S))

    def forwarder(self, name, rel=None, depth=0):
        """该函数是否把「入参」喂给真实请求的路径位？返回映射或 None。

        映射 = {path_idx, method|method_idx, asserts, void_ret, probe_only, via}
        - asserts：体内对响应内容有断言 ⇒ 该 helper 自带断言（C 档来源）；
        - void_ret：签名返回 Promise<void>；
        - probe_only：只回 void 且体内无内容断言 ⇒ 纯可达/状态面探针（B 档来源）。
        经由别的 forwarder 间接发出时（如 probeListApiIfConfigured → verifyEndpointHealthy），
        继承被调方的映射，并把 via 记下来，供报告点名"这个请求其实是几跳外发出来的"。
        顶层函数按名字跨文件解析；局部函数只在同文件内解析（避免同名串台）。
        """
        key, info = self.fn_info(name, rel)
        if key is None:
            return None
        if key in self.forwarders:
            return self.forwarders[key]
        if depth > 4:
            return None
        self.forwarders[key] = None
        params = info["params"]
        body = info["body"]
        if self.is_mock(name, rel, depth):
            return None
        local = const_map(body)
        for cand in self._raw_sites(body):
            pexpr = self._expand_locals(cand["path_expr"], local)
            pi = self._driven_param(pexpr, local, params)
            if pi is None:
                continue
            midx = None
            method = cand["method"]
            if method is None and cand["method_expr"]:
                for j, q in enumerate(params):
                    if q and re.search(r"\b%s\b" % re.escape(q), cand["method_expr"]):
                        midx = j
                        break
            self.forwarders[key] = {
                "path_idx": pi, "method": method, "method_idx": midx,
                "kind": cand["kind"], "asserts": self._body_asserts(body),
                "void_ret": "Promise<void>" in info["ret"], "probe_only": False, "via": None,
            }
            break
        if not self.forwarders[key]:
            reg = re.search(r"\.push\(\s*(\{[^{}]*\})", body)
            if reg and self._body_registers_request(body):
                obj = reg.group(1)[1:-1]
                idx = {}
                for item in obj.split(","):
                    kv = item.split(":", 1)
                    k = kv[0].strip()
                    v = (kv[1] if len(kv) > 1 else kv[0]).strip()
                    if k in ("path", "method"):
                        for pi, p in enumerate(params):
                            if p and re.fullmatch(re.escape(v), p or "\x00"):
                                idx[k] = pi
                                break
                if "path" in idx:
                    self.forwarders[key] = {
                        "path_idx": idx["path"], "method": None,
                        "method_idx": idx.get("method"), "kind": "register",
                        "asserts": self._body_asserts(body),
                        "void_ret": "Promise<void>" in info["ret"] or info["ret"].startswith("): void"),
                        "probe_only": False, "via": None,
                    }
        if not self.forwarders[key]:
            for callee in re.findall(r"\b([A-Za-z_]\w*)\s*\(", body):
                if callee == name:
                    continue
                sub = self.forwarder(callee, info["file"], depth + 1)
                if not sub:
                    continue
                merged = dict(sub)
                merged["via"] = callee
                merged["asserts"] = self._body_asserts(body) or sub["asserts"]
                merged["void_ret"] = "Promise<void>" in info["ret"]
                self.forwarders[key] = merged
                break
        if self.forwarders[key]:
            f = self.forwarders[key]
            f["probe_only"] = bool(f["void_ret"] and not f["asserts"])
        return self.forwarders[key]

    def is_mock(self, name, rel=None, depth=0):
        """只做拦截注册（context.route / route.fulfill）而不发真实请求的 helper。"""
        key, info = self.fn_info(name, rel)
        if key is None or depth > 4:
            return False
        if key in self.mocks:
            return self.mocks[key]
        self.mocks[key] = False
        body = info["body"]
        real = bool(API_CALL_RE.search(body)) or bool(PAGE_REQ_RE.search(body))
        self.mocks[key] = bool(MOCK_RE.search(body)) and not real
        return self.mocks[key]

    # ---------------- 位点采集 ----------------
    def enclosing_fn(self, rel, pos):
        """包含 pos 的最内层函数（局部函数优先于它所在的顶层函数）。"""
        best = None
        for name, info in self.file_fns[rel].items():
            a, b = info["range"]
            if a <= pos <= b and (best is None or (b - a) < (best[1]["range"][1] - best[1]["range"][0])):
                best = (name, info)
        return best if best else (None, None)

    @staticmethod
    def _body_registers_request(info_body):
        """「登记型」helper：体内把入参塞进 {method, path} 队列（不当场发请求）。

        deferCleanup(queue, method, path, label) 就是这个形态（helpers.ts:3556-3562
        `queue.push({ method, path, label })`），真正的请求由 flushDeferredCleanups
        转手 tryCleanup 发出。不当 forwarder 识别的话，这批路径字面量会在调用位点整片丢失。
        """
        return bool(re.search(r"\.push\(\s*\{[^{}]*\bmethod\b[^{}]*\bpath\b", info_body))

    def _path_literals(self, rel, src):
        """同文件里对象成员 `{ path: '...', method: '...' }` 的字面量清单，供矩阵位点展开。

        返回 (path 字面量, method 字面量或 None, 偏移)。方法优先取消费点上的字面量
        （tryCleanup(page, 'DELETE', c.path)），消费点也从对象取（c.method）时退回成员值。
        """
        if rel in self._pf_cache:
            return self._pf_cache[rel]
        lits = []
        for m in re.finditer(r"\bpath\s*:\s*(['\"`])([^'\"`\n]*)\1", src):
            ctx = src[max(0, m.start() - 200): m.end() + 200]
            mm = re.search(r"\bmethod\s*:\s*['\"]([A-Z]+)['\"]", ctx)
            lits.append((m.group(2), mm.group(1) if mm else None, m.start()))
        self._pf_cache[rel] = lits
        return lits

    def _extra_rules(self, rel, origin, src, consts):
        # R7 位置型：任一调用的实参里"方法字面量与路径字面量相邻"，即视为请求位点。
        # 覆盖 callB('POST', `/x/${id}`)、captureUiSubmit(page, url, 'PUT', …) 这类
        # 局部闭包/解构出来的封装（函数名解析不到，但实参形态无歧义）。
        for m in re.finditer(r"\b([A-Za-z_]\w*)\s*(?=<|\()", src):
            name = m.group(1)
            if name.startswith("apiCall") or self.forwarder(name, rel) or self.is_mock(name, rel):
                continue
            if name in ("apiCallRawVersion", "expect", "assert"):
                continue
            head = src[:m.start()].rstrip()
            if head.endswith(("function", "typeof", "declare", "const", "return")):
                continue
            op = self._call_paren(src, m.end())
            if op < 0:
                continue
            args, _ = split_args(src, op)
            if len(args) < 2:
                continue
            for i, a in enumerate(args):
                mm = re.match(r"^['\"](GET|POST|PUT|PATCH|DELETE)['\"]$", a)
                if not mm:
                    continue
                for j in (i - 1, i + 1):
                    if 0 <= j < len(args) and re.match(r"^['\"`]/[A-Za-z]", args[j]):
                        self._add(rel, origin, args[j], mm.group(1), None, m.start(),
                                  "heuristic:positional(%s)" % name, None, consts, src)
                        break

        # R8 观测型：waitForResponse 谓词里同时出现 URL 片段与方法比较。
        # 请求由前端页面发出，用例只等并断言响应 ⇒ 记 B（可达+状态面），不计深覆盖。
        for m in re.finditer(r"\bwaitForResponse\s*\(", src):
            op = self._call_paren(src, m.end() - 1)
            if op < 0:
                continue
            args, after = split_args(src, op)
            blob = " ".join(args)
            um = re.search(r"url\(\)\s*\.includes\(\s*([^)]{2,80}?)\s*\)", blob)
            meth = re.search(r"method\(\)\s*===\s*['\"](GET|POST|PUT|PATCH|DELETE)['\"]", blob)
            if um and meth:
                raw = um.group(1)
                # 谓词里可能写变量（`const importUrl = '/x/import'` 再 includes(importUrl)）：
                # 把原始实参文本交给 _add，由常量表解析，不要当成字面量加引号
                if raw.startswith(("'", '"', "`")):
                    inner = raw[1:-1] if raw[-1] == raw[0] else raw[1:]
                    if inner.startswith(API_BASE_PREFIX):
                        inner = inner[len(API_BASE_PREFIX):]
                    raw = "'%s'" % inner
                self._add(rel, origin, raw, meth.group(1), None, m.start(),
                          "observed:waitForResponse", None, consts, src, force_probe=True)

        # R6a 对象矩阵声明：method 只认本条目自己的顶层成员，缺省方法按数组名派生。
        for m in re.finditer(r"\bpath\s*:\s*(['\"`])([^'\"`\n]+)\1", src):
            span = self._entry_span(rel, src, m.start())
            meth = None
            if span:
                meth = self._entry_method(src, span)
            if meth:
                self._add(rel, origin, "'%s'" % m.group(2), meth, None, m.start(),
                          "registry:object-matrix", None, consts, src, force_probe=True)
            elif m.group(1) in ("'", '"'):
                arr = enclosing_const_name(src, span[0] if span else m.start())
                dft = REGISTRY_DEFAULT_METHOD.get(arr)
                if dft:
                    # 缺省方法逐数组有据：审批矩阵见 41-approve-endpoints.spec.ts:46
                    # `method ?? 'POST'` 与配置头注释「method：审批 HTTP 方法，缺省 POST」；
                    # 敏感导出数组的类型 Array<{path;resource}> 没有 method 成员，
                    # 消费点 39-export-endpoints.spec.ts:36 走 page.request.get ⇒ GET。
                    self._add(rel, origin, "'%s'" % m.group(2), dft, None, m.start(),
                              "registry:object-matrix(default-%s)" % dft, None, consts, src,
                              force_probe=True)
                elif span and arr.endswith("_ENDPOINTS"):
                    # 数组名按 _ENDPOINTS 结尾即声明自己是端点矩阵，却不在派生表里：
                    # 不编缺省方法，也不静默丢弃，记进 registry_no_method 响亮计数，
                    # 交人工补表（PATROL_ROUTES 与 LIST_PAGES 这类页面清单不以该后缀命名，
                    # 它们的 path 成员是 UI 路由不是请求路径，与改动前一样不进判定）。
                    self.registry_no_method.append(
                        {"file": rel, "line": line_of(src, m.start()),
                         "path": m.group(2), "array": arr})

    def _entry_span(self, rel, src, pos):
        """包住 pos 的最内层对象字面量区间 (开括号, 闭括号)，认多行条目形态。

        缺陷修正点：窗口不能靠上一条目的 `{ path:` 字面量定位。旧写法向前 rfind
        命中的是相邻条目自己的开括号，窗口里带着邻居的 method 成员，于是无 method
        的条目继承了前一条目的方法；而多行条目（`{` 与 `path:` 分行）根本不以
        `{ path:` 开头，前向 find 又会越过真正的下一条，把更大范围并进本条目。
        现在用一遍扫描的花括号配平区间取本条目自身，嵌套子对象取最内层。
        """
        spans = self._obj_spans.get(rel)
        if spans is None:
            spans = sorted(brace_spans(src))
            self._obj_spans[rel] = spans
        best = None
        for op, cl in spans:
            if op <= pos <= cl and (best is None or op > best[0]):
                best = (op, cl)
        return best

    @staticmethod
    def _entry_method(src, span):
        """取本条目顶层的 method 成员值（HTTP 方法字面量），没有或非法返回 None。"""
        members = entry_members(src, span[0], span[1])
        raw = (members.get("method") or "").strip()
        mm = re.match(r"^(?:['\"])(GET|POST|PUT|PATCH|DELETE)(?:['\"])$", raw)
        return mm.group(1) if mm else None

    # ---------------- 数据表字段与中间变量展开 ----------------
    def _member_table(self, rel, src):
        """同文件对象成员里写成路径字面量的字段表，字段名到取值与偏移的列表。

        调用方 _member_alts。入参 rel 是仓库相对路径，src 是 Analyzer 建好的去注释源码。
        结果只在本文件内生效并缓存，不跨文件串联同名字段。
        """
        if rel in self._member_cache:
            return self._member_cache[rel]
        table = defaultdict(list)
        for m in MEMBER_PATH_RE.finditer(src):
            table[m.group(1)].append((m.group(3), m.start()))
        self._member_cache[rel] = table
        return table

    @staticmethod
    def _inline_member_lits(expr, field):
        """实参本身就是对象字面量时，只取该对象内 field 成员的路径字面量。"""
        pat = r"(?<![\w.$])%s\s*:\s*(['\"`])(/[^'\"`\n]*)\1" % re.escape(field)
        return [m.group(2) for m in re.finditer(pat, expr)]

    @staticmethod
    def _degenerate(path):
        """求值结果只剩通配段（空、/api/v1/erp、/api/v1/erp/*、/*/*）时不可信，要再展开。"""
        if not path:
            return True
        rest = path[len(API_BASE_PREFIX):] if path.startswith(API_BASE_PREFIX) else path
        return not [s for s in rest.split("/") if s and s != "*"]

    @staticmethod
    def _ternary_alts(text):
        """顶层三元 cond 问号 a 冒号 b 拆成两个分支，非三元原样返回单元素列表。

        字符串、模板字面量与方括号整体视为一层，其中的问号与冒号不切；
        可选链与空值合并（问号后紧跟点号或问号）不是三元，误切会把可用路径劈成幽灵。
        """
        quote = ""
        depth = 0
        qi = -1
        i = 0
        n = len(text)
        while i < n:
            c = text[i]
            if quote:
                if c == "\\":
                    i += 2
                    continue
                if c == quote:
                    quote = ""
                i += 1
                continue
            if c in "'\"`":
                quote = c
                i += 1
                continue
            if c in "([{":
                depth += 1
            elif c in ")]}":
                if depth:
                    depth -= 1
            elif c == "?":
                nxt = text[i + 1] if i + 1 < n else ""
                if depth == 0 and qi < 0 and nxt not in "?.":
                    qi = i
            elif c == ":" and depth == 0 and qi >= 0:
                return [text[qi + 1:i].strip(), text[i + 1:].strip()]
            i += 1
        return [text]

    @staticmethod
    def _loose_map(body):
        """同函数体内 const/let 到右值原文的表，三元与模板串都按原文保留。

        const_map 只收以引号开头的字面量，形如 const delPath = 条件 问号 分支 冒号 分支
        的中间变量收不到，删除腿与详情回读腿会整片判成零覆盖。
        """
        out = {}
        for m in LOOSE_DECL_RE.finditer(body):
            i = m.end()
            buf = []
            depth = 0
            quote = ""
            while i < len(body):
                c = body[i]
                if quote:
                    if c == "\\":
                        buf.append(c)
                        if i + 1 < len(body):
                            buf.append(body[i + 1])
                        i += 2
                        continue
                    if c == quote:
                        quote = ""
                    buf.append(c)
                    i += 1
                    continue
                if c in "'\"`":
                    quote = c
                    buf.append(c)
                    i += 1
                    continue
                if c in "([{":
                    depth += 1
                elif c in ")]}":
                    if depth == 0:
                        break
                    depth -= 1
                elif c in ";," and depth == 0:
                    break
                buf.append(c)
                i += 1
            val = "".join(buf).strip()
            if val and len(val) <= 400:
                out.setdefault(m.group(1), val)
        return out

    def _locals_of(self, rel, pos):
        """位点所在函数体内的中间变量表，按文件与函数区间缓存。"""
        fname, finfo = self.enclosing_fn(rel, pos)
        if finfo is None:
            return {}
        key = (rel, fname, finfo["range"])
        if key not in self._loose_cache:
            self._loose_cache[key] = self._loose_map(finfo["body"])
        return self._loose_cache[key]

    def _member_alts(self, expr, rel, src, consts, pos):
        """把成员访问与内联对象实参两类数据表位点展开成候选表达式列表。

        形态 A 成员访问（形如 c.createApi）用同文件该字段的全部路径字面量逐个代入，
        归属口径与既有 R6 矩阵一致，按文件级而非按具体数组，展开位点记在成员声明处。
        形态 B 实参本身就是对象字面量，只取该对象内白名单成员的取值，不混入文件级同名成员。
        字段字面量数超上限时登记未解析，不硬吃进分子。
        """
        table = self._member_table(rel, src)
        if not table and not expr.lstrip().startswith("{"):
            return []
        inline = expr.lstrip().startswith("{")
        if inline:
            cands = sorted(API_MEMBER_FIELDS)
        else:
            cands = sorted(
                set(re.findall(r"[A-Za-z_$][\w$]*\s*\.\s*([A-Za-z_$][\w$]*)", expr))
                & set(table))
        out = []
        for field in cands:
            if inline:
                lits = self._inline_member_lits(expr, field)
                tmpl = MEMBER_PLACEHOLDER
                if not lits:
                    continue
            else:
                acc = re.compile(r"(?<![\w.$])[A-Za-z_$][\w$]*\s*\.\s*%s\b"
                                 % re.escape(field), re.A)
                if not acc.search(expr):
                    continue
                lits = [v for (v, _p) in table[field]]
                tmpl = acc.sub(MEMBER_PLACEHOLDER, expr)
            if len(lits) > MEMBER_FANOUT_LIMIT:
                self.unparsed.append({
                    "file": rel, "line": line_of(src, pos),
                    "expr": re.sub(r"\s+", " ", expr[:80]),
                    "why": "member-fanout-too-many:%s" % field, "how": "expand"})
                continue
            for lit in lits:
                child = dict(consts)
                child[MEMBER_PLACEHOLDER] = lit
                out.append((tmpl, child))
        return out

    def _resolve_paths(self, expr, rel, src, consts, pos, locals_, depth=0):
        """求一个路径表达式的可信取值集合，返回（（路径、来源）列表、未解析原因）。

        来源三值：plain 是表达式自身定型，member 是数据表成员字面量代入，
        branch 是三元分支展开后由分支自身决定的来源。
        顺序：三元分支优先（问号冒号在顶层时，整体求值会把分支文本截成假路径段），
        其次原式求值；结果退化、求值失败或实参是对象字面量时，再依次试
        同函数体内中间变量右值与数据表成员字面量展开。展开只在原求值不可信时介入。
        """
        alts = self._ternary_alts(expr)
        if len(alts) > 1 and depth < 3:
            got = []
            for alt in alts:
                sub, _u = self._resolve_paths(alt, rel, src, consts, pos, locals_, depth + 1)
                got.extend(sub)
            if got:
                return got, ""
        path, unres = to_canonical(expr, consts)
        if path and not self._degenerate(path) and not expr.lstrip().startswith("{"):
            return [(path, "plain")], unres or ""
        if depth >= 3:
            return ([(path, "plain")] if path else []), unres or ""
        got = []
        if unres:
            for nm in [x for x in unres.split(",") if x]:
                rhs = locals_.get(nm)
                if rhs:
                    sub, _u = self._resolve_paths(rhs, rel, src, consts, pos,
                                                  locals_, depth + 1)
                    got.extend(sub)
        if not got:
            for e2, child in self._member_alts(expr, rel, src, consts, pos):
                p2, _u2 = to_canonical(e2, child)
                if p2 and not self._degenerate(p2):
                    got.append((p2, "member"))
        if got:
            return got, ""
        return ([(path, "plain")] if path else []), unres or ""

    def collect(self):
        for rel in sorted(self.srcs):
            src = self.srcs[rel]
            consts = self.consts[rel]
            origin = origin_of(rel)
            # R1/R2/R3 直接位点
            for cand in self._raw_sites(src):
                fname, finfo = self.enclosing_fn(rel, cand["pos"])
                if finfo is not None:
                    local = const_map(finfo["body"])
                    expanded = self._expand_locals(cand["path_expr"], local)
                    pi = self._driven_param(expanded, local, finfo["params"])
                    if (pi is not None and self.forwarder(fname, rel) is not None
                            and self._is_pure_param_path(expanded, local, finfo["params"])):
                        # 整条路径只由入参决定：体内位点得不到任何契约信息，交给调用位点计；
                        # 骨架里还有具名段（/print-targets/${id}）时体内位点必须保留，
                        # 否则该函数被别处以变量调用时会把真实覆盖整片判成零覆盖
                        continue
                self._add(rel, origin, cand["path_expr"], cand["method"], cand["method_expr"],
                          cand["pos"], "direct", None, consts, src)
            # R4 forwarder 调用位点（跳过其自身体内的定义处）
            for m in re.finditer(r"\b([A-Za-z_]\w*)\s*\(", src):
                fname = m.group(1)
                if fname.startswith("apiCall"):
                    continue
                fwd = self.forwarder(fname, rel)
                if not fwd:
                    continue
                encl, _ = self.enclosing_fn(rel, m.start())
                if encl == fname:
                    continue
                decl = self.file_fns[rel].get(fname)
                if decl is not None and decl["line"] == line_of(src, m.start()):
                    # 函数声明行本身不是调用位点。早先没这条时，`async function f(page, path)`
                    # 这处匹配会把形参列表当实参取，得到 `path: string` 之类文本并计入未解析
                    # 位点：每多一个带路径形参的 helper 就凭空多一条噪声，既抬高未解析计数，
                    # 又会盖住真正该报的形态（调用点把路径写成变量 ⇒ 证据隐形）。
                    # 声明行上可能存在的真实调用（一行式体内的递归）已由上面的 enclosing
                    # 判据先跳过，这条只摘掉声明名字那一个 token。
                    continue
                args, _ = split_args(src, src.index("(", m.end() - 1))
                pidx = fwd["path_idx"]
                if pidx >= len(args):
                    continue
                method = fwd["method"]
                if method is None and fwd["method_idx"] is not None and fwd["method_idx"] < len(args):
                    mm = re.match(r"^['\"](GET|POST|PUT|PATCH|DELETE)['\"]$", args[fwd["method_idx"]])
                    method = mm.group(1) if mm else None
                # R6 矩阵型：路径写成 { path: '/x/1' } 对象成员，调用位点只传 c.path。
                # 按同文件的 path 字面量逐条展开（方法与消费点一致），否则整片清理/矩阵
                # 路径会判成"从未出现"，这是上一轮 depth 体检同源的盲区。
                if re.fullmatch(r"[A-Za-z_]\w*\.path", args[pidx].strip()):
                    lits = self._path_literals(rel, src)
                    if not lits:
                        self.unparsed.append({
                            "file": rel, "line": line_of(src, m.start()),
                            "expr": args[pidx].strip()[:80],
                            "why": "matrix-path-no-literal", "how": "via:" + fname})
                        continue
                    for lit, lmethod, lpos in lits:
                        self._add(rel, origin, "'%s'" % lit, method or lmethod, None, lpos,
                                  "matrix:%s" % fname, fname, consts, src, fwd=fwd)
                    continue
                self._add(rel, origin, args[pidx], method, None, m.start(), "via:" + fname,
                          fname, consts, src, fwd=fwd)
            # R6a/R7/R8 补充规则（对象矩阵声明、位置型实参对、观测型谓词）
            self._extra_rules(rel, origin, src, consts)
            # R5 显式注册表
            mm = ROUTES_DECL_RE.search(src)
            if mm:
                close = src.find("\n]", mm.end())
                block = src[mm.end(): close if close > 0 else len(src)]
                for lm in re.finditer(r"['\"](/[^'\"]*)['\"]", block):
                    self._add(rel, origin, "'%s'" % lm.group(1), "GET", None, mm.end() + lm.start(),
                              "registry:ROUTES", None, consts, src, force_probe=True)
            for lm in LISTAPI_RE.finditer(src):
                self._add(rel, origin, "'%s'" % lm.group(2), "GET", None, lm.start(),
                          "registry:listApi", None, consts, src, force_probe=True)

    def _add(self, rel, origin, expr, method, method_expr, pos, how, helper, consts, src,
             fwd=None, force_probe=False):
        line = line_of(src, pos)
        resolved, unres = self._resolve_paths(
            expr, rel, src, consts, pos, self._locals_of(rel, pos))
        if not resolved:
            self.unparsed.append({"file": rel, "line": line,
                                  "expr": re.sub(r"\s+", " ", (expr or "")[:80]),
                                  "why": unres or "empty", "how": how})
            return
        if method is None and method_expr:
            mm = re.match(r"^['\"](GET|POST|PUT|PATCH|DELETE)['\"]$", method_expr.strip())
            method = mm.group(1) if mm else None
        if method is None:
            self.unparsed.append({"file": rel, "line": line,
                                  "expr": re.sub(r"\s+", " ", (expr or "")[:80]),
                                  "why": "method-unknown", "how": how})
            return
        # 数据表成员展开的位点固定记 B：展开按文件级成员做代入，只能证明「这条路径真的
        # 被这个方法的请求打过」，证明不了该取值有字段断言或回读，不能靠展开抬成深覆盖。
        # 方法维度始终严格：同路径的其他方法一律不顶替，快照未挂该方法就落方法不符。
        for path, kind in sorted(set(resolved)):
            how_full = how if kind == "plain" else "%s|%s" % (how, kind)
            sig = (rel, line, method, path, how_full)
            if sig in self._site_seen:
                continue
            self._site_seen.add(sig)
            if kind == "member":
                strength = "B"
                reason = "数据表字段驱动位点：只证该方法真实发出过，不断字段"
            else:
                strength, reason = self._strength(rel, src, pos, method, path,
                                                  force_probe, fwd, how_full)
            self.sites.append({"file": rel, "line": line, "method": method, "path": path,
                               "expr": re.sub(r"\s+", " ", (expr or "")[:60]),
                               "how": how_full, "origin": origin,
                               "strength": strength, "reason": reason})

    # ---------------- 深浅判定 ----------------
    def _strength(self, rel, src, pos, method, path, force_probe, fwd, how):
        if force_probe:
            return "B", "注册表探针：只证可达，不断字段"
        if fwd is not None:
            who = fwd.get("via") or how.split(":", 1)[-1]
            if "leanup" in who or "leanup" in how:
                return "B", "清理/登记位点：失败仅告警，不参与断言"
            if fwd.get("probe_only"):
                return "B", "经 %s（返回 void 且体内无内容断言，仅状态面）" % who
            if fwd.get("asserts"):
                return "C", "经 %s 体内 expect 断言" % who
        lo, hi = innermost_scope(self.scopes[rel], pos, len(src))
        before = src[lo:pos]
        after = src[pos:hi]
        if re.search(r"(?:expect|assert)[A-Za-z0-9_.]*\s*(?:\.\w+)*\s*\(\s*(?:await\s+)?\s*$", before):
            return "C", "调用被包在 expect(...) 内"
        var = None
        bm = re.search(r"(?:const|let|var)\s+([A-Za-z_]\w*|\{[^{}]*\})\s*=\s*(?:await\s+)?\s*$",
                       before)
        if bm:
            var = bm.group(1)
            if var.startswith("{"):
                names = re.findall(r"[A-Za-z_]\w*", var)
                var = names[0] if names else None
        if var:
            m = re.search(r"\b%s\b" % re.escape(var), after)
            if m:
                seg = after[m.start():m.start() + 400]
                if re.search(r"\b(?:expect|assert)[A-Za-z0-9_.]*\s*\(|\bthrow\b|!==|===|if\s*\(", seg):
                    return "C", "返回值 %s 被后续断言/终态判定使用" % var
        if method in WRITE_METHODS:
            rb = self._readback(after, path)
            if rb:
                return "C", "写后同作用域 GET 回读 %s" % rb
            return "B", "写口无回读、结果未断言（只信 HTTP 码）"
        if var:
            return "B", "返回值 %s 绑定后未被断言" % var
        if EXPECT_RE.search(after) and re.search(r"\.(code|data|status|items)\b", after):
            return "C", "同作用域对响应体字段有断言"
        return "B", "结果未被取用（无 expect/回读/字段断言）"

    def _readback(self, after, path):
        """同作用域内、站点之后，是否出现同路径或父/子路径的 GET 回读。"""
        for m in re.finditer(
                r"\b(apiCall|apiCallRaw|safeGet|safeGetList|verifyEndpointHealthy|request\.get)\s*\(",
                after):
            name = m.group(1)
            args, _ = split_args(after, after.index("(", m.end() - 1))
            if name in ("apiCall", "apiCallRaw"):
                if len(args) < 3 or not re.match(r"^['\"]GET['\"]$", args[1]):
                    continue
                p, _ = to_canonical(args[2], {})
            elif name in ("safeGet", "safeGetList", "verifyEndpointHealthy"):
                if len(args) < 2:
                    continue
                p, _ = to_canonical(args[1], {})
            else:
                if not args:
                    continue
                p, _ = to_canonical(args[0], {})
            if not p:
                continue
            if p == path or path.startswith(p + "/") or p.startswith(path.rsplit("/", 1)[0] + "/"):
                return p
        return None


# ---------------------------------------------------------------- 汇总
def module_of(path):
    """按 /api/v1/erp 之后的第一段聚模块；快照第 4 列（routes_file）另作对照用。"""
    if path.startswith(API_BASE_PREFIX):
        rest = path[len(API_BASE_PREFIX):]
    else:
        rest = path
    seg = [s for s in rest.split("/") if s]
    return seg[0] if seg else "(root)"


def analyze(repo, list_limit):
    snap_path, header, rows = load_snapshot(repo)
    an = Analyzer(repo)

    snap_keys = {}
    for r in rows:
        snap_keys["%s %s" % (r["method"], r["path"])] = r
    snap_paths = defaultdict(set)
    for r in rows:
        snap_paths[r["path"]].add(r["method"])

    hit = defaultdict(list)
    ghost = []          # e2e 打了快照里没有的路径
    method_mismatch = defaultdict(list)
    for s in an.sites:
        key = "%s %s" % (s["method"], s["path"])
        if key in snap_keys:
            hit[key].append(s)
        elif s["path"] in snap_paths:
            method_mismatch[key].append(s)
            ghost.append({"kind": "method-not-registered", **s,
                          "registered": sorted(snap_paths[s["path"]])})
        else:
            ghost.append({"kind": "path-not-registered", **s})

    tierA, tierB, tierC = [], [], []
    for key, r in sorted(snap_keys.items()):
        if key not in hit:
            tierA.append(key)
        elif any(s["strength"] == "C" for s in hit[key]):
            tierC.append(key)
        else:
            tierB.append(key)

    # 豁免剔除在分档完成后立即执行：analyze 的返回值即净 A 档，
    # 打印层与任何直接消费 tierA 的脚本（#139 残量统计等）口径唯一。
    exempt_hits = [k for k in tierA if k in COVERAGE_EXEMPT]
    tierA = [k for k in tierA if k not in COVERAGE_EXEMPT]
    return {"snap_path": snap_path, "header": header, "rows": rows, "snap_keys": snap_keys,
            "snap_paths": snap_paths, "hit": hit, "tierA": tierA, "exempt_hits": exempt_hits, "tierB": tierB, "tierC": tierC,
            "ghost": ghost, "unparsed": an.unparsed, "method_mismatch": method_mismatch,
            "registry_no_method": an.registry_no_method,
            "files": an.files, "forwarders": an.forwarders}


# ---------------------------------------------------------------- 自证检测力
# 夹具仓库：七条在册端点，每档覆盖来源彼此隔离，任何一类位点被摘掉都必须让 A 变大。
# 落盘位置由命令行入参给出，源码里不写死任何环境的绝对路径。
SELFTEST_SNAPSHOT = "\n".join([
    "# layers=1 endpoints=7",
    "GET\t/api/v1/erp/webhooks\tlist_webhooks\twebhook.rs",
    "GET\t/api/v1/erp/reports/export/{id}\texport_report\treport.rs",
    "POST\t/api/v1/erp/crm/customers\tcreate_customer\tcrm.rs",
    "GET\t/api/v1/erp/crm/customers\tlist_customers\tcrm.rs",
    "DELETE\t/api/v1/erp/crm/customers/{id}\tdelete_customer\tcrm.rs",
    "GET\t/api/v1/erp/advanced/quote-converts\tlist_quote_convert\tadvanced.rs",
    "POST\t/api/v1/erp/advanced/ai/recipe-optimization\toptimize_recipe\tadvanced.rs",
    "",
])

SELFTEST_HELPERS = r"""
export const API_BASE = process.env.API_BASE || 'http://localhost:8082';
export const API_PREFIX = '/api/v1/erp';

export async function apiCallRaw<T>(page: Page, method: string, path: string): Promise<Response> {
  return fetch(`${API_BASE}${API_PREFIX}${path}`, { method });
}

export async function apiCall<T>(page: Page, method: string, path: string, body?: unknown): Promise<ApiResp<T>> {
  const res = await apiCallRaw(page, method, path);
  return (await res.json()) as ApiResp<T>;
}

export async function verifyEndpointHealthy(page: Page, path: string): Promise<void> {
  try {
    const res = await apiCallRaw(page, 'GET', path);
    void res;
  } catch (e) {
    throw new Error(`GET ${path} 探活失败`);
  }
}

export async function verifyDownloadEndpointHealthy(page: Page, path: string): Promise<void> {
  const res = await page.request.get(`${API_BASE}${API_PREFIX}${path}`);
  if (res.status() >= 500) {
    throw new Error(`GET ${path} 下载端点服务错误`);
  }
}
"""

SELFTEST_HELPER_SPEC = r"""
import { verifyEndpointHealthy, verifyDownloadEndpointHealthy } from './helpers';

test.describe('夹具一 helper 包装探活', () => {
  test('列表探活', async ({ page }) => {
    await verifyEndpointHealthy(page, '/webhooks?page=1&page_size=5');
    await verifyDownloadEndpointHealthy(page, '/reports/export/123');
  });
});
"""

SELFTEST_MATRIX_SPEC = r"""
import { apiCall } from './helpers';

interface DelCase {
  label: string;
  createApi: string;
  payload: Record<string, unknown>;
  deleteApi?: (id: number) => string;
}

async function createThenApiDelete(page: Page, c: DelCase): Promise<void> {
  const resp = await apiCall<{ id?: number }>(page, 'POST', c.createApi, c.payload);
  const id = resp?.data?.id ?? 1;
  const listResp = await page.request.get(`${API_BASE}${API_PREFIX}${c.createApi}?page=1&page_size=200`);
  if (!listResp.ok()) {
    throw new Error('列表回读失败');
  }
  const delPath = c.deleteApi ? c.deleteApi(id) : `${c.createApi}/${id}`;
  await apiCall(page, 'DELETE', delPath);
}

const CASES: DelCase[] = [
  { label: '客户', createApi: '/crm/customers', payload: { name: '夹具客户' } },
];

test('夹具二 删除矩阵', async ({ page }) => {
  for (const c of CASES) {
    await createThenApiDelete(page, c);
  }
});
"""

SELFTEST_REQUEST_SPEC = r"""
test('夹具三 直连读请求', async ({ page }) => {
  const res = await page.request.get(`${API_BASE}${API_PREFIX}/advanced/quote-converts?page=1`);
  expect(res.ok()).toBeTruthy();
});
"""

SELFTEST_PROBE_SPEC = r"""
const methodGateCases: { path: string; evidence: string }[] = [
  { path: '/advanced/ai/recipe-optimization', evidence: '快照仅 POST' },
];

test('夹具四 方法门负探针', async ({ page }) => {
  for (const c of methodGateCases) {
    const resp = await page.request.get(`${API_BASE}${API_PREFIX}${c.path}`);
    expect(resp.status(), '负向方法门期望 405').toBe(405);
  }
});
"""

SELFTEST_GHOST_SPEC = r"""
import { apiCall } from './helpers';

test('夹具五 快照外路径', async ({ page }) => {
  const r = await apiCall(page, 'GET', '/not-registered/thing');
  expect(r?.data).toBeTruthy();
});
"""

SELFTEST_FILES = {
    "frontend/e2e/flow/helpers.ts": SELFTEST_HELPERS,
    "frontend/e2e/flow/10-helper-probe.spec.ts": SELFTEST_HELPER_SPEC,
    "frontend/e2e/flow/20-delete-matrix.spec.ts": SELFTEST_MATRIX_SPEC,
    "frontend/e2e/flow/30-page-request.spec.ts": SELFTEST_REQUEST_SPEC,
    "frontend/e2e/flow/40-method-gate.spec.ts": SELFTEST_PROBE_SPEC,
    "frontend/e2e/flow/50-ghost.spec.ts": SELFTEST_GHOST_SPEC,
}

# 变异自证：每次只摘掉一类位点的发起，A 必须变大（负探针那次必须不变）。
# 调用方 self_test，入参是夹具文件表与替换对，产物只打标准输出，不落仓库。
SELFTEST_MUTATIONS = (
    ("摘除 helper 包装探活", "frontend/e2e/flow/10-helper-probe.spec.ts",
     "    await verifyEndpointHealthy(page, '/webhooks?page=1&page_size=5');\n", ""),
    ("摘除下载探活", "frontend/e2e/flow/10-helper-probe.spec.ts",
     "    await verifyDownloadEndpointHealthy(page, '/reports/export/123');\n", ""),
    ("摘除数据表字段驱动", "frontend/e2e/flow/20-delete-matrix.spec.ts",
     "  { label: '客户', createApi: '/crm/customers', payload: { name: '夹具客户' } },\n", ""),
    ("摘除 page.request 直连", "frontend/e2e/flow/30-page-request.spec.ts",
     "  const res = await page.request.get(`${API_BASE}${API_PREFIX}/advanced/quote-converts?page=1`);\n", ""),
    ("摘除方法门负探针", "frontend/e2e/flow/40-method-gate.spec.ts",
     "    const resp = await page.request.get(`${API_BASE}${API_PREFIX}${c.path}`);\n", ""),
)


def build_selftest_repo(base, overrides=None):
    """在 base 下铺一份夹具仓库。返回仓库绝对路径，调用方 self_test。"""
    repo = os.path.join(base, "cov_selftest")
    for rel, content in SELFTEST_FILES.items():
        p = os.path.join(repo, *rel.split("/"))
        if not os.path.isdir(os.path.dirname(p)):
            os.makedirs(os.path.dirname(p))
        text = SELFTEST_FILES[rel] if overrides is None else overrides.get(rel, SELFTEST_FILES[rel])
        io.open(p, "w", encoding="utf-8", newline="\n").write(text)
    snap = os.path.join(repo, "frontend", "scripts", "route-snapshot.txt")
    if not os.path.isdir(os.path.dirname(snap)):
        os.makedirs(os.path.dirname(snap))
    io.open(snap, "w", encoding="utf-8", newline="\n").write(SELFTEST_SNAPSHOT)
    return repo


def _broken_lane(base, name, snap_text=None, with_e2e=False):
    """铺一个故意残缺的夹具车道，专供假零防御取证。返回车道绝对路径。

    调用方 self_test，传给 analyze 作为输入。snap_text 为 None 时不写快照文件，
    with_e2e 为真时补一个空的 frontend/e2e 目录。本函数不做任何判读，只造残缺输入。
    """
    repo = os.path.join(base, name)
    if not os.path.isdir(repo):
        os.makedirs(repo)
    if snap_text is not None:
        d = os.path.join(repo, "frontend", "scripts")
        if not os.path.isdir(d):
            os.makedirs(d)
        io.open(os.path.join(d, "route-snapshot.txt"), "w", encoding="utf-8", newline="\n").write(snap_text)
    if with_e2e:
        d = os.path.join(repo, "frontend", "e2e")
        if not os.path.isdir(d):
            os.makedirs(d)
    return repo


def self_test(base):
    """自证判定器有检测力，返回失败条目的列表（空即全过），并附夹具数、变异轮数与假零例数。

    五个夹具判据加五轮变异：夹具必须把三类间接位点认成覆盖、把快照外路径与
    方法不符的负探针挡在覆盖之外；每摘掉一类发起，A 档必须变大。
    四类假零防御：喂不存在的车道、缺 e2e 目录、空快照与截断快照，
    必须响亮报错而不是静默报零，报零的判定器等于把输入缺失伪装成覆盖达标。
    """
    res = analyze(build_selftest_repo(base), 20)
    A, B, C = set(res["tierA"]), set(res["tierB"]), set(res["tierC"])
    covered = B | C
    checks = [
        ("helper 包装探活被认成覆盖", "GET /api/v1/erp/webhooks" in covered),
        ("下载探活被认成覆盖", "GET /api/v1/erp/reports/export/*" in covered),
        ("下载探活只记浅覆盖不抬深", "GET /api/v1/erp/reports/export/*" in B
         and "GET /api/v1/erp/reports/export/*" not in C),
        ("数据表字段创建的 POST 被认成覆盖", "POST /api/v1/erp/crm/customers" in covered),
        ("数据表字段回读的 GET 被认成覆盖", "GET /api/v1/erp/crm/customers" in covered),
        ("数据表默认删除腿的 DELETE 被认成覆盖", "DELETE /api/v1/erp/crm/customers/*" in covered),
        ("page.request 直连被认成覆盖", "GET /api/v1/erp/advanced/quote-converts" in covered),
        ("快照外路径不进任何档",
         not any(p == "/api/v1/erp/not-registered/thing" for _m, p in
                 (k.split(" ", 1) for k in A | B | C))),
        ("快照外路径被登记成对不上快照",
         any(g["kind"] == "path-not-registered" and g["path"] == "/api/v1/erp/not-registered/thing"
             for g in res["ghost"])),
        ("方法门负探针不顶替在册方法", "POST /api/v1/erp/advanced/ai/recipe-optimization" in A),
        ("方法门负探针被登记成方法不符",
         any(m == "GET /api/v1/erp/advanced/ai/recipe-optimization"
             for m in res["method_mismatch"])),
        ("基线 A 档只余负探针一条", len(A) == 1),
    ]
    lines = ["== 自证夹具（基线 A=%d，基线位点=%d）==" % (len(A), sum(len(v) for v in res["hit"].values()))]
    fails = []
    for label, ok in checks:
        lines.append("    %-42s %s" % (label, "PASS" if ok else "FAIL"))
        if not ok:
            fails.append(label)
    lines.append("== 变异自证（每轮只摘一类发起，A 必须变大；负探针必须不变）==")
    lines.append("    基线 A=%d  %s" % (len(A), " ".join(sorted(A))))
    for label, rel, needle, repl in SELFTEST_MUTATIONS:
        text = SELFTEST_FILES[rel].replace(needle, repl, 1)
        if text == SELFTEST_FILES[rel]:
            lines.append("    %-28s FAIL 变异夹具没命中要摘的发起" % label)
            fails.append(label)
            continue
        mut = analyze(build_selftest_repo(base, {rel: text}), 20)
        grew = len(mut["tierA"]) > len(A)
        probe_round = label.endswith("负探针")
        want = (not grew) if probe_round else grew
        delta = len(mut["tierA"]) - len(A)
        lines.append("    %-28s %s  A=%d 变化%+d  期望%s" % (
            label, "PASS" if want else "FAIL", len(mut["tierA"]), delta,
            "不变" if probe_round else "变大"))
        if not want:
            fails.append(label)
    # 假零防御，四类残缺输入各喂一次。判据只看「响亮报错」，静默返回结果即判 FAIL。
    # 报错文本必须点出缺的是哪个输入，否则等于把输入缺失伪装成覆盖达标。
    lines.append("== 假零防御（残缺输入必须响亮报错，不许静默报零）==")
    zero_specs = [
        ("不存在的车道目录", os.path.join(base, "cov_zero_no_such_lane")),
        ("快照在但 e2e 目录缺失", _broken_lane(base, "cov_zero_no_e2e", SELFTEST_SNAPSHOT)),
        ("快照存在但零条端点行", _broken_lane(base, "cov_zero_empty_snap",
                                              "# layers=0 endpoints=0\n", True)),
        ("头行与行数不符的截断快照", _broken_lane(base, "cov_zero_truncated",
                                                  "# layers=1 endpoints=3\nGET\t/api/v1/erp/webhooks\tlist_webhooks\twebhook.rs\n", True)),
    ]
    for label, path in zero_specs:
        try:
            r = analyze(path, 20)
        except SystemExit as e:
            msg = e.code if isinstance(e.code, str) else ""
            if msg.strip():
                lines.append("    %-30s PASS 响亮报错 %s" % (label, msg.strip().split("\n")[0][:76]))
            else:
                lines.append("    %-30s FAIL 报错没点出缺哪个输入" % label)
                fails.append(label)
        else:
            lines.append("    %-30s FAIL 静默返回 端点=%d A=%d" % (label, len(r["snap_keys"]), len(r["tierA"])))
            fails.append(label)
    text = "\n".join(lines) + "\n"
    stream = getattr(sys.stdout, "buffer", None)
    if stream is not None:
        stream.write(text.encode("utf-8"))
        stream.flush()
    else:
        sys.stdout.write(text)
    return fails, len(checks), len(SELFTEST_MUTATIONS), len(zero_specs)


def gap_board(res, limit, by_routes_file=False):
    per = defaultdict(list)
    for key in res["tierA"]:
        r = res["snap_keys"][key]
        mod = r["routes_file"] if by_routes_file else module_of(r["path"])
        per[mod].append(key)
    order = sorted(per.items(), key=lambda kv: (-len(kv[1]), kv[0]))
    lines = []
    for mod, keys in order:
        if by_routes_file:
            total_in_mod = sum(1 for r in res["rows"] if r["routes_file"] == mod)
        else:
            total_in_mod = sum(1 for r in res["rows"] if module_of(r["path"]) == mod)
        lines.append((mod, len(keys), total_in_mod, sorted(keys)[:limit]))
    return order, lines


def main():
    sys.setrecursionlimit(100000)
    repo = None
    limit = 20
    show_sites = False
    do_selftest = False
    argv = sys.argv[1:]
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--list-limit":
            limit = int(argv[i + 1])
            i += 2
        elif a == "--sites":
            show_sites = True
            i += 1
        elif a == "--self-test":
            do_selftest = True
            i += 1
        elif a.startswith("-"):
            i += 1
        else:
            repo = a.rstrip("/\\")
            i += 1
    if do_selftest:
        if not repo:
            sys.exit("自证需要一个可写的工作目录作为入参，夹具仓库铺在它下面的 cov_selftest")
        if not os.path.isdir(repo):
            os.makedirs(repo)
        fails, n_fix, n_mut, n_zero = self_test(repo)
        if fails:
            print("SELFTEST_FAIL %d 项：%s" % (len(fails), "、".join(fails)))
            sys.exit(1)
        print("SELFTEST_OK 夹具 %d 项与变异 %d 轮与假零防御 %d 例全部符合预期" % (n_fix, n_mut, n_zero))
        return
    if not repo:
        sys.exit(__doc__)

    res = analyze(repo, limit)
    rows, keys = res["rows"], res["snap_keys"]
    uniq_paths = len(res["snap_paths"])
    A, B, C = res["tierA"], res["tierB"], res["tierC"]

    exempt_hits = res["exempt_hits"]
    stale_exempt = [k for k in COVERAGE_EXEMPT if k not in exempt_hits and k in res["snap_keys"]]
    gone_exempt = [k for k in COVERAGE_EXEMPT if k not in res["snap_keys"]]
    sites = res["hit"]
    total_sites = sum(len(v) for v in sites.values())
    hit_paths = set(k.split(" ", 1)[1] for k in sites)

    out = []
    w = out.append
    w("route -> e2e 覆盖判定  仓库 %s" % repo)
    w("分母源 %s" % res["snap_path"])
    w("快照头 %s" % res["header"])
    w("(METHOD,PATH) 端点=%d  按路径去重=%d" % (len(keys), uniq_paths))
    w("分子：e2e 文件=%d  采信请求位点=%d  命中端点=%d  命中路径(去方法)=%d" % (
        len(res["files"]), total_sites, len(sites), len(hit_paths)))
    w("三档：A 零覆盖=%d  B 浅覆盖=%d  C 深覆盖=%d  （A+B+C=%d）" % (len(A), len(B), len(C), len(keys)))
    w("不可复现风险计数：解析不出路径的位点=%d  e2e 字面量对不上快照=%d  方法不符=%d  "
      "矩阵条目缺方法=%d" % (
        len(res["unparsed"]), len(res["ghost"]), sum(len(v) for v in res["method_mismatch"].values()),
        len(res["registry_no_method"])))

    w("")
    w("== 覆盖分母豁免（#148 终裁登记；已从 A 档剔除 %d 条，不算 A 档残量，禁止重复派工）==" % len(exempt_hits))
    for k in sorted(exempt_hits):
        r = res["snap_keys"][k]
        w("    %-6s %-58s handler=%s" % (r["method"], r["path"], r["handler"][:40]))
        w("           豁免声明：%s" % COVERAGE_EXEMPT[k])
    for k in sorted(stale_exempt):
        w("    !! 豁免过期（当轮未命中 A 档，规则需人工复核）：%s" % k)
    for k in sorted(gone_exempt):
        w("    !! 豁免键已不在快照（规则失效，待清理或重新登记）：%s" % k)

    w("")
    w("== 模块缺口榜（按 A 条数降序；总数=该模块 (METHOD,PATH) 端点数）==")
    order, board = gap_board(res, limit)
    w("%-26s %6s %6s" % ("module", "A", "total"))
    for mod, n, tot, _ in board:
        w("%-26s %6d %6d" % (mod, n, tot))

    w("")
    w("== 各模块 A 档明细（前 %d 条，超出部分只计榜上数字）==" % limit)
    for mod, n, tot, keys_list in board:
        w("--- %s  A=%d/%d" % (mod, n, tot))
        for k in keys_list:
            r = res["snap_keys"][k]
            w("    %-6s %-58s %s" % (r["method"], r["path"], r["handler"][:52]))

    w("")
    w("== 对照榜：按快照第 4 列 routes_file（后端路由文件口径）聚 A 档，取前 25 名 ==")
    _, board2 = gap_board(res, 0, by_routes_file=True)
    for mod, n, tot, _ in board2[:25]:
        w("    %-26s A=%-5d 该文件端点=%d" % (mod, n, tot))

    w("")
    w("== B 档（浅覆盖，逐条可点验；前 %d 条）==" % limit)
    for k in sorted(B)[:limit]:
        r = res["snap_keys"][k]
        st = sites[k][0]
        w("    %-6s %-56s %s:%d %s" % (r["method"], r["path"], st["file"], st["line"], st["reason"]))

    w("")
    w("== 未解析位点（前 %d 条，全量 %d 条）==" % (limit, len(res["unparsed"])))
    for u in res["unparsed"][:limit]:
        w("    %s:%d  [%s]  %s  <- %s" % (u["file"], u["line"], u["how"], u["why"], u["expr"]))

    w("")
    w("== e2e 字面量对不上快照（前 %d 条，全量 %d 条）==" % (limit, len(res["ghost"])))
    seen = set()
    shown = 0
    for g in sorted(res["ghost"], key=lambda x: (x["kind"], x["path"], x["file"], x["line"])):
        sig = (g["kind"], g["method"], g["path"])
        if sig in seen:
            continue
        seen.add(sig)
        shown += 1
        if shown > limit:
            break
        w("    %-20s %-6s %-56s %s:%d" % (g["kind"], g["method"], g["path"], g["file"], g["line"]))

    w("")
    w("== 端点矩阵条目缺方法（数组名不在派生表；全量 %d 条）==" % len(res["registry_no_method"]))
    for g in sorted(res["registry_no_method"], key=lambda x: (x["file"], x["line"]))[:limit]:
        w("    %-30s %-56s %s:%d" % (g["array"], g["path"], g["file"], g["line"]))
    if res["registry_no_method"]:
        w("    以上条目本轮不派生缺省方法也不静默丢弃：要么给该数组补消费点实证后入派生表，"
          "要么在条目里显式写 method，二者都不许把它当已覆盖。")

    w("")
    w("== 方法不符（路径在册、该方法未注册；前 %d 条）==" % limit)
    for k in sorted(res["method_mismatch"])[:limit]:
        st = res["method_mismatch"][k][0]
        pth = k.split(" ", 1)[1]
        w("    %-56s e2e=%s 在册=%s  %s:%d" % (
            pth, k.split()[0], ",".join(sorted(res["snap_paths"].get(pth, []))),
            st["file"], st["line"]))

    w("")
    w("== 口径备查：A 档按形态分桶（未从分母剔除，只供判读；桶之间可重叠）==")
    for label, needles in OUT_OF_SCOPE_BUCKETS:
        sel = [k for k in A if any(s in res["snap_keys"][k]["path"].lower() for s in needles)]
        tot = [r for r in rows if any(s in r["path"].lower() for s in needles)]
        w("    %-42s A 中 %d 条 / 该形态共 %d 条" % (label, len(sel), len(tot)))

    if show_sites:
        w("")
        w("== 全部采信位点 ==")
        for k in sorted(sites):
            for st in sorted(sites[k], key=lambda s: (s["file"], s["line"])):
                w("    %s  %s:%d  %s  %s  %s" % (k, st["file"], st["line"], st["how"],
                                                 st["strength"], st["reason"]))

    text = "\n".join(out) + "\n"
    stream = getattr(sys.stdout, "buffer", None)
    if stream is not None:
        stream.write(text.encode("utf-8"))
        stream.flush()
    else:
        sys.stdout.write(text)


if __name__ == "__main__":
    main()
