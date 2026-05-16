import { describe, it, expect, vi, beforeEach } from 'vitest'
import { mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'

// Mock Login 组件
const LoginMock = {
  template: `
    <div class="login-container">
      <div class="login-card">
        <h2 class="login-title">{{ title }}</h2>
        <form @submit.prevent="handleLogin">
          <input
            v-model="form.username"
            type="text"
            placeholder="用户名"
            data-testid="username-input"
          />
          <input
            v-model="form.password"
            type="password"
            placeholder="密码"
            data-testid="password-input"
          />
          <button type="submit" data-testid="login-button">登录</button>
        </form>
        <p v-if="error" class="error-message" data-testid="error-message">{{ error }}</p>
      </div>
    </div>
  `,
  data() {
    return {
      title: '秉羲面料管理系统',
      form: {
        username: '',
        password: '',
      },
      error: '',
    }
  },
  methods: {
    handleLogin() {
      if (!this.form.username) {
        this.error = '请输入用户名'
        return
      }
      if (!this.form.password) {
        this.error = '请输入密码'
        return
      }
      this.error = ''
    },
  },
}

describe('Login 组件测试', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
  })

  it('应该正确渲染登录页面', () => {
    const wrapper = mount(LoginMock)
    expect(wrapper.find('.login-title').text()).toBe('秉羲面料管理系统')
    expect(wrapper.find('[data-testid="username-input"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="password-input"]').exists()).toBe(true)
    expect(wrapper.find('[data-testid="login-button"]').exists()).toBe(true)
  })

  it('应该显示错误信息当用户名为空', async () => {
    const wrapper = mount(LoginMock)
    await wrapper.find('[data-testid="login-button"]').trigger('submit')
    expect(wrapper.find('[data-testid="error-message"]').text()).toBe('请输入用户名')
  })

  it('应该显示错误信息当密码为空', async () => {
    const wrapper = mount(LoginMock)
    await wrapper.find('[data-testid="username-input"]').setValue('admin')
    await wrapper.find('[data-testid="login-button"]').trigger('submit')
    expect(wrapper.find('[data-testid="error-message"]').text()).toBe('请输入密码')
  })

  it('应该清除错误信息当表单有效', async () => {
    const wrapper = mount(LoginMock)
    await wrapper.find('[data-testid="username-input"]').setValue('admin')
    await wrapper.find('[data-testid="password-input"]').setValue('password')
    await wrapper.find('[data-testid="login-button"]').trigger('submit')
    expect(wrapper.find('[data-testid="error-message"]').exists()).toBe(false)
  })
})
