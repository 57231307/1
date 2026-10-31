# Bingxi Management Platform 接口文档

本文件描述后端对外接口的实际形态。所有陈述均以主干代码为据，逐条给出出处，
并在文末给出可复现的统计口径与重算命令。文档不描述"计划中的接口"，只描述已注册的接口。

## 概述

对外接口只有一种传输方式，REST over HTTP，JSON 编码。

本项目没有 gRPC 服务，`backend/Cargo.toml` 不含 tonic 或 grpc 依赖，代码中也没有
服务定义与 stub。此前版本的本文件把 gRPC 列为第二种接口方式，与实现不符，已删。

业务接口的统一前缀是 `/api/v1/erp/`。在此之外还挂载三类端点：

| 前缀 | 用途 | 出处 |
|---|---|---|
| `/api/v1/erp/` | 全部业务接口 | `backend/src/routes/mod.rs` |
| `/metrics` | Prometheus 抓取 | `backend/src/routes/mod.rs` |
| `/swagger-ui` 与 `/api-docs/openapi.json` | 由 utoipa 生成的接口描述 | `backend/src/routes/mod.rs` |
| `/static/*` 与 `/bingxi_frontend.*` | 前端静态资源 | `backend/src/routes/mod.rs` |

`/api/v1` 下另有一个 `v1::routes()` 挂载位，`routes/mod.rs` 的路径规划注释称其为占位。

## 路由装配

装配入口是 `backend/src/routes/mod.rs` 的 `create_router`，它把
`backend/src/routes/` 下的各业务域子路由文件用 `nest` 与 `merge` 组合成最终 Router。
当前该目录有 44 个路由模块文件。

主要挂载前缀与对应模块，摘自 `routes/mod.rs` 的路径规划注释：

| 挂载前缀 | 模块 |
|---|---|
| `/api/v1/erp/auth` | `auth::routes()` |
| `/api/v1/erp` | `iam`、`catalog`、`analytics`、`system` 四域合并，共享同一前缀故必须用 merge |
| `/api/v1/erp/inventory` | `inventory::routes()` |
| `/api/v1/erp/sales` | `sales::routes()` |
| `/api/v1/erp/purchase` | `purchase::routes()` |
| `/api/v1/erp/finance` | `finance::routes(state)`，唯一需要显式传 state 的域 |
| `/api/v1/erp/production` | `production::routes()` |
| `/api/v1/erp/crm` | `crm::routes()` |

## 认证与凭据

凭据是 JWT，但传递方式与常见的 Bearer 用法相反：会话凭据由服务端下发为
httpOnly Cookie，浏览器随请求自动携带，前端不持有也不拼接令牌。

登录成功时 `backend/src/handlers/auth_handler.rs` 的 `build_login_cookies` 写两枚加密
httpOnly Cookie：

| Cookie | HttpOnly | 有效期 | 说明 |
|---|---|---|---|
| `access_token` | 是 | 8 小时 | 会话凭据，`SameSite=Strict` |
| `refresh_token` | 是 | 2 天 | 续期凭据，原为 7 天，已缩短 |

第三枚 `csrf_token` 由调用方用普通 `Set-Cookie` 明文下发，不经
`PrivateCookieJar` 加密，因为它需要被前端脚本读出来回填请求头。

Cookie 的 `Secure` 标志按请求实际协议判定，优先看 `X-Forwarded-Proto`，回退
`is_production`。原因是 HTTP 部署下 `secure(true)` 的 Cookie 会被浏览器拒绝存储，
表现为登录成功但会话建立不起来。

服务端接受凭据的优先顺序见 `backend/src/middleware/auth.rs`：

1. httpOnly Cookie `access_token`，当前唯一由登录与刷新流程写入的凭据位；
2. 旧版命名 Cookie `jwt`，登录与刷新已不再写入（`B03-P2-1` 移除双写），此处读取
   仅为过渡期清理旧客户端残留会话，旧 Cookie 自然过期后该分支不再命中；
3. `Authorization` 头，形如 `Bearer <token>`，供非浏览器客户端使用。

日志侧对第 3 项做脱敏：只记录格式与长度，例如 `Bearer abc***(len=143)`，
不落完整令牌。

刷新走 `POST /api/v1/erp/auth/refresh`。浏览器路径下不需要请求体携带
`refresh_token`，Cookie 自动随 `withCredentials` 发送，响应通过 `Set-Cookie`
更新 `access_token` 与 `csrf_token`。

## CSRF

对所有有副作用的方法，即 POST、PUT、PATCH、DELETE，强制要求请求头
`x-csrf-token`，常量定义在 `backend/src/middleware/csrf.rs` 的 `CSRF_HDR_NAME`。

无需携带该头的公开路径白名单维护在客户端一侧，见
`frontend/src/api/request.ts` 的 `CSRF_PUBLIC_PREFIXES`，前缀匹配，至少包含
`/auth/login`、`/auth/refresh`、`/auth/logout`，这些路径在后端 CSRF 中间件中已放行，
前端不再注入该头。公开路径上仍有一层 L-1 防御（`middleware/csrf.rs`）：
要求请求携带 `X-Requested-With` 或 `x-csrf-token` 任一自定义头，用来排除
跨站表单式简单请求。

令牌是一次性消费。多标签页或并发请求共用同一枚 Cookie 时，后发请求会携带已被
消费的旧令牌，此时服务端返回 403 且业务码为 `CSRF_TOKEN_MISSING` 或
`CSRF_TOKEN_INVALID`，并通过响应头 `x-new-csrf-token` 下发恢复令牌；客户端
据此重写 Cookie 后重放，而不是直接把用户踢出登录。

## 限流与熔断

`backend/src/middleware/rate_limit.rs` 提供速率限制，
`backend/src/middleware/circuit_breaker.rs` 提供熔断。
两者与 `security_headers`（6 个安全响应头，由 `main.rs` 统一挂载）以及
`sql_injection_audit_middleware`（挂在 Router 链最外层）共同构成请求前置链。

## 统一响应包络

成功响应形如：

```json
{ "code": 200, "message": "", "data": {} }
```

错误响应携带 `code`、`message`，并按错误族映射到相应 HTTP 状态码。

错误文案有一条硬性安全边界，定义在 `backend/src/utils/error.rs` 的模块文档：
出参的 `message` 默认是固定脱敏常量，真实文案只进 tracing 日志。只有构造点
显式使用 `AppError::business_displayable`、`AppError::validation_displayable`
或 `AppError::not_found_displayable` 时，响应才携带真实文案。不存在任何全局
开关或环境变量能改变这一行为，是否外显由每个构造点逐条声明。

`validator` 派生校验产生的文案一律以可外显形态返回，因为它只描述调用方
自己刚提交的字段。手工构造时同理：文案只涉及请求字段就可用可外显构造，
一旦包含第三方库原文、表名、SQL、堆栈或内部路径就必须走脱敏构造。

可外显文案的允许范围同样受限：不得包含权限判定依据、他人数据、内部标识
（表名、记录 ID、内部编码、文件路径）与任何系统实现细节。

## 数据范围与字段级权限

读侧行级数据范围由统一判定器 `check_resource_owner_by_member_scope`
（`backend/src/utils/data_scope.rs`）承担，语义为 All 放行、Dept 要求归属人属于
可见部门成员集合、Self 仅本人；归属列为 NULL 时 Dept 与 Self 一律拒绝，
不因脏数据留放行口子。列表接口按同一口径在构造处下推过滤，
使 `total` 与 `items` 同源，不出现列表比 total 少的错位。

字段级权限用于价格与成本类敏感列，读侧按角色掩码。读写必须配对：
读侧掩码掉的列，写侧不得无条件把它抄进更新载荷，否则非授权角色一次保存
就会把库里的真实值冲空。

## 前后端契约对照与可复现统计

接口数量随功能增长持续变化，本文件不固化端点清单，只固化统计口径与重算命令。
下列数字由 `frontend/scripts/check-api-paths.mjs` 从代码实测得出，
它从 `backend/src/routes/mod.rs` 的 `create_router` 出发递归展开
`nest`、`merge`、`route`，并把 `{param}`、`:param`、`${expr}` 归一为通配段，
再与前端 API 层的实际调用对照。

重算方式，在 `frontend` 目录执行：

```bash
node scripts/check-api-paths.mjs
```

主干最近一次实测结果：

| 指标 | 数量 |
|---|---|
| 后端 (path, method) 组合 | 1776 |
| 后端路径去重 | 1356 |
| 前端 (path, method) 组合 | 1146 |
| 后端存在但前端该路径零调用 | 588 |
| 前端调用而后端不存在，且已登记为功能缺口 | 3 |

三条已登记缺口逐条附原因，登记在 `check-api-paths.mjs` 的例外表内，不是静默豁免：

| 端点 | 原因 |
|---|---|
| `POST /api/v1/erp/currencies` | 前端 `createCurrency` 新增币种，后端 `currency_handler` 无 `create_currency`，币种主数据目前只读并只可设本位币 |
| `GET /api/v1/erp/customer-shares` | 客户共享清单读侧未提供 |
| `GET /api/v1/erp/system-update/backups/*` | 备份详情读侧未提供 |

同目录还有两支相关门禁，接口改动需一并满足：
`scripts/check-api-keys.mjs` 维护前端接口键基线（当前 108 枚），
`scripts/check-api-envelope.mjs` 校验响应包络形态。基线属生成物，
只能由对应脚本的写入模式重算，禁止手改。

## 修订说明

本次修订只更新本文件，改动项为：删除并不存在的 gRPC 接口描述；把认证一节
从"需要 JWT Token 认证"改写为按代码实测的 Cookie 优先凭据链与三档取值顺序；
补齐令牌有效期、CSRF 头名与一次性消费的恢复路径、错误文案的白名单式外显规则、
数据范围与字段级权限的读写配对要求；接口规模由旧版无从核对的表述改为
带重算命令的实测口径，并登记三条已知功能缺口。
