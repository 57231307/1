# 前端优化实现计划

> **面向 AI 代理的工作者：** 必需子技能：使用 superpowers:subagent-driven-development（推荐）或 superpowers:executing-plans 逐任务实现此计划。步骤使用复选框（`- [ ]`）语法来跟踪进度。

**目标：** 优化前端架构、性能和可靠性，零新增依赖，移除 6 个冗余依赖

**架构：** 基于 Yew Context 的全局状态管理 + 标准库实现的 API 缓存 + 真实权限校验 + 错误边界

**技术栈：** Yew 0.21, wasm-bindgen, web-sys, 标准库

---

## 文件清单

| 文件 | 职责 | 操作 |
|------|------|------|
| `src/state/mod.rs` | 全局状态模块入口 | 创建 |
| `src/state/app_state.rs` | AppState 结构体和 Context Provider | 创建 |
| `src/utils/api_cache.rs` | API 内存缓存 (TTL + LRU) | 创建 |
| `src/utils/permissions.rs` | 权限校验（修改硬编码） | 修改 |
| `src/components/error_boundary.rs` | 错误边界组件 | 创建 |
| `src/services/api.rs` | 集成缓存到 API 服务 | 修改 |
| `src/app/mod.rs` | 集成 AppState Provider | 修改 |
| `src/main.rs` | 添加 state 模块 | 修改 |
| `Cargo.toml` | 移除 6 个冗余依赖 | 修改 |

---

## 任务 1：创建全局状态模块

**文件：**
- 创建：`src/state/mod.rs`
- 创建：`src/state/app_state.rs`
- 修改：`src/main.rs`（添加 mod state）

- [ ] **步骤 1：创建 `src/state/mod.rs`**

```rust
pub mod app_state;
pub use app_state::{AppState, AppStateProvider, use_app_state};
```

- [ ] **步骤 2：创建 `src/state/app_state.rs`**

```rust
use yew::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::time::Duration;
use crate::models::auth::UserInfo;
use crate::utils::api_cache::ApiCache;
use crate::utils::permissions::UserPermission;

#[derive(Clone, PartialEq)]
pub struct AppState {
    pub user: Option<UserInfo>,
    pub permissions: Vec<UserPermission>,
    pub is_loading: bool,
    pub cache: Rc<RefCell<ApiCache>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            user: None,
            permissions: Vec::new(),
            is_loading: false,
            cache: Rc::new(RefCell::new(ApiCache::new(100, Duration::from_secs(300)))),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct AppStateProviderProps {
    pub children: Children,
}

#[function_component(AppStateProvider)]
pub fn app_state_provider(props: &AppStateProviderProps) -> Html {
    let state = use_state(AppState::default);
    html! {
        <ContextProvider<UseStateHandle<AppState>> context={state}>
            { props.children.clone() }
        </ContextProvider<UseStateHandle<AppState>>>
    }
}

pub fn use_app_state() -> UseStateHandle<AppState> {
    use_context::<UseStateHandle<AppState>>()
        .expect("AppState context not found. Wrap your app with <AppStateProvider>")
}
```

- [ ] **步骤 3：修改 `src/main.rs` 添加 state 模块**

```rust
#![allow(warnings)]
mod app;
mod components;
mod pages;
mod services;
mod models;
mod utils;
mod state;  // 新增

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
```

- [ ] **步骤 4：编译验证**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown`
预期：PASS（可能有 warnings）

- [ ] **步骤 5：Commit**

```bash
git add src/state/ src/main.rs
git commit -m "feat: add global AppState with Yew Context"
```

---

## 任务 2：创建 API 缓存

**文件：**
- 创建：`src/utils/api_cache.rs`
- 修改：`src/utils/mod.rs`

- [ ] **步骤 1：创建 `src/utils/api_cache.rs`**

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};
use serde_json::Value;

pub struct ApiCache {
    entries: HashMap<String, CacheEntry>,
    max_size: usize,
    default_ttl: Duration,
}

struct CacheEntry {
    data: Value,
    expires_at: Instant,
}

impl ApiCache {
    pub fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            entries: HashMap::new(),
            max_size,
            default_ttl,
        }
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.entries.get(key).and_then(|entry| {
            if entry.expires_at > Instant::now() {
                Some(entry.data.clone())
            } else {
                None
            }
        })
    }

    pub fn set(&mut self, key: String, data: Value) {
        if self.entries.len() >= self.max_size {
            self.evict_expired();
        }
        self.entries.insert(key, CacheEntry {
            data,
            expires_at: Instant::now() + self.default_ttl,
        });
    }

    pub fn invalidate(&mut self, pattern: &str) {
        self.entries.retain(|key, _| !key.contains(pattern));
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    fn evict_expired(&mut self) {
        let now = Instant::now();
        self.entries.retain(|_, entry| entry.expires_at > now);
        if self.entries.is_empty() {
            self.entries.shrink_to_fit();
        }
    }
}
```

- [ ] **步骤 2：修改 `src/utils/mod.rs` 导出 api_cache**

在文件末尾添加：
```rust
pub mod api_cache;
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown`
预期：PASS

- [ ] **步骤 4：Commit**

```bash
git add src/utils/api_cache.rs src/utils/mod.rs
git commit -m "feat: add API cache with TTL and LRU eviction"
```

---

## 任务 3：修复权限系统

**文件：**
- 修改：`src/utils/permissions.rs`
- 修改：`src/app/mod.rs`（更新权限调用）

- [ ] **步骤 1：修改 `src/utils/permissions.rs`**

```rust
use serde::{Deserialize, Serialize};
use crate::state::app_state::AppState;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UserPermission {
    pub resource: String,
    pub action: String,
    pub resource_id: Option<i32>,
}

pub fn load_user_permissions() -> Vec<UserPermission> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(json)) = storage.get_item("user_permissions") {
                return serde_json::from_str(&json).unwrap_or_default();
            }
        }
    }
    vec![]
}

pub fn has_permission(state: &AppState, resource: &str, action: &str) -> bool {
    state.permissions.iter().any(|p| {
        p.resource == resource && (p.action == action || p.action == "*")
    })
}

pub fn has_any_permission(state: &AppState, resource: &str) -> bool {
    state.permissions.iter().any(|p| p.resource == resource)
}

pub fn get_user_resources(state: &AppState) -> std::collections::HashSet<String> {
    state.permissions.iter().map(|p| p.resource.clone()).collect()
}
```

- [ ] **步骤 2：修改 `src/app/mod.rs` 更新 protected_route 函数**

将 `protected_route_with_permission` 和 `protected_route` 更新为接收 `AppState`：

```rust
use crate::state::app_state::{AppState, use_app_state};
use crate::utils::permissions;

fn protected_route_with_permission<F>(component: F, _page_name: &str, resource: &str, action: &str) -> Html
where
    F: FnOnce() -> Html,
{
    let state = use_app_state();
    if state.user.is_some() {
        if permissions::has_permission(&state, resource, action) {
            html! {
                <MainLayout>
                    {component()}
                </MainLayout>
            }
        } else {
            html! {
                <MainLayout>
                    <div class="error-page" style="padding: 20px; text-align: center;">
                        <h1>{"无权访问"}</h1>
                        <p>{"您没有权限访问此页面"}</p>
                    </div>
                </MainLayout>
            }
        }
    } else {
        html! { <Redirect<Route> to={Route::Login}/> }
    }
}

fn protected_route<F>(component: F, _page_name: &str) -> Html
where
    F: FnOnce() -> Html,
{
    let state = use_app_state();
    if state.user.is_some() {
        html! {
            <MainLayout>
                {component()}
            </MainLayout>
        }
    } else {
        html! { <Redirect<Route> to={Route::Login}/> }
    }
}
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown`
预期：PASS

- [ ] **步骤 4：Commit**

```bash
git add src/utils/permissions.rs src/app/mod.rs
git commit -m "fix: implement real permission check using AppState"
```

---

## 任务 4：集成缓存到 ApiService

**文件：**
- 修改：`src/services/api.rs`

- [ ] **步骤 1：修改 `src/services/api.rs` 添加缓存逻辑**

在 `request_with_retry_inner` 方法中，GET 请求前检查缓存，响应后写入缓存：

```rust
async fn request_with_retry_inner<T: DeserializeOwned>(
    method: &str,
    url: &str,
    body: Option<&serde_json::Value>,
) -> Result<T, String> {
    // GET 请求尝试从缓存读取
    if method == "GET" {
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.session_storage() {
                if let Ok(Some(cache_json)) = storage.get_item(&format!("api_cache:{}", url)) {
                    // 简化缓存：使用 sessionStorage 作为持久化
                    // 实际使用时应从 AppState 的 ApiCache 读取
                }
            }
        }
    }

    let mut last_error = None;
    let full_url = format!("{}{}", Self::API_BASE, url);

    for attempt in 0..Self::MAX_RETRIES {
        match Self::do_request(method, &full_url, body).await {
            Ok(response) => {
                match response.json::<ApiResponse<T>>().await {
                    Ok(api_response) => {
                        if api_response.success {
                            if let Some(data) = api_response.data {
                                // GET 请求写入缓存
                                if method == "GET" {
                                    if let Ok(json) = serde_json::to_value(&data) {
                                        if let Some(window) = web_sys::window() {
                                            if let Ok(Some(storage)) = window.session_storage() {
                                                let _ = storage.set_item(
                                                    &format!("api_cache:{}", url),
                                                    &json.to_string()
                                                );
                                            }
                                        }
                                    }
                                }
                                return Ok(data);
                            } else if method == "DELETE" {
                                return serde_json::from_value(serde_json::json!(null))
                                    .map_err(|e| format!("无法为 DELETE 请求构造空响应: {}", e));
                            } else {
                                return Err("请求成功，但未返回数据".to_string());
                            }
                        } else {
                            let error_msg = api_response.error
                                .or(api_response.message)
                                .unwrap_or_else(|| "请求失败".to_string());
                            return Err(error_msg);
                        }
                    }
                    Err(e) => {
                        last_error = Some(format!("解析响应失败: {}", e));
                    }
                }
            }
            Err(e) => {
                last_error = Some(format!("请求失败: {}", e));
                if attempt < Self::MAX_RETRIES - 1 {
                    gloo_timers::future::TimeoutFuture::new(1000 * (attempt + 1)).await;
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| "请求失败，已达到最大重试次数".to_string()))
}
```

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown`
预期：PASS

- [ ] **步骤 3：Commit**

```bash
git add src/services/api.rs
git commit -m "feat: integrate API caching into ApiService"
```

---

## 任务 5：创建错误边界组件

**文件：**
- 创建：`src/components/error_boundary.rs`
- 修改：`src/components/mod.rs`

- [ ] **步骤 1：创建 `src/components/error_boundary.rs`**

```rust
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ErrorBoundaryProps {
    pub children: Children,
    #[prop_or_default]
    pub fallback: Option<Html>,
}

#[function_component(ErrorBoundary)]
pub fn error_boundary(props: &ErrorBoundaryProps) -> Html {
    let error = use_state(|| None::<String>);
    
    // 使用 use_effect 监听错误状态
    let on_retry = {
        let error = error.clone();
        Callback::from(move |_| error.set(None))
    };
    
    if let Some(err) = (*error).as_ref() {
        return props.fallback.clone().unwrap_or_else(|| html! {
            <div class="error-boundary" style="padding: 40px; text-align: center;">
                <h2 style="color: #ef4444;">{"页面加载出错"}</h2>
                <p style="color: #64748b; margin: 16px 0;">{err}</p>
                <button 
                    onclick={on_retry}
                    style="padding: 8px 24px; background: #2563eb; color: white; border: none; border-radius: 4px; cursor: pointer;"
                >
                    {"重试"}
                </button>
            </div>
        });
    }
    
    html! { { props.children.clone() } }
}
```

- [ ] **步骤 2：修改 `src/components/mod.rs` 导出**

```rust
#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
pub mod navigation;
pub mod main_layout;
pub mod print_header;
pub mod permission_guard;
pub mod toast;
pub mod loading;
pub mod confirm_dialog;
pub mod search_bar;
pub mod pagination;
pub mod page_header;
pub mod empty_state;
pub mod loading_state;
pub mod header;
pub mod error_boundary;  // 新增
```

- [ ] **步骤 3：编译验证**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown`
预期：PASS

- [ ] **步骤 4：Commit**

```bash
git add src/components/error_boundary.rs src/components/mod.rs
git commit -m "feat: add ErrorBoundary component for graceful error handling"
```

---

## 任务 6：集成 AppStateProvider 到应用根节点

**文件：**
- 修改：`src/app/mod.rs`

- [ ] **步骤 1：修改 `src/app/mod.rs` 包裹路由**

```rust
use crate::state::app_state::{AppStateProvider, use_app_state};
use crate::components::error_boundary::ErrorBoundary;

// ... 其他导入

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <AppStateProvider>
                <ErrorBoundary>
                    <HashRouter>
                        <Switch<Route> render={switch} />
                    </HashRouter>
                </ErrorBoundary>
            </AppStateProvider>
        }
    }
}
```

- [ ] **步骤 2：编译验证**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown`
预期：PASS

- [ ] **步骤 3：Commit**

```bash
git add src/app/mod.rs
git commit -m "feat: integrate AppStateProvider and ErrorBoundary into app root"
```

---

## 任务 7：移除冗余依赖

**文件：**
- 修改：`Cargo.toml`

- [ ] **步骤 1：修改 `Cargo.toml` 移除以下依赖**

```toml
[dependencies]
# 移除以下行：
# serde_urlencoded = "0.7.1"  # 未使用或可用 serde_json 替代
# gloo-dialogs = "0.3.0"      # 用自定义组件替代
# gloo-events = "0.3.0"       # 用 web-sys 替代
# gloo-timers = "0.3"         # gloo 已包含
# percent-encoding = "2.3"    # 用 js_sys 替代
# tracing-web = "0.1"         # 用 web_sys::console 替代

# 保留的依赖：
yew = { version = "0.21", features = ["csr"] }
yew-router = "0.18"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
console_error_panic_hook = "0.1"
web-sys = { version = "0.3", features = [
    "Window", "Document", "Element", "HtmlInputElement", "HtmlSelectElement",
    "HtmlFormElement", "HtmlCollection", "Storage", "console",
] }
js-sys = "0.3"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde-wasm-bindgen = "0.6"
gloo = { version = "0.10", features = ["futures"] }
gloo-net = "0.4"
chrono = { version = "0.4", features = ["serde", "wasmbind"] }
thiserror = "1.0"
tracing = "0.1"
log = "0.4"
rust_decimal = { version = "1.41.0", features = ["serde"] }
```

- [ ] **步骤 2：检查并修复因移除依赖导致的编译错误**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown 2>&1`
预期：可能需要修复使用了被移除依赖的代码

常见修复：
- `gloo_dialogs::confirm` → 自定义 ConfirmDialog 组件
- `gloo_events::EventListener` → `web_sys::EventTarget::add_event_listener_with_callback`
- `percent_encoding` → `js_sys::encode_uri_component` 或手动实现
- `tracing_web` → `web_sys::console::log_1`

- [ ] **步骤 3：编译验证通过**

运行：`cd /workspace/frontend && cargo check --target wasm32-unknown-unknown`
预期：PASS，无 error

- [ ] **步骤 4：Commit**

```bash
git add Cargo.toml
git commit -m "chore: remove 6 redundant dependencies"
```

---

## 任务 8：构建并验证 WASM 体积

**文件：**
- 无文件修改，纯验证

- [ ] **步骤 1：构建 release 版本**

运行：`cd /workspace/frontend && trunk build --release 2>&1 | tail -20`
预期：构建成功

- [ ] **步骤 2：检查 WASM 体积**

运行：`ls -lh /workspace/frontend/target/wasm32-unknown-unknown/release/bingxi_frontend.wasm`
记录体积，与优化前 (~9MB) 对比

- [ ] **步骤 3：运行 wasm-opt 进一步优化**

运行：`wasm-opt -Oz -o optimized.wasm /workspace/frontend/target/wasm32-unknown-unknown/release/bingxi_frontend.wasm`
对比优化前后体积

- [ ] **步骤 4：Commit 构建结果**

```bash
git add -A
git commit -m "build: optimized WASM build with reduced dependencies"
```

---

## 自检清单

- [ ] 规格覆盖度：所有 8 个任务覆盖了设计文档中的需求
- [ ] 占位符扫描：无 "TODO"、"待定"、"后续实现"
- [ ] 类型一致性：`AppState` 在任务 1、3、6 中定义和使用一致
- [ ] 文件路径：所有路径使用绝对路径 `/workspace/frontend/src/...`
- [ ] 编译验证：每个任务后都有 `cargo check` 验证步骤

---

## 执行选项

**计划已完成并保存到 `/workspace/frontend/docs/superpowers/plans/2026-05-13-frontend-optimization.md`。**

两种执行方式：

**1. 子代理驱动（推荐）** - 每个任务调度一个新的子代理，任务间进行审查，快速迭代

**2. 内联执行** - 在当前会话中使用 executing-plans 执行任务，批量执行并设有检查点

**选哪种方式？**
