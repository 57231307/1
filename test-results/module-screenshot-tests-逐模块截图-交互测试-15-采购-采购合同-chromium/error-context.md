# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: module-screenshot-tests.spec.ts >> 逐模块截图+交互测试 >> 15 [采购] 采购合同
- Location: module-screenshot-tests.spec.ts:141:9

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
        - heading "采购合同管理" [level=1] [ref=e40]
        - button "新建合同" [ref=e41] [cursor=pointer]
      - generic [ref=e42]:
        - textbox "搜索合同编号或名称..." [active] [ref=e43]
        - combobox [ref=e44]:
          - option "全部状态"
          - option "草稿"
          - option "已审核"
          - option "执行中"
          - option "已完成"
          - option "已取消" [selected]
        - button "刷新" [ref=e45] [cursor=pointer]
      - generic [ref=e46]: 解析响应失败：invalid length 0, expected struct PurchaseContractListResponse with 4 elements at line 1 column 25
      - table [ref=e48]:
        - rowgroup [ref=e49]:
          - row "合同编号 合同名称 供应商 总金额 交货日期 状态 操作" [ref=e50]:
            - columnheader "合同编号" [ref=e51]
            - columnheader "合同名称" [ref=e52]
            - columnheader "供应商" [ref=e53]
            - columnheader "总金额" [ref=e54]
            - columnheader "交货日期" [ref=e55]
            - columnheader "状态" [ref=e56]
            - columnheader "操作" [ref=e57]
        - rowgroup
      - generic [ref=e58]:
        - generic [ref=e59]: 共 0 条记录
        - button "上一页" [disabled] [ref=e60]
        - generic [ref=e61]: 第 1 页
        - button "下一页" [disabled] [ref=e62]
        - combobox [ref=e63]:
          - option "10条/页"
          - option "20条/页"
          - option "50条/页" [selected]
```