import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount, flushPromises } from '@vue/test-utils'
import { createI18n } from 'vue-i18n'
import ElementPlus from 'element-plus'

// 批次 29 v7 P0-7 修复：原测试用 LoginMock 自定义组件，未测试真实 Login.vue。
// 改为 mount 真实 Login.vue，并 mock 依赖（userStore / securityApi / router），
// 验证真实组件的渲染、表单校验、登录流程、错误处理。
//
// 批次 29 v7 P0-7 修复补丁 1：vi.mock 工厂函数会被 hoist 到文件顶部，
// 此时顶层 const 变量尚未初始化（ReferenceError: Cannot access 'mockLogin' before initialization）。
// 改用 vi.hoisted() 创建 mock 函数，确保 hoist 后变量仍可访问。
//
// 批次 29 v7 P0-7 修复补丁 2：tests/setup.ts 全局 mock 了 vue-router，但只导出
// useRouter/useRoute/createRouter/createWebHistory，未导出 createMemoryHistory，
// 导致本测试导入 createMemoryHistory 失败。
// 解决方案：本测试文件不依赖真实 vue-router，而是在本文件内重新 mock vue-router，
// 用 spy 控制 useRouter().push 与 useRoute().query.redirect，覆盖 setup.ts 的 mock。

const {
  mockLogin,
  mockCheckLockStatus,
  pushSpy,
  routeRef,
} = vi.hoisted(() => ({
  // userStore.login：默认 resolve，可通过 mockRejectedValueOnce 修改行为
  mockLogin: vi.fn().mockResolvedValue(undefined),
  // securityApi.checkLockStatus：默认返回未锁定
  mockCheckLockStatus: vi.fn().mockResolvedValue({
    data: {
      is_locked: false,
      failed_attempts: 0,
      locked_until: null,
      max_attempts: 5,
    },
  }),
  // useRouter().push 的 spy，用于断言跳转目标
  pushSpy: vi.fn(),
  // useRoute() 返回的响应式对象，测试中修改 query.redirect 测试不同场景
  routeRef: {
    path: '/login',
    query: {} as Record<string, string>,
    params: {} as Record<string, string>,
    meta: {},
  },
}))

vi.mock('@/store/user', () => ({
  useUserStore: () => ({
    login: mockLogin,
    userInfo: null,
  }),
}))

vi.mock('@/api/security', () => ({
  securityApi: {
    checkLockStatus: mockCheckLockStatus,
  },
}))

// 重新 mock vue-router，覆盖 tests/setup.ts 的全局 mock
// 关键：提供 useRouter（返回带 pushSpy 的对象）+ useRoute（返回 routeRef）
vi.mock('vue-router', () => ({
  useRouter: () => ({
    push: pushSpy,
    replace: vi.fn(),
    go: vi.fn(),
    back: vi.fn(),
    forward: vi.fn(),
  }),
  useRoute: () => routeRef,
  // 保留其他可能被 Login.vue 间接引用的导出
  RouterLink: { template: '<a><slot /></a>' },
  RouterView: { template: '<div><slot /></div>' },
}))

// Mock logger（防止测试输出噪声）
vi.mock('@/utils/logger', () => ({
  logger: {
    warn: vi.fn(),
    info: vi.fn(),
    error: vi.fn(),
    debug: vi.fn(),
  },
}))

// Mock ElMessage（避免 Element Plus 全局副作用）
vi.mock('element-plus', async () => {
  const actual = await vi.importActual<typeof import('element-plus')>('element-plus')
  return {
    ...actual,
    ElMessage: {
      success: vi.fn(),
      error: vi.fn(),
      warning: vi.fn(),
      info: vi.fn(),
    },
  }
})

import Login from '@/views/Login.vue'

// 创建测试用 i18n 实例
const i18n = createI18n({
  legacy: false,
  locale: 'zh-CN',
  messages: {
    'zh-CN': {
      login: {
        subtitle: '面料管理系统',
        username: '用户名',
        password: '密码',
        submit: '登录',
        formLabel: '登录表单',
        usernameRequired: '请输入用户名',
        passwordRequired: '请输入密码',
        success: '登录成功',
        lockedAlert: '账号已锁定，请 {minutes} 分钟后重试',
        failedAttempts: '失败次数：{count}',
        remainingTime: '剩余时间：{minutes} 分 {seconds} 秒',
        unlocked: '账号已解锁，可以登录',
        failedFallback: '登录失败',
      },
    },
  },
})

function mountLogin() {
  const wrapper = mount(Login, {
    global: {
      plugins: [i18n, ElementPlus],
    },
  })
  return { wrapper }
}

describe('Login.vue 真实组件测试', () => {
  beforeEach(() => {
    mockLogin.mockClear()
    mockLogin.mockResolvedValue(undefined) // 重置为默认 resolve
    mockCheckLockStatus.mockClear()
    pushSpy.mockClear()
    // 重置 routeRef.query
    routeRef.query = {}
  })

  it('应该正确渲染登录页面（标题 + 用户名/密码输入框 + 登录按钮）', async () => {
    const { wrapper } = mountLogin()
    await flushPromises()
    // 标题
    expect(wrapper.find('.login-title').text()).toBe('面料管理系统')
    // 用户名输入框存在（el-input 渲染为 input）
    const inputs = wrapper.findAll('input')
    expect(inputs.length).toBeGreaterThanOrEqual(2)
    // 登录按钮存在
    const button = wrapper.find('button')
    expect(button.exists()).toBe(true)
    expect(button.text()).toBe('登录')
  })

  it('用户名为空时点击登录应触发 form 校验（不调用 userStore.login）', async () => {
    const { wrapper } = mountLogin()
    await flushPromises()
    const button = wrapper.find('button')
    await button.trigger('click')
    await flushPromises()
    // 校验失败不应调用 login
    expect(mockLogin).not.toHaveBeenCalled()
  })

  it('用户名 + 密码有效时点击登录应调用 userStore.login', async () => {
    const { wrapper } = mountLogin()
    await flushPromises()
    const inputs = wrapper.findAll('input')
    // 输入用户名
    await inputs[0].setValue('admin')
    // 输入密码
    await inputs[1].setValue('password123')
    await flushPromises()
    // 点击登录
    const button = wrapper.find('button')
    await button.trigger('click')
    await flushPromises()
    // 应调用 userStore.login，参数为 { username: 'admin', password: 'password123' }
    expect(mockLogin).toHaveBeenCalledTimes(1)
    expect(mockLogin).toHaveBeenCalledWith({
      username: 'admin',
      password: 'password123',
    })
  })

  it('登录失败时不应跳转路由（userStore.login reject 时 router.push 不执行）', async () => {
    const { wrapper } = mountLogin()
    await flushPromises()
    // 模拟登录失败
    mockLogin.mockRejectedValueOnce(new Error('用户名或密码错误'))
    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('admin')
    await inputs[1].setValue('wrongpassword')
    await flushPromises()
    await wrapper.find('button').trigger('click')
    await flushPromises()
    // 登录失败，不应 push
    expect(pushSpy).not.toHaveBeenCalled()
  })

  it('登录成功后应跳转到 redirect 参数指定的安全路径', async () => {
    // 设置 route.query.redirect = '/dashboard'
    routeRef.query = { redirect: '/dashboard' }
    const { wrapper } = mountLogin()
    await flushPromises()
    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('admin')
    await inputs[1].setValue('password123')
    await flushPromises()
    await wrapper.find('button').trigger('click')
    await flushPromises()
    // 应跳转到 /dashboard
    expect(pushSpy).toHaveBeenCalledWith('/dashboard')
  })

  it('登录成功后 redirect 为外部 URL 时应回退到 /（防 Open Redirect）', async () => {
    // 设置 route.query.redirect = '//evil.com'（外部 URL）
    routeRef.query = { redirect: '//evil.com' }
    const { wrapper } = mountLogin()
    await flushPromises()
    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('admin')
    await inputs[1].setValue('password123')
    await flushPromises()
    await wrapper.find('button').trigger('click')
    await flushPromises()
    // 应跳转到 /，而非 //evil.com
    expect(pushSpy).toHaveBeenCalledWith('/')
  })

  it('用户名输入框失焦时应调用 securityApi.checkLockStatus 预检查锁定状态', async () => {
    const { wrapper } = mountLogin()
    await flushPromises()
    const inputs = wrapper.findAll('input')
    await inputs[0].setValue('lockeduser')
    await flushPromises()
    // 触发 blur 事件
    await inputs[0].trigger('blur')
    await flushPromises()
    // 应调用 checkLockStatus
    expect(mockCheckLockStatus).toHaveBeenCalledWith('lockeduser')
  })
})
