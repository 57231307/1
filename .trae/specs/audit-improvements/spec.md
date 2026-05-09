# 审计改进实施规格

## Why
基于2026-05-09完整审计报告，系统存在安全、数据一致性、架构扩展性等方面的改进空间。本规格提取审计发现的所有问题，按优先级规划实施方法。

## What Changes
- **安全修复**: 移除生产环境debug日志、修复硬编码密钥警告
- **数据一致性**: 添加数据库级外键约束，消除数据孤岛风险
- **架构优化**: 推广DI容器使用，提升可测试性
- **代码质量**: 增加集成测试覆盖，完善gRPC服务启用
- **功能完善**: 补充缺失的CRUD操作，统一API响应格式

## Impact
- 受影响代码: backend/src/middleware/, backend/src/services/, backend/src/utils/, database/migration/
- 受影响文档: README.md, CHANGELOG.md

## ADDED Requirements

### Requirement: 安全修复
The system SHALL remove all debug logs that expose sensitive information in production environment.

#### Scenario: Debug log removal
- **WHEN** the system runs in production mode
- **THEN** no debug logs output JWT key lengths or user information

### Requirement: 数据库外键完善
The system SHALL add database-level foreign key constraints for core business entities.

#### Scenario: Foreign key creation
- **WHEN** creating new migration for foreign keys
- **THEN** all core business tables have proper foreign key relationships

### Requirement: DI容器推广
The system SHALL use DIContainer for service creation instead of direct AppState dependency.

#### Scenario: Service creation via DI
- **WHEN** a new service is created
- **THEN** it is registered in DIContainer and retrieved from it

### Requirement: 集成测试覆盖
The system SHALL have integration tests for critical business flows.

#### Scenario: Test coverage
- **WHEN** running cargo test
- **THEN** critical paths have test coverage

## MODIFIED Requirements

### Requirement: 现有安全中间件
The existing auth middleware SHALL be enhanced to remove debug logging in production.

### Requirement: 现有数据模型
The existing models SHALL have database-level foreign key constraints added.

## REMOVED Requirements
None.
