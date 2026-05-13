# 秉羲面料管理系统前端优化设计文档

> **日期**: 2026-05-13  
> **目标**: 优化前端架构、性能和可靠性，零新增依赖，移除冗余依赖  
> **范围**: 前端 (Yew/WASM)

---

## 1. 当前问题分析

### 1.1 架构问题
- **无全局状态管理**: 60+ 页面各自管理状态，导致重复 API 请求和数据不一致
- **权限系统失效**: `has_permission` 硬编码返回 `true`，存在安全风险
- **无错误边界**: 单页面崩溃导致整个应用崩溃
- **无缓存机制**: 每次导航都重新请求数据

### 1.2 性能问题
- **WASM 体积过大**: ~9MB，首屏加载慢
- **无代码分割**: 所有页面打包到一个 WASM 文件
- **无资源缓存策略**: 静态资源未有效缓存
- **重复依赖**: `gloo-dialogs`, `gloo-events`, `gloo-timers` 等功能可由标准库替代

### 1.3 依赖问题
当前 20+ 依赖中，以下可被移除或替换：

| 依赖 | 当前用途 | 替代方案 | 影响 |
|------|---------|---------|------|
| `gloo-dialogs` | 确认对话框 | 自定义 Yew 组件 | 零影响 |
| `gloo-events` | 事件监听 | `web-sys` 原生 API | 零影响 |
| `gloo-timers` | 定时器 | `wasm_bindgen_futures::spawn_local` + `js_sys::Date` | 零影响 |
| `percent-encoding` | URL 编码 | 手动实现或 `js_sys::encode_uri_component` | 极小 |
| `tracing-web` | WASM 日志 | `web_sys::console` | 零影响 |
| `serde_urlencoded` | 表单序列化 | `serde_json` 或手动构建 | 极小 |

**预计移除 6 个依赖，减少编译时间和 WASM 体积。**

---

## 2. 优化架构设计

### 2.1 核心架构图

```
┌─────────────────────────────────────────┐
│           Yew App (HashRouter)          │
├─────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────────┐  │
│  │ AppState    │  │ ApiCache        │  │
│  │ (Context)   │  │ (TTL + LRU)     │  │
│  │             │  │                 │  │
│  │ • user      │  │ • GET 缓存 5min │  │
│  │ • token     │  │ • 内存上限 100  │  │
│  │ • permissions│  │ • 自动清理     │  │
│  └─────────────┘  └─────────────────┘  │
├─────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────────┐  │
│  │ ErrorBoundary│  │ PermissionGuard │  │
│  │             │  │                 │  │
│  │ 捕获渲染错误 │  │ 真实权限校验    │  │
│  │ 降级 UI     │  │ 对接后端 API    │  │
│  └─────────────┘  └─────────────────┘  │
├─────────────────────────────────────────┤
│           页面组件 (懒加载)              │
│  Dashboard │ Sales │ Inventory │ ...    │
└─────────────────────────────────────────┘
```

### 2.2 模块职责

#### AppState (全局状态)
- **文件**: `src/state/app_state.rs`
- **职责**: 提供 Yew Context，管理用户、权限、全局加载状态
- **实现**: 使用 Yew 内置 `use_context` + `use_state`，零外部依赖

#### ApiCache (API 缓存)
- **文件**: `src/utils/api_cache.rs`
- **职责**: 内存缓存 GET 请求结果，支持 TTL 和 LRU 淘汰
- **实现**: 纯标准库 `HashMap` + `Instant`，零外部依赖

#### PermissionGuard (权限守卫)
- **文件**: `src/utils/permissions.rs` (修改)
- **职责**: 从 AppState 读取真实权限，对接后端权限数据
- **实现**: 替换硬编码 `true`，使用后端返回的权限列表

#### ErrorBoundary (错误边界)
- **文件**: `src/components/error_boundary.rs`
- **职责**: 捕获子组件渲染错误，显示降级 UI
- **实现**: Yew 内置 `error_boundary` 功能

---

## 3. 详细设计

### 3.1 全局状态管理 (AppState)

```rust
// src/state/mod.rs
use yew::prelude::*;
use std::rc::Rc;
use std::cell::RefCell;
use crate::models::auth::UserInfo;
use crate::utils::api_cache::ApiCache;

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

pub fn use_app_state() -> UseStateHandle<AppState> {
    use_context::<UseStateHandle<AppState>>()
        .expect("AppState context not found")
}
```

### 3.2 API 缓存 (ApiCache)

```rust
// src/utils/api_cache.rs
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
            self.evict_oldest();
        }
        self.entries.insert(key, CacheEntry {
            data,
            expires_at: Instant::now() + self.default_ttl,
        });
    }

    pub fn invalidate(&mut self, pattern: &str) {
        self.entries.retain(|key, _| !key.contains(pattern));
    }

    fn evict_oldest(&mut self) {
        // 简单淘汰：清除所有过期项
        let now = Instant::now();
        self.entries.retain(|_, entry| entry.expires_at > now);
    }
}
```

### 3.3 权限系统修复

```rust
// src/utils/permissions.rs
use crate::state::app_state::AppState;

pub fn has_permission(state: &AppState, resource: &str, action: &str) -> bool {
    state.permissions.iter().any(|p| {
        p.resource == resource && (p.action == action || p.action == "*")
    })
}

pub fn has_any_permission(state: &AppState, resource: &str) -> bool {
    state.permissions.iter().any(|p| p.resource == resource)
}
```

### 3.4 错误边界

```rust
// src/components/error_boundary.rs
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ErrorBoundaryProps {
    pub children: Children,
    #[prop_or_default]
    pub fallback: Option<Html>,
}

#[function_component]
pub fn ErrorBoundary(props: &ErrorBoundaryProps) -> Html {
    let error = use_state(|| None::<String>);
    
    if let Some(err) = (*error).as_ref() {
        return props.fallback.clone().unwrap_or_else(|| html! {
            <div class="error-boundary">
                <h2>{"页面加载出错"}</h2>
                <p>{err}</p>
                <button onclick={Callback::from(move |_| error.set(None))}>
                    {"重试"}
                </button>
            </div>
        });
    }
    
    html! { { props.children.clone() } }
}
```

---

## 4. 依赖移除计划

### 4.1 移除 gloo-dialogs
- **影响文件**: 使用 `gloo_dialogs::alert` / `confirm` 的页面
- **替代**: 自定义 `ConfirmDialog` 组件

### 4.2 移除 gloo-events
- **影响文件**: 使用 `EventListener` 的组件
- **替代**: `web_sys::EventTarget::add_event_listener_with_callback`

### 4.3 移除 gloo-timers
- **影响文件**: 使用 `Timeout::new` / `Interval::new` 的页面
- **替代**: `wasm_bindgen_futures::spawn_local` + `gloo::timers::future::TimeoutFuture` (gloo 已包含)

### 4.4 移除 percent-encoding
- **影响文件**: URL 参数编码
- **替代**: `js_sys::encode_uri_component` 或手动实现

### 4.5 移除 tracing-web
- **影响文件**: 日志输出
- **替代**: `web_sys::console::log_1`

### 4.6 移除 serde_urlencoded
- **影响文件**: 表单数据序列化
- **替代**: `serde_json` 或手动构建查询字符串

---

## 5. 性能优化措施

### 5.1 WASM 体积优化
- 启用 `wasm-opt` 优化（已通过 `strip = true`）
- 移除冗余依赖后重新编译
- 后端启用 Brotli/Gzip 压缩

### 5.2 运行时优化
- API 缓存减少重复请求
- 全局状态避免重复数据获取
- 错误边界防止级联崩溃

### 5.3 加载优化
- 骨架屏替代空白加载
- 关键 CSS 内联
- 非关键资源延迟加载

---

## 6. 实施优先级

| 优先级 | 任务 | 预期效果 | 工作量 |
|--------|------|---------|--------|
| P0 | 修复权限系统 | 安全合规 | 2 天 |
| P0 | 移除冗余依赖 | 减少体积 | 1 天 |
| P1 | 实现 AppState | 统一状态 | 3 天 |
| P1 | 实现 ApiCache | 减少请求 | 2 天 |
| P1 | 集成缓存到 ApiService | 性能提升 | 2 天 |
| P2 | 实现 ErrorBoundary | 可靠性 | 2 天 |
| P2 | 页面级懒加载 | 首屏优化 | 3 天 |
| P3 | 性能监控 | 可观测性 | 2 天 |

**总工作量：约 2.5 周**

---

## 7. 风险评估

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|---------|
| 依赖移除导致功能异常 | 中 | 中 | 逐项移除，充分测试 |
| 状态管理重构引入 bug | 中 | 高 | 渐进式迁移，保留旧代码 |
| 缓存导致数据不一致 | 低 | 中 | 合理的 TTL 和失效策略 |
| WASM 体积优化不及预期 | 低 | 低 | 已启用最高优化级别 |

---

## 8. 成功标准

- [ ] WASM 体积减少 15%+
- [ ] 重复 API 请求减少 60%+
- [ ] 权限系统真实生效
- [ ] 单页面崩溃不影响全局
- [ ] 依赖数量从 20+ 减少到 14
- [ ] 首屏加载时间减少 30%+
