# Instructions

- Following Playwright test failed.
- Explain why, be concise, respect Playwright best practices.
- Provide a snippet of code with the fix, if possible.

# Test info

- Name: module-screenshot-tests.spec.ts >> 逐模块截图+交互测试 >> 02 [系统] 用户管理
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
      - heading "用户管理" [level=1] [ref=e40]
      - generic [ref=e41]:
        - table [ref=e42]:
          - rowgroup [ref=e43]:
            - row "ID 用户名 邮箱 手机号 角色 状态 创建时间" [ref=e44]:
              - columnheader "ID" [ref=e45]
              - columnheader "用户名" [ref=e46]
              - columnheader "邮箱" [ref=e47]
              - columnheader "手机号" [ref=e48]
              - columnheader "角色" [ref=e49]
              - columnheader "状态" [ref=e50]
              - columnheader "创建时间" [ref=e51]
          - rowgroup [ref=e52]:
            - 'row "1 测试用户1 - - 角色 #100 正常 2026-05-03T14:14:32.472113Z" [ref=e53]':
              - cell "1" [ref=e54]
              - cell "测试用户1" [ref=e55]
              - cell "-" [ref=e56]
              - cell "-" [ref=e57]
              - 'cell "角色 #100" [ref=e58]'
              - cell "正常" [ref=e59]:
                - generic [ref=e60]: 正常
              - cell "2026-05-03T14:14:32.472113Z" [ref=e61]
            - 'row "2 测试用户2 - - 角色 #100 正常 2026-05-03T14:14:32.472113Z" [ref=e62]':
              - cell "2" [ref=e63]
              - cell "测试用户2" [ref=e64]
              - cell "-" [ref=e65]
              - cell "-" [ref=e66]
              - 'cell "角色 #100" [ref=e67]'
              - cell "正常" [ref=e68]:
                - generic [ref=e69]: 正常
              - cell "2026-05-03T14:14:32.472113Z" [ref=e70]
            - 'row "0 admin admin@example.com - 角色 #1 正常 2026-05-03T14:14:39.671576Z" [ref=e71]':
              - cell "0" [ref=e72]
              - cell "admin" [ref=e73]
              - cell "admin@example.com" [ref=e74]
              - cell "-" [ref=e75]
              - 'cell "角色 #1" [ref=e76]'
              - cell "正常" [ref=e77]:
                - generic [ref=e78]: 正常
              - cell "2026-05-03T14:14:39.671576Z" [ref=e79]
        - generic [ref=e80]:
          - button "上一页" [disabled] [ref=e81]
          - generic [ref=e82]: 第 1 页 / 共 3 条
          - button "下一页" [disabled] [ref=e83]
```