import { vi } from 'vitest'
import { config } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import zhCN from '@/locales/zh-CN'
import enUS from '@/locales/en-US'

// Mock Element Plus（基于 importActual 保留全部真实导出，覆盖 ElTableV2/ElAutoResizer 为可挂载的测试桩）
// 备注：cherry-pick trae V2Table 测试需要 ElAutoResizer/ElTableV2 的测试桩；其余组件（ElMessage/ElPagination 等）保留真实导出
vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')
  return {
    ...actual,
    ElTableV2: {
      name: 'ElTableV2',
      props: ['columns', 'data', 'width', 'height', 'estimatedRowHeight', 'loading', 'emptyText', 'rowKey'],
      emits: ['row-click', 'selection-change', 'scroll', 'column-sort'],
      // 测试桩：调用 cellRenderer 以便验证 V2Table 的 renderCell 缓存逻辑
      template: `<div class="el-table-v2">
        <div v-if="!data || data.length === 0" class="el-table-v2__empty">{{ emptyText }}</div>
        <div v-else class="el-table-v2__rows">
          <div v-for="(row, i) in data" :key="i" class="el-table-v2__row">
            <div v-for="col in columns" :key="col.key" class="el-table-v2__cell" :data-key="col.key">
              {{ col.cellRenderer ? col.cellRenderer({ rowData: row, rowIndex: i, column: col }) : '' }}
            </div>
          </div>
        </div>
      </div>`,
    },
    ElAutoResizer: {
      name: 'ElAutoResizer',
      template: '<div class="el-auto-resizer"><slot :width="0" :height="0" /></div>',
    },
  }
})

// Mock Vue Router
// 注意：createRouter 必须返回含 beforeEach/afterEach 等方法的对象，
// 否则 src/router/index.ts 顶层调用 router.beforeEach 时会因 undefined 报错
// （通过 @/api/request → @/router 的传递导入链触发）
vi.mock('vue-router', () => {
  const routerInstance = {
    beforeEach: vi.fn(),
    afterEach: vi.fn(),
    beforeResolve: vi.fn(),
    push: vi.fn(),
    replace: vi.fn(),
    go: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
    addRoute: vi.fn(),
    removeRoute: vi.fn(),
    hasRoute: vi.fn().mockReturnValue(false),
    getRoutes: vi.fn().mockReturnValue([]),
  }
  return {
    useRouter: () => routerInstance,
    useRoute: () => ({
      path: '/',
      query: {},
      params: {},
      meta: {},
    }),
    createRouter: vi.fn().mockReturnValue(routerInstance),
    createWebHistory: vi.fn(),
  }
})

// Mock Pinia
vi.mock('pinia', () => ({
  defineStore: vi.fn().mockReturnValue(vi.fn()),
  createPinia: vi.fn().mockReturnValue({
    install: vi.fn(),
  }),
  setActivePinia: vi.fn(),
  storeToRefs: vi.fn().mockReturnValue({}),
}))

// Mock Axios
vi.mock('axios', () => ({
  default: {
    create: vi.fn().mockReturnValue({
      interceptors: {
        request: { use: vi.fn() },
        response: { use: vi.fn() },
      },
      get: vi.fn().mockResolvedValue({ data: {} }),
      post: vi.fn().mockResolvedValue({ data: {} }),
      put: vi.fn().mockResolvedValue({ data: {} }),
      delete: vi.fn().mockResolvedValue({ data: {} }),
    }),
    get: vi.fn().mockResolvedValue({ data: {} }),
    post: vi.fn().mockResolvedValue({ data: {} }),
    put: vi.fn().mockResolvedValue({ data: {} }),
    delete: vi.fn().mockResolvedValue({ data: {} }),
  },
}))

// Mock window.matchMedia
Object.defineProperty(window, 'matchMedia', {
  writable: true,
  value: vi.fn().mockImplementation((query) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: vi.fn(),
    removeListener: vi.fn(),
    dispatchEvent: vi.fn(),
  })),
})

// Mock IntersectionObserver
class MockIntersectionObserver {
  observe = vi.fn()
  disconnect = vi.fn()
  unobserve = vi.fn()
}

Object.defineProperty(window, 'IntersectionObserver', {
  writable: true,
  configurable: true,
  value: MockIntersectionObserver,
})

// Mock ResizeObserver
class MockResizeObserver {
  observe = vi.fn()
  disconnect = vi.fn()
  unobserve = vi.fn()
}

Object.defineProperty(window, 'ResizeObserver', {
  writable: true,
  configurable: true,
  value: MockResizeObserver,
})

// 全局注入 i18n 插件（项目所有 .vue 组件均使用 useI18n，测试环境需模拟生产环境）
const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  fallbackLocale: 'en-US',
  messages: {
    'zh-CN': zhCN,
    'en-US': enUS,
  },
})
config.global.plugins = [i18n]
