# 前端架构文档

## 一、技术栈

| 类别 | 技术 | 版本 |
|------|------|------|
| 框架 | Vue 3 | ^3.4.0 |
| 构建工具 | Vite | ^6.4.3 |
| 类型系统 | TypeScript | ~5.4.0 |
| UI 框架 | Element Plus | ^2.6.0 |
| 状态管理 | Pinia | ^2.1.0 |
| 路由 | Vue Router | ^4.3.0 |
| HTTP 客户端 | Axios | ^1.6.0 |
| 图表库 | ECharts | ^6.1.0 |
| 拖拽排序 | vuedraggable | ^4.1.0 |
| 打印 | print-js | ^1.6.0 |
| 代码规范 | ESLint + Prettier | 8.56 + 3.2 |
| 测试框架 | Vitest + Playwright | 2.1 + 1.60 |

## 二、目录结构

```
frontend/
├── src/
│   ├── api/              # API 调用层
│   │   ├── request.ts    # Axios 封装
│   │   ├── auth.ts       # 认证 API
│   │   ├── dashboard.ts  # 仪表盘 API
│   │   ├── sales.ts      # 销售 API
│   │   └── ...           # 其他业务 API
│   │
│   ├── components/       # 公共组件
│   │   ├── Layout/       # 布局组件
│   │   ├── Charts/       # 图表组件
│   │   ├── AdvancedFilter.vue
│   │   ├── BatchActions.vue
│   │   └── DraggableTable.vue
│   │
│   ├── directives/       # 自定义指令
│   │
│   ├── router/           # 路由配置
│   │   └── index.ts      # 路由定义
│   │
│   ├── store/            # Pinia 状态管理
│   │   ├── index.ts      # Store 导出
│   │   ├── user.ts       # 用户状态
│   │   ├── permission.ts # 权限状态
│   │   ├── dashboard.ts  # 仪表盘状态
│   │   ├── sales.ts      # 销售状态
│   │   ├── fabric.ts     # 面料状态
│   │   └── inventory.ts  # 库存状态
│   │
│   ├── types/            # TypeScript 类型定义
│   │   └── api.ts        # API 响应类型
│   │
│   ├── utils/            # 工具函数
│   │   ├── index.ts      # 通用工具
│   │   ├── storage.ts    # 本地存储
│   │   ├── lazy-loader.ts# 懒加载
│   │   ├── export.ts     # 导出功能
│   │   └── print.ts      # 打印功能
│   │
│   ├── views/            # 页面组件
│   │   ├── Dashboard.vue # 仪表盘
│   │   ├── Login.vue     # 登录页
│   │   ├── Setup.vue     # 初始化页
│   │   ├── 403.vue       # 无权限页
│   │   ├── 404.vue       # 404页面
│   │   ├── sales/        # 销售模块
│   │   ├── purchase/     # 采购模块
│   │   ├── inventory/    # 库存模块
│   │   ├── finance/      # 财务模块
│   │   ├── crm/          # CRM模块
│   │   ├── production/   # 生产模块
│   │   └── ...           # 其他业务模块
│   │
│   └── main.ts           # 入口文件
│
├── public/               # 静态资源
├── dist/                 # 构建产物
├── scripts/              # 构建脚本
├── vite.config.ts        # Vite 配置
├── tsconfig.json         # TypeScript 配置
├── package.json          # 依赖配置
└── nginx.conf            # Nginx 配置
```

## 三、核心配置

### 3.1 Vite 配置

```typescript
// vite.config.ts
export default defineConfig({
  plugins: [vue(), AutoImport(), Components()],
  server: {
    port: 3000,
    proxy: {
      '/api': { target: 'http://localhost:8082' }  // API 代理
    }
  },
  build: {
    outDir: 'dist',
    assetsDir: 'static',
    sourcemap: false
  }
})
```

### 3.2 API 基础配置

```typescript
// src/api/request.ts
const request = axios.create({
  baseURL: '/api/v1/erp',
  timeout: 30000
})
```

### 3.3 路由配置

```typescript
// src/router/index.ts
- 76 个页面路由
- 支持懒加载
- 认证守卫
- 权限检查
```

## 四、模块统计

| 模块 | 数量 |
|------|------|
| 页面组件 | 76 |
| API 文件 | 80 |
| Store 模块 | 7 |
| 公共组件 | 5 |
| 工具函数 | 5 |

## 五、构建与部署

### 5.1 开发环境

```bash
npm run dev        # 启动开发服务器
npm run build      # 构建生产版本
npm run preview    # 预览构建结果
npm run lint       # 代码检查
npm run test       # 运行测试
```

### 5.2 生产部署

```bash
# 构建
npm run build

# 部署到 Nginx
cp -r dist/* /opt/bingxi-erp/frontend/dist/
```

### 5.3 Nginx 配置

```nginx
server {
    listen 80;
    root /opt/bingxi-erp/frontend/dist;
    
    location / {
        try_files $uri $uri/ /index.html;
    }
    
    location /api/ {
        proxy_pass http://127.0.0.1:8082;
    }
}
```

## 六、代码规范

### 6.1 命名规范

- **文件名**：kebab-case（如 `user-profile.ts`）
- **组件名**：PascalCase（如 `UserProfile.vue`）
- **变量名**：camelCase（如 `userInfo`）
- **常量名**：UPPER_SNAKE_CASE（如 `API_BASE_URL`）

### 6.2 文件组织

- 每个业务模块一个目录
- API 文件与页面组件对应
- 共享组件放在 `components/` 目录
- 工具函数放在 `utils/` 目录
