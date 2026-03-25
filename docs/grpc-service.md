# gRPC 服务文档

## 概述

秉羲管理系统提供了完整的 gRPC 服务接口，用于高性能的 RPC 通信场景。gRPC 服务与 REST API 并行运行，提供更高效的通信方式。

## 服务地址

- **gRPC 服务地址**: `http://localhost:50051`
- **HTTP 服务地址**: `http://localhost:8000`

## 服务定义

### 1. 用户服务 (UserService)

用户服务提供了完整的用户管理功能。

#### 1.1 GetUser - 获取用户信息

获取指定用户的详细信息。

**请求参数**:
```protobuf
message GetUserRequest {
  int32 user_id = 1;  // 用户 ID
}
```

**响应参数**:
```protobuf
message GetUserResponse {
  bool success = 1;           // 是否成功
  string message = 2;         // 响应消息
  User user = 3;              // 用户信息
}
```

**使用示例**:
```rust
let request = tonic::Request::new(GetUserRequest {
    user_id: 1,
});

let response = user_client.get_user(request).await?;
let user = response.into_inner().user.unwrap();
println!("用户名：{}", user.username);
```

---

#### 1.2 CreateUser - 创建用户

创建新用户。

**请求参数**:
```protobuf
message CreateUserRequest {
  string username = 1;    // 用户名
  string password = 2;    // 密码
  string email = 3;       // 邮箱
  string phone = 4;       // 手机号
  string role = 5;        // 角色
}
```

**响应参数**:
```protobuf
message CreateUserResponse {
  bool success = 1;           // 是否成功
  string message = 2;         // 响应消息
  User user = 3;              // 创建的用户信息
}
```

**使用示例**:
```rust
let request = tonic::Request::new(CreateUserRequest {
    username: "new_user".to_string(),
    password: "password123".to_string(),
    email: "user@example.com".to_string(),
    phone: Some("13800138000".to_string()),
    role: "user".to_string(),
});

let response = user_client.create_user(request).await?;
```

---

#### 1.3 UpdateUser - 更新用户

更新用户信息。

**请求参数**:
```protobuf
message UpdateUserRequest {
  int32 user_id = 1;      // 用户 ID
  string email = 2;       // 邮箱
  string phone = 3;       // 手机号
  string role = 4;        // 角色
  string status = 5;      // 状态
}
```

**响应参数**:
```protobuf
message UpdateUserResponse {
  bool success = 1;           // 是否成功
  string message = 2;         // 响应消息
  User user = 3;              // 更新后的用户信息
}
```

**使用示例**:
```rust
let request = tonic::Request::new(UpdateUserRequest {
    user_id: 1,
    email: "new_email@example.com".to_string(),
    phone: None,
    role: Some("admin".to_string()),
    status: Some("active".to_string()),
});

let response = user_client.update_user(request).await?;
```

---

#### 1.4 DeleteUser - 删除用户

删除指定用户。

**请求参数**:
```protobuf
message DeleteUserRequest {
  int32 user_id = 1;  // 用户 ID
}
```

**响应参数**:
```protobuf
message DeleteUserResponse {
  bool success = 1;           // 是否成功
  string message = 2;         // 响应消息
}
```

**使用示例**:
```rust
let request = tonic::Request::new(DeleteUserRequest {
    user_id: 1,
});

let response = user_client.delete_user(request).await?;
```

---

#### 1.5 ListUsers - 列出用户

获取用户列表，支持分页和过滤。

**请求参数**:
```protobuf
message ListUsersRequest {
  int32 page = 1;           // 页码
  int32 page_size = 2;      // 每页数量
  string status = 3;        // 状态过滤
  string role = 4;          // 角色过滤
}
```

**响应参数**:
```protobuf
message ListUsersResponse {
  bool success = 1;           // 是否成功
  string message = 2;         // 响应消息
  repeated User users = 3;    // 用户列表
  int32 total = 4;            // 总数
  int32 page = 5;             // 当前页
  int32 page_size = 6;        // 每页数量
  int32 total_pages = 7;      // 总页数
}
```

**使用示例**:
```rust
let request = tonic::Request::new(ListUsersRequest {
    page: 1,
    page_size: 10,
    status: None,
    role: None,
});

let response = user_client.list_users(request).await?;
let list_response = response.into_inner();
println!("共 {} 个用户", list_response.total);
```

---

### 2. 认证服务 (AuthService)

认证服务提供用户登录和 Token 验证功能。

#### 2.1 Login - 用户登录

用户登录并获取 JWT Token。

**请求参数**:
```protobuf
message LoginRequest {
  string username = 1;    // 用户名
  string password = 2;    // 密码
}
```

**响应参数**:
```protobuf
message LoginResponse {
  bool success = 1;           // 是否成功
  string message = 2;         // 响应消息
  string token = 3;           // JWT Token
  User user = 4;              // 用户信息
}
```

**使用示例**:
```rust
let request = tonic::Request::new(LoginRequest {
    username: "admin".to_string(),
    password: "admin123".to_string(),
});

let response = auth_client.login(request).await?;
let login_response = response.into_inner();
println!("Token: {}", login_response.token);
```

---

#### 2.2 VerifyToken - 验证 Token

验证 JWT Token 的有效性。

**请求参数**:
```protobuf
message VerifyTokenRequest {
  string token = 1;  // JWT Token
}
```

**响应参数**:
```protobuf
message VerifyTokenResponse {
  bool success = 1;           // 是否成功
  string message = 2;         // 响应消息
  bool valid = 3;             // Token 是否有效
  User user = 4;              // 用户信息（如果有效）
}
```

**使用示例**:
```rust
let request = tonic::Request::new(VerifyTokenRequest {
    token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...".to_string(),
});

let response = auth_client.verify_token(request).await?;
let verify_response = response.into_inner();
if verify_response.valid {
    println!("Token 有效，用户：{}", verify_response.user.unwrap().username);
}
```

---

## 数据模型

### User

用户数据模型。

```protobuf
message User {
  int32 id = 1;               // 用户 ID
  string username = 2;        // 用户名
  string email = 3;           // 邮箱
  string phone = 4;           // 手机号
  string role = 5;            // 角色
  string status = 6;          // 状态
  int64 created_at = 7;       // 创建时间（Unix 时间戳）
  int64 updated_at = 8;       // 更新时间（Unix 时间戳）
  int64 last_login_at = 9;    // 最后登录时间（Unix 时间戳）
}
```

---

## 快速开始

### 1. 启动服务器

```bash
cd backend
cargo run
```

服务器启动后会同时监听：
- HTTP 端口：8000
- gRPC 端口：50051

### 2. 使用客户端示例

```bash
cargo run --example grpc_client
```

### 3. 在代码中使用

**添加依赖**:
```toml
[dependencies]
tonic = "0.10"
prost = "0.12"
tokio = { version = "1.0", features = ["full"] }
```

**连接服务**:
```rust
use tonic::transport::Channel;
use bingxi_backend::grpc::proto::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel = Channel::from_static("http://[::1]:50051")
        .connect()
        .await?;
    
    let mut user_client = UserServiceClient::new(channel);
    let mut auth_client = AuthServiceClient::new(channel);
    
    // 使用客户端...
    Ok(())
}
```

---

## 错误处理

gRPC 服务使用标准的 gRPC 状态码：

- `OK` (0): 成功
- `INVALID_ARGUMENT` (3): 参数无效
- `NOT_FOUND` (5): 资源不存在
- `INTERNAL` (13): 服务器内部错误
- `UNAUTHENTICATED` (16): 未认证

**处理示例**:
```rust
match user_client.get_user(request).await {
    Ok(response) => {
        // 处理成功响应
    }
    Err(e) => {
        match e.code() {
            tonic::Code::NotFound => {
                println!("用户不存在");
            }
            tonic::Code::Internal => {
                println!("服务器错误");
            }
            _ => {
                println!("其他错误：{}", e);
            }
        }
    }
}
```

---

## 测试

运行 gRPC 服务测试：

```bash
cargo test --test grpc_test
```

测试用例包括：
- 服务创建测试
- 登录成功/失败测试
- 用户创建测试
- 用户列表测试

---

## 最佳实践

### 1. 连接管理

复用 Channel 连接，避免频繁创建：

```rust
// 推荐：创建全局 Channel
lazy_static! {
    static ref GRPC_CHANNEL: Channel = {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Channel::from_static("http://[::1]:50051").connect())
            .unwrap()
    };
}
```

### 2. 错误处理

始终检查响应的 `success` 字段：

```rust
let response = client.create_user(request).await?;
let body = response.into_inner();
if !body.success {
    eprintln!("创建失败：{}", body.message);
    return Err(...);
}
```

### 3. 超时设置

为请求设置合理的超时时间：

```rust
let request = tonic::Request::new(GetUserRequest { user_id: 1 })
    .set_timeout(Duration::from_secs(5));
```

### 4. 重试机制

对于临时失败，实现重试逻辑：

```rust
use tokio::time::{sleep, Duration};

async fn call_with_retry<F, T>(mut f: F, max_retries: u32) -> Result<T, Error>
where
    F: FnMut() -> futures::future::BoxFuture<'static, Result<T, Error>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                sleep(Duration::from_millis(100 * attempts)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

---

## 常见问题

### Q: gRPC 和 REST API 有什么区别？

A: 
- **gRPC**: 基于 HTTP/2，使用 Protobuf 序列化，性能更高，适合内部服务通信
- **REST API**: 基于 HTTP/1.1，使用 JSON，易于调试，适合对外暴露

### Q: 如何选择使用 gRPC 还是 REST API？

A:
- 使用 **gRPC**: 微服务间通信、高性能要求、流式数据传输
- 使用 **REST API**: 浏览器客户端、第三方集成、简单 CRUD 操作

### Q: 如何调试 gRPC 请求？

A:
1. 使用 `grpcurl` 工具
2. 启用详细日志：`RUST_LOG=debug cargo run`
3. 使用 Wireshark 抓包分析

### Q: gRPC 支持浏览器直接调用吗？

A:
不直接支持。需要通过 gRPC-Web 代理（如 Envoy）或使用 REST API。

---

## 参考资料

- [gRPC 官方文档](https://grpc.io/docs/)
- [Tonic 框架文档](https://docs.rs/tonic/)
- [Prost 文档](https://docs.rs/prost/)
- [Protobuf 语言指南](https://protobuf.dev/programming-guides/proto3/)

---

**文档版本**: v1.0  
**最后更新**: 2026-03-15  
**维护者**: 秉羲团队
