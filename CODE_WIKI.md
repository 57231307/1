# 秉羲管理系统 (BingXi ERP) - 核心模块逐行级源码深度剖析

本文档对秉羲管理系统中最核心、最具行业代表性的功能模块进行了**逐行级别**的源码剖析。涵盖了系统启动、身份认证、RBAC权限控制、面料五维度管理、高并发防超卖事务以及前端通信基座。

---

## 1. 系统启动与兜底机制 (`backend/src/main.rs`)

`main.rs` 是后端服务的入口，负责初始化所有基础设施。它实现了一个极具企业级考量的特性：**数据库连接失败降级机制**。

### 1.1 核心代码片段
```rust
117 #[tokio::main]
118 async fn main() -> Result<(), Box<dyn std::error::Error>> {
119     let settings = AppSettings::new()?; // 1. 加载配置
...
181     // 2. 尝试连接数据库
182     let db_result = Database::connect(&settings.database.connection_string).await;
183 
184     let app = match db_result {
185         Ok(db) => {
...             // 3A. 数据库连接成功，启动完整业务路由
190             let app_state = crate::utils::app_state::AppState::new(Arc::new(db), settings.auth.jwt_secret.clone());
191             let app_state_clone = app_state.clone();
192             create_router(app_state)
...                 // 挂载全局安全中间件
221                 .layer(SetResponseHeaderLayer::overriding(axum::http::header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY")))
...
249         }
250         Err(e) => {
...             // 3B. 数据库连接失败，启动降级路由
254             create_init_router()
...                 .layer(cors.clone())
304         }
305     };
...
314     axum::serve(tokio::net::TcpListener::bind(http_addr).await?, app).await?;
315     Ok(())
316 }
```

### 1.2 逐行/块级分析
- **L117-118**: 使用 `#[tokio::main]` 宏将异步 main 函数包装为同步运行时，返回标准的 Error。
- **L181-182**: 调用 SeaORM 的 `Database::connect`，这里是启动的关键分水岭。
- **L184-249 (Ok分支)**: 
  - 如果数据库连接成功，实例化 `AppState` (包含 `Arc<Database>` 和 JWT Secret)。
  - 调用 `create_router(app_state)` 生成包含所有业务接口的完整 Router。
  - 随后链式挂载大量中间件：包括 `TraceLayer` (打点日志)、`CorsLayer` (跨域)、`auth_middleware` (认证)、`permission_middleware` (鉴权)，以及防点击劫持的 `X-Frame-Options` 等一整套安全 Header。
- **L250-304 (Err分支)**:
  - 核心亮点：**系统没有 panic 退出**。如果数据库未准备好，系统调用 `create_init_router()`。该路由仅提供 `/init/*` 接口，允许管理员通过前端页面重新配置数据库账号密码并一键建表（初始化引导）。
- **L314**: 绑定 TCP 端口并启动 Axum 服务器。

---

## 2. 安全与认证体系 (`backend/src/services/auth_service.rs`)

秉羲 ERP 采用了目前业界最推荐的 `Argon2id` 算法进行密码哈希，并使用 `jsonwebtoken` 进行状态无状态会话管理。

### 2.1 密码哈希逻辑
```rust
118     pub fn hash_password(password: &str) -> Result<String, AuthError> {
119         let salt = SaltString::generate(&mut OsRng);
120         // 使用更安全的Argon2参数配置
121         let argon2 = Argon2::new(
122             argon2::Algorithm::Argon2id,
123             argon2::Version::V0x13,
124             argon2::Params::new(19456, 2, 1, None).unwrap(),
125         );
126         let hash = argon2
127             .hash_password(password.as_bytes(), &salt)
128             .map_err(|_| AuthError::HashError)?;
129 
130         Ok(hash.to_string())
131     }
```
### 2.2 逐行分析
- **L119**: 生成高强度随机盐 (`OsRng`)，防止彩虹表攻击。
- **L121-125**: 实例化 Argon2 对象。
  - `Argon2id`: 结合了 Argon2d（抗GPU破解）和 Argon2i（抗侧信道攻击）的优点。
  - `Params::new(19456, 2, 1, None)`: `19456` KB 内存开销 (约19MB)，`2` 次迭代，`1` 个并行线程。这是经过调整的参数，能在保证高安全性的同时，适配服务器内存较小的部署环境。
- **L126-128**: 对明文密码加盐哈希，失败则抛出自定义枚举错误 `AuthError::HashError`。

---

## 3. 动态 RBAC 权限拦截 (`backend/src/middleware/permission.rs`)

该中间件基于请求的 URL 路径和 HTTP Method，动态计算资源访问权限。

### 3.1 核心鉴权逻辑
```rust
14 pub async fn permission_middleware(
15     State(state): State<AppState>,
16     request: Request<Body>,
17     next: Next,
18 ) -> Result<Response, StatusCode> {
...
23     let public_paths = ["/health", "/api/v1/erp/auth/login", ...];
38     if public_paths.iter().any(|p| path.starts_with(p)) {
39         return Ok(next.run(request).await);
40     }
...
42     let auth = request.extensions().get::<AuthContext>().cloned();
...
51     if auth.user_id == 1 {
52         return Ok(next.run(request).await); // 超管放行
53     }
54 
55     let (resource_type, resource_id) = extract_resource_info(path);
56     let has_permission = check_permission(
...
62         &method_to_action(method),
63     ).await;
...
66     if has_permission {
67         Ok(next.run(request).await)
68     } else {
70         Err(StatusCode::FORBIDDEN)
71     }
72 }
```
### 3.2 逐行分析
- **L23-40**: 定义白名单路由（如登录、健康检查），匹配则直接 `next.run()` 放行。
- **L42**: 从 Axum 请求的 `Extensions` 提取 `AuthContext`（该上下文由上游的 `auth_middleware` 解析 JWT 后注入）。
- **L51-53**: 硬编码后门：`user_id == 1` 为超级管理员，拥有上帝权限，直接放行。
- **L55**: `extract_resource_info` 是核心解析器。它将类似 `/api/v1/erp/sales/orders/12` 的路径，拆解提取出 `resource_type = "sales"` 和 `resource_id = 12`。
- **L56-63**: 数据库鉴权比对。`method_to_action` 将 HTTP 动词转为动作 (GET->read, POST->create)。`check_permission` 会在数据库的 `role_permissions` 表中查询：当前 `role_id` 是否允许对 `resource_type` 执行对应 `action`，并根据结果返回 `403 FORBIDDEN` 或放行。

---

## 4. 行业特色：面料五维度管理 (`backend/src/utils/fabric_five_dimension.rs`)

面料行业的痛点在于库存不仅看产品，还要看批次、色号、缸号。系统抽象了 `FabricFiveDimension` 结构体。

### 4.1 唯一身份标识生成
```rust
48     pub fn generate_unique_id(&self) -> String {
49         let dye_lot = self.dye_lot_no.as_deref().unwrap_or("N");
50         format!(
51             "P{}|B{}|C{}|D{}|G{}",
52             self.product_id, self.batch_no, self.color_no, dye_lot, self.grade
53         )
54     }
```
### 4.2 逐行分析
- **L48-49**: 如果 `dye_lot_no` (缸号) 是 `None`，则降级为 `"N"`。
- **L50-53**: 格式化宏拼接。生成如 `P123|B2405|C001|D01|G一等品` 的字符串。这个 ID 在 `business_trace_chain` (追溯链) 表中被用作核心外键，将分散在采购、库存、销售表中的孤立数据，强制串联为一根连续的生命线。

---

## 5. 防超卖与复杂事务：销售订单创建 (`backend/src/services/sales_service.rs`)

创建订单涉及主表、明细表、库存表等多个表的写操作，必须保证 ACID 特性。

### 5.1 事务与库存锁定代码
```rust
297     pub async fn create_order(&self, request: CreateSalesOrderRequest) -> Result<SalesOrderDetail, sea_orm::DbErr> {
302         let txn = (*self.db).begin().await?; // 1. 开启事务
303 
305         let order_no = self.generate_order_no().await?;
308         let existing_order = SalesOrderEntity::find().filter(sales_order::Column::OrderNo.eq(&order_no)).one(&txn).await?;
...
320         let order = sales_order::ActiveModel { ... };
345         let order_entity = order.insert(&txn).await?;
346 
347         // 2. 检查库存是否充足
348         self.check_inventory(&request.items, &txn).await?;
...
427         // 3. 更新订单总金额
430         let mut order_update: sales_order::ActiveModel = order_entity.into();
...
439         // 4. 提交事务
440         txn.commit().await?;
...
```
### 5.2 逐行分析
- **L302**: `(*self.db).begin().await?` 调用底层 SQLx 创建数据库事务。所有的后续数据库操作都必须传入 `&txn`，否则会引发死锁或数据不一致。
- **L305-317**: `generate_order_no` 结合当前日期生成单号，同时紧接着在事务内加锁查询 `existing_order`，如果存在则说明并发碰撞，主动 `txn.rollback()`，这是防重单的第一道防线。
- **L320-345**: 构建 `ActiveModel` 并执行 `insert(&txn)`，插入订单主表。
- **L348**: `check_inventory` 是防超卖的核心。它遍历订单明细，并在事务内查询 `InventoryStockEntity` 的 `quantity_available` (可用库存)。如果不满足购买量，函数抛出错误，外层使用 `?` 语法糖会直接中断并导致事务被丢弃（隐式回滚）。
- **L427-439**: 经过一系列税费计算并插入 `sales_order_items` 后，更新主表的最终总价，并调用 `txn.commit()` 落盘。

---

## 6. 前端通信基座与指数退避重试 (`frontend/src/services/api.rs`)

在 WebAssembly (Yew) 侧，网络不稳定是常态，项目实现了一个带有**指数退避（Exponential Backoff）**机制的 `ApiService`。

### 6.1 请求重试封装
```rust
52     async fn request_with_retry<T: DeserializeOwned>(
53         method: &str,
54         url: &str,
55         body: Option<&serde_json::Value>,
56     ) -> Result<T, String> {
...
60         for attempt in 0..Self::MAX_RETRIES {
61             match Self::do_request(method, &full_url, body).await {
62                 Ok(response) => {
63                     match response.json::<ApiResponse<T>>().await {
64                         Ok(api_response) => {
65                             if api_response.success {
66                                 return Ok(api_response.data.unwrap());
...
84                 Err(e) => {
85                     last_error = Some(e.clone());
86                     if attempt < Self::MAX_RETRIES - 1 {
87                         let delay_ms = 1000 * 2u64.pow(attempt);
88                         gloo_timers::future::TimeoutFuture::new(delay_ms as u32).await;
89                     }
90                 }
91             }
92         }
95         Err(last_error.unwrap_or_else(|| "未知错误".to_string()))
96     }
```
### 6.2 逐行分析
- **L52-56**: 泛型方法定义。`T: DeserializeOwned` 约束了返回值必须能够被 Serde 反序列化。
- **L60-61**: 最多重试 `MAX_RETRIES` (默认 3) 次。`do_request` 会从本地存储读取 JWT Token 并组装 Fetch 请求。
- **L63-66**: 将后端的原生响应反序列化为统一包装体 `ApiResponse<T>`。如果 `api_response.success` 为 `true`，直接解包出里面的 `data` 并返回给调用方，极大简化了业务页面的判断逻辑。
- **L84-90**: **高阶实现**。如果底层请求报错（如断网），记录错误。如果还有重试机会，执行 `1000 * 2^attempt` 毫秒的退避延迟（第0次失败等1秒，第1次等2秒）。使用 `TimeoutFuture::new().await` 非阻塞挂起 WASM 线程，这是提升前端应用鲁棒性的经典范式。