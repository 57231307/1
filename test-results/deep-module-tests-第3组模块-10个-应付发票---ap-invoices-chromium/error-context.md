# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: deep-module-tests.spec.ts >> 第3组模块 (10个) >> 应付发票 - /ap-invoices
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
        - heading "应付发票管理" [level=1] [ref=e40]
        - paragraph [ref=e41]: 供应商应付发票管理、账龄分析和余额汇总
      - generic [ref=e42]:
        - button "发票列表" [ref=e43] [cursor=pointer]
        - button "账龄分析" [ref=e44] [cursor=pointer]
        - button "余额汇总" [ref=e45] [cursor=pointer]
      - generic [ref=e46]:
        - generic [ref=e47]:
          - heading "应付账款余额汇总" [level=2] [ref=e48]
          - button "刷新" [active] [ref=e49] [cursor=pointer]
        - paragraph [ref=e53]: 加载中...
```