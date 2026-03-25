# 秉羲管理系统 - 数据迁移方案

## 迁移目标

将现有 Go 系统的 PostgreSQL 数据库完整迁移到 Rust 系统，确保数据完整性、一致性和安全性。

## 迁移原则

1. **数据完整性**: 确保所有数据完整迁移，不丢失任何记录
2. **业务一致性**: 保持业务逻辑和关联关系的一致性
3. **零停机时间**: 采用渐进式迁移策略，最小化系统停机时间
4. **可回滚**: 每个迁移步骤都可回滚，确保迁移失败时能快速恢复

## 迁移范围

### 核心数据表
- users (用户表)
- roles (角色表)
- departments (部门表)
- role_permissions (角色权限表)
- finance_payments (财务付款表)
- finance_invoices (财务发票表)
- sales_orders (销售订单表)
- sales_order_items (销售订单明细表)
- inventory_stock (库存表)
- products (产品表)
- product_categories (产品分类表)
- warehouses (仓库表)

## 迁移步骤

### 阶段一：迁移准备 (1-2 周)

1. **数据审计**
   - 统计现有数据量和表结构
   - 识别数据质量问题（重复、缺失、不一致）
   - 分析数据依赖关系

2. **环境准备**
   - 部署新的 PostgreSQL 18 数据库实例
   - 配置数据库连接参数（Version=18.0）
   - 创建数据库用户和权限

3. **工具准备**
   - 编写数据迁移脚本（Python/Go）
   - 准备数据验证工具
   - 搭建测试环境

### 阶段二：架构迁移 (2-3 周)

1. **Schema 迁移**
   ```sql
   -- 示例：创建 users 表
   CREATE TABLE users (
       id SERIAL PRIMARY KEY,
       username VARCHAR(100) NOT NULL UNIQUE,
       password_hash VARCHAR(255) NOT NULL,
       email VARCHAR(255),
       phone VARCHAR(50),
       role_id INTEGER,
       department_id INTEGER,
       is_active BOOLEAN DEFAULT true,
       last_login_at TIMESTAMP WITH TIME ZONE,
       created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
       updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
   );
   
   -- 创建索引
   CREATE INDEX idx_users_username ON users(username);
   CREATE INDEX idx_users_role_id ON users(role_id);
   CREATE INDEX idx_users_department_id ON users(department_id);
   ```

2. **数据字典迁移**
   - 迁移枚举类型和常量
   - 配置默认值
   - 设置约束条件

3. **触发器和存储过程**
   - 重写触发器逻辑（使用 Rust 代码或 PostgreSQL 触发器）
   - 迁移必要的存储过程

### 阶段三：数据迁移 (3-4 周)

1. **静态数据迁移**
   - 产品分类、仓库等基础数据
   - 角色和权限配置
   - 部门架构数据

2. **动态数据迁移**
   - 用户数据（密码需要重新哈希为 bcrypt）
   - 业务数据（订单、库存、财务）
   - 历史交易记录

3. **数据转换规则**
   ```python
   # 示例：密码哈希转换
   import bcrypt
   
   def migrate_password(old_hash):
       # 如果原系统使用 bcrypt，直接复制
       # 如果使用其他算法，需要验证后重新哈希
       return bcrypt.hashpw(password.encode('utf-8'), bcrypt.gensalt())
   ```

### 阶段四：验证和测试 (1-2 周)

1. **数据验证**
   - 记录数对比
   - 关键字段抽样检查
   - 关联关系验证
   - 业务规则验证

2. **功能测试**
   - 登录认证测试
   - CRUD 操作测试
   - 报表和查询测试
   - 权限控制测试

3. **性能测试**
   - 查询性能对比
   - 并发负载测试
   - 响应时间测试

### 阶段五：切换上线 (1 周)

1. **预上线演练**
   - 完整迁移流程演练
   - 回滚方案演练
   - 应急预案演练

2. **正式切换**
   ```
   步骤 1: 停止原系统写入（只读模式）
   步骤 2: 执行最终数据同步
   步骤 3: 数据验证
   步骤 4: 切换 DNS/负载均衡到新系统
   步骤 5: 监控运行状态
   ```

3. **上线后监控**
   - 实时监控系统性能
   - 收集用户反馈
   - 快速响应问题

## 数据迁移脚本示例

### Python 迁移脚本框架

```python
#!/usr/bin/env python3
"""
秉羲管理系统 - 数据迁移工具
"""

import psycopg2
from psycopg2.extras import DictCursor
import bcrypt
from datetime import datetime

class DataMigrator:
    def __init__(self, source_conn_str, target_conn_str):
        self.source_conn = psycopg2.connect(source_conn_str)
        self.target_conn = psycopg2.connect(target_conn_str)
    
    def migrate_users(self):
        """迁移用户数据"""
        with self.source_conn.cursor(cursor_factory=DictCursor) as src_cur:
            with self.target_conn.cursor() as tgt_cur:
                src_cur.execute("SELECT * FROM users")
                users = src_cur.fetchall()
                
                for user in users:
                    tgt_cur.execute("""
                        INSERT INTO users (id, username, password_hash, email, phone, 
                                         role_id, department_id, is_active, 
                                         last_login_at, created_at, updated_at)
                        VALUES (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)
                        ON CONFLICT (id) DO NOTHING
                    """, (
                        user['id'],
                        user['username'],
                        user['password_hash'],  # 可能需要重新哈希
                        user['email'],
                        user['phone'],
                        user['role_id'],
                        user['department_id'],
                        user['is_active'],
                        user['last_login_at'],
                        user['created_at'],
                        user['updated_at']
                    ))
        
        self.target_conn.commit()
        print(f"迁移完成：{len(users)} 个用户")
    
    def verify_migration(self):
        """验证迁移结果"""
        tables = ['users', 'roles', 'departments', 'products', 'sales_orders']
        
        for table in tables:
            with self.source_conn.cursor() as src_cur:
                with self.target_conn.cursor() as tgt_cur:
                    src_cur.execute(f"SELECT COUNT(*) FROM {table}")
                    src_count = src_cur.fetchone()[0]
                    
                    tgt_cur.execute(f"SELECT COUNT(*) FROM {table}")
                    tgt_count = tgt_cur.fetchone()[0]
                    
                    if src_count == tgt_count:
                        print(f"✓ {table}: {src_count} 条记录")
                    else:
                        print(f"✗ {table}: 源{src_count}条，目标{tgt_count}条")
    
    def close(self):
        self.source_conn.close()
        self.target_conn.close()

if __name__ == '__main__':
    migrator = DataMigrator(
        source_conn_str="postgresql://user:pass@old-host:5432/bingxi_old",
        target_conn_str="postgresql://user:pass@new-host:5432/bingxi_new?Version=18.0"
    )
    
    migrator.migrate_users()
    migrator.verify_migration()
    migrator.close()
```

## 回滚方案

### 回滚触发条件
- 数据验证失败率 > 1%
- 关键功能无法正常工作
- 性能指标不达标
- 严重安全漏洞

### 回滚步骤
1. 停止新系统服务
2. 切换回原系统数据库连接
3. 恢复原系统写入权限
4. 验证原系统功能
5. 分析迁移失败原因

## 风险评估

| 风险项 | 可能性 | 影响程度 | 缓解措施 |
|--------|--------|----------|----------|
| 数据丢失 | 低 | 高 | 完整备份、多次演练 |
| 数据不一致 | 中 | 高 | 自动验证工具、人工抽查 |
| 迁移时间超时 | 中 | 中 | 分批次迁移、优化脚本 |
| 性能下降 | 低 | 中 | 性能测试、索引优化 |
| 密码不兼容 | 低 | 高 | 提前测试、准备重置流程 |

## 成功标准

1. **数据完整性**: 所有表记录数一致率 100%
2. **功能正确性**: 所有核心功能测试通过
3. **性能指标**: 查询响应时间 ≤ 原系统
4. **用户体验**: 用户无感知切换

## 后续工作

1. 建立数据备份和恢复机制
2. 实施数据监控和告警
3. 定期执行数据质量检查
4. 优化数据库性能和索引

## 附录

### A. 数据库连接字符串示例
```
# 源数据库
postgresql://bingxi_user:password@old-server:5432/bingxi_old

# 目标数据库
postgresql://bingxi_user:password@new-server:5432/bingxi_new?Version=18.0
```

### B. 迁移检查清单
- [ ] 数据库备份完成
- [ ] 迁移脚本测试通过
- [ ] 验证工具准备就绪
- [ ] 回滚方案演练完成
- [ ] 监控告警配置完成
- [ ] 相关人员培训完成
- [ ] 用户通知已发送

### C. 联系人列表
- 项目负责人：[姓名]
- 数据库管理员：[姓名]
- 开发负责人：[姓名]
- 运维负责人：[姓名]
