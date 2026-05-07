# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: deep-module-tests.spec.ts >> 第1组模块 (10个) >> 产品管理 - /products
- Location: deep-module-tests.spec.ts:179:11

# Error details

```
Test timeout of 600000ms exceeded.
```

# Page snapshot

```yaml
- generic [ref=e2]:
  - navigation [ref=e3]:
    - generic [ref=e4]: 秉羲面料管理
    - generic [ref=e5]:
      - generic [ref=e6]:
        - generic [ref=e7] [cursor=pointer]:
          - generic [ref=e8]: 工作台
          - generic [ref=e9]: ▼
        - generic [ref=e10]:
          - generic [ref=e11]: 首页
          - generic [ref=e12]: 我的待办
      - generic [ref=e14] [cursor=pointer]:
        - generic [ref=e15]: 基础数据
        - generic [ref=e16]: ▼
      - generic [ref=e18] [cursor=pointer]:
        - generic [ref=e19]: 供应链管理
        - generic [ref=e20]: ▼
      - generic [ref=e22] [cursor=pointer]:
        - generic [ref=e23]: 仓储与质量
        - generic [ref=e24]: ▼
      - generic [ref=e26] [cursor=pointer]:
        - generic [ref=e27]: 财务核算
        - generic [ref=e28]: ▼
      - generic [ref=e30] [cursor=pointer]:
        - generic [ref=e31]: 面料行业特色
        - generic [ref=e32]: ▼
      - generic [ref=e34] [cursor=pointer]:
        - generic [ref=e35]: 系统与分析
        - generic [ref=e36]: ▼
  - main [ref=e37]:
    - generic [ref=e38]:
      - generic [ref=e39]:
        - heading "产品管理" [level=1] [ref=e40]
        - button "新建产品" [ref=e41] [cursor=pointer]
      - generic [ref=e42]: "删除失败: 请求失败 (500): {\"error\":\"DatabaseError\",\"message\":\"Query Error: Exec(SqlxError(Database(PgDatabaseError { severity: Error, code: \\\"23503\\\", message: \\\"update or delete on table \\\\\\\"products\\\\\\\" violates foreign key constraint \\\\\\\"fk_dye_lot_product\\\\\\\" on table \\\\\\\"batch_dye_lot\\\\\\\"\\\", detail: Some(\\\"Key (id)=(2) is still referenced from table \\\\\\\"batch_dye_lot\\\\\\\".\\\"), hint: None, position: None, where: None, schema: Some(\\\"public\\\"), table: Some(\\\"batch_dye_lot\\\"), column: None, data_type: None, constraint: Some(\\\"fk_dye_lot_product\\\"), file: Some(\\\"ri_triggers.c\\\"), line: Some(2609), routine: Some(\\\"ri_ReportViolation\\\") })))\"}"
```