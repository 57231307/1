import { vi } from 'vitest'

// Mock Element Plus（基于 importActual 保留全部真实导出，再覆盖需要的组件为可挂载的测试桩）
vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')
  return {
    ...actual,
    ElTableV2: {
      name: 'ElTableV2',
      props: ['columns', 'data', 'width', 'height', 'estimatedRowHeight', 'loading', 'emptyText', 'rowKey'],
      emits: ['row-click', 'selection-change', 'scroll', 'column-sort'],
      // 测试桩：空数据时显示 emptyText，便于断言「暂无数据」
      template: `<div class="el-table-v2">
        <div v-if="!data || data.length === 0" class="el-table-v2__empty">{{ emptyText }}</div>
        <div v-else class="el-table-v2__rows">
          <div v-for="(row, i) in data" :key="i" class="el-table-v2__row">{{ JSON.stringify(row) }}</div>
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
vi.mock('vue-router', () => ({
  useRouter: () => ({
    push: vi.fn(),
    replace: vi.fn(),
    go: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  useRoute: () => ({
    path: '/',
    query: {},
    params: {},
    meta: {},
  }),
  createRouter: vi.fn(),
  createWebHistory: vi.fn(),
}))

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
