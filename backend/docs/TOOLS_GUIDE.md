# 秉羲管理系统 - 工具使用指南

本文档介绍如何使用项目中的各种工具和文档。

## 目录

1. [Swagger UI 使用](#swagger-ui-使用)
2. [Postman 集合导入](#postman-集合导入)
3. [性能测试](#性能测试)
4. [Rustdoc 文档](#rustdoc-文档)

---

## Swagger UI 使用

### 启动服务

```bash
cd e:\1\10\bingxi-rust\backend
cargo run
```

### 访问 Swagger UI

服务启动后，在浏览器中访问:

```
http://localhost:8080/swagger-ui/
```

### 功能说明

Swagger UI 提供以下功能:

1. **API 浏览**: 查看所有可用的 API 端点，按模块分类
2. **请求测试**: 直接在浏览器中测试 API
3. **模型查看**: 查看数据模型的结构
4. **认证管理**: 管理 JWT Token

### 使用步骤

1. **认证**: 
   - 点击Authorize按钮
   - 输入JWT Token (格式：`Bearer your_token`)
   - 点击Authorize确认

2. **测试 API**:
   - 展开要测试的端点
   - 点击"Try it out"
   - 填写请求参数
   - 点击"Execute"执行
   - 查看响应结果

3. **查看模型**:
   - 滚动到页面底部
   - 查看Schemas部分
   - 查看所有数据模型定义

---

## Postman 集合导入

### 导入集合

1. 打开 Postman
2. 点击左上角的"Import"按钮
3. 选择文件 `docs/postman_collection.json`
4. 点击"Import"完成导入

### 配置环境变量

集合已自动配置以下变量:

- `base_url`: 默认为 `http://localhost:8080/api/v1/erp`
- `jwt_token`: 登录后自动设置

### 使用集合

#### 1. 登录获取 Token

- 展开"认证"文件夹
- 选择"用户登录"请求
- 发送请求
- Token 会自动保存到环境变量

#### 2. 测试采购合同

- 展开"采购合同管理"文件夹
- 按顺序执行请求:
  1. 获取采购合同列表
  2. 创建采购合同
  3. 获取单个采购合同
  4. 审核采购合同
  5. 执行采购合同
  6. 取消采购合同
  7. 删除采购合同

#### 3. 测试其他模块

同样的方式测试:
- 销售合同管理
- 固定资产管理
- 预算管理

### 自动化测试

Postman 集合包含自动测试脚本:

- 登录成功后自动保存 Token
- 所有请求自动使用 Token 认证
- 响应状态码验证

---

## 性能测试

### 使用 wrk 测试

#### 安装 wrk

**Windows:**
```bash
choco install wrk
```

**macOS:**
```bash
brew install wrk
```

**Linux:**
```bash
apt-get install wrk
# 或
yum install wrk
```

#### 运行测试

```bash
cd e:\1\10\bingxi-rust\backend\scripts

# 设置环境变量 (可选)
export JWT_TOKEN="your_test_token_here"
export BASE_URL="http://localhost:8080"

# 运行测试
chmod +x performance_test_wrk.sh
./performance_test_wrk.sh
```

#### 测试结果解读

wrk 输出包含:

- **Latency**: 请求延迟统计
  - Avg: 平均延迟
  - Stdev: 标准差
  - Max: 最大延迟
  - 百分位数：50%, 75%, 90%, 95%, 99%

- **Req/Sec**: 每秒请求数 (吞吐量)
  - 平均值和峰值

- **Requests**: 总请求数

#### 自定义测试参数

编辑 `performance_test_wrk.sh`:

```bash
# 修改测试参数
connections=100    # 并发连接数
threads=8          # 线程数
duration=30s       # 测试持续时间
```

### 使用 Apache Bench (ab) 测试

#### 安装 ab

**Windows:**
```bash
choco install apache
```

**macOS:**
```bash
brew install httpd
```

**Linux:**
```bash
apt-get install apache2-utils
# 或
yum install httpd-tools
```

#### 运行测试

```bash
cd e:\1\10\bingxi-rust\backend\scripts

# 设置环境变量
export JWT_TOKEN="your_test_token_here"

# 运行测试
chmod +x performance_test_ab.sh
./performance_test_ab.sh
```

#### 测试结果解读

ab 输出包含:

- **Requests per second**: 每秒请求数
- **Time per request**: 每个请求的时间
- **Transfer rate**: 数据传输速率
- **Connection Times**: 连接时间统计
- **Percentage of requests served**: 请求完成百分比

---

## Rustdoc 文档

### 生成文档

```bash
cd e:\1\10\bingxi-rust\backend

# 生成文档
cargo doc --no-deps

# 生成并打开文档
cargo doc --no-deps --open
```

### 查看文档

生成的文档位于:

```
target/doc/bingxi_backend/
```

用浏览器打开 `target/doc/bingxi_backend/index.html`

### 文档内容

Rustdoc 生成的文档包含:

1. **模块文档**: 所有模块的说明
2. **结构体文档**: 所有数据结构的定义
3. **函数文档**: 所有公共函数的签名和说明
4. **Trait 文档**: 所有 Trait 的定义和实现

### 添加文档注释

在代码中使用文档注释:

```rust
/// 采购合同服务
/// 
/// 提供采购合同管理的业务逻辑
/// 
/// # Examples
/// 
/// ```
/// let service = PurchaseContractService::new(db);
/// let contracts = service.get_list(params).await?;
/// ```
pub struct PurchaseContractService {
    // ...
}
```

### 包含私有项

```bash
# 生成包含私有项的完整文档
cargo doc --no-deps --document-private-items
```

---

## 综合使用建议

### 开发阶段

1. **编写代码**: 使用文档注释
2. **生成 Rustdoc**: `cargo doc --open`
3. **测试 API**: 使用 Postman 集合
4. **查看 Swagger**: 浏览器访问 Swagger UI

### 测试阶段

1. **功能测试**: Postman 集合
2. **性能测试**: wrk 或 ab
3. **文档检查**: Rustdoc 和 Swagger

### 部署前

1. **运行所有测试**: `cargo test`
2. **性能基准测试**: wrk/ab
3. **生成完整文档**: `cargo doc --document-private-items`
4. **导出 API 文档**: 从 Swagger UI 导出 OpenAPI JSON

---

## 常见问题

### Q: Swagger UI 无法访问？

A: 确保服务已启动，并检查端口是否正确:
```bash
# 检查服务是否运行
curl http://localhost:8080/api/health
```

### Q: Postman 请求返回 401?

A: 确保已执行登录请求并成功获取 Token。检查环境变量 `jwt_token` 是否有值。

### Q: 性能测试失败？

A: 检查:
1. 服务是否正在运行
2. JWT Token 是否有效
3. 测试工具是否正确安装

### Q: Rustdoc 缺少某些项？

A: 默认只生成公共项文档。使用 `--document-private-items` 生成完整文档。

---

## 最佳实践

1. **每次代码变更后**: 重新生成 Rustdoc
2. **添加新功能时**: 更新 Postman 集合
3. **定期性能测试**: 每周至少一次
4. **文档版本控制**: 将 docs/ 目录纳入版本管理
5. **自动化**: 在 CI/CD 中集成文档生成和测试

---

## 技术支持

如有问题，请联系:
- 邮箱：support@bingxi.com
- 文档：docs/README.md
