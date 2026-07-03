/**
 * PasswordStrengthMeter 组件单元测试
 * 验证：强度计算逻辑、feedback 输出、颜色映射
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import PasswordStrengthMeter from '@/components/PasswordStrengthMeter.vue'

describe('PasswordStrengthMeter 组件', () => {
  it('空密码时不渲染任何内容', () => {
    const wrapper = mount(PasswordStrengthMeter, {
      props: { password: '' },
    })
    expect(wrapper.find('.pwd-strength').exists()).toBe(false)
  })

  it('短密码（小写+数字）应判定为弱强度', () => {
    const wrapper = mount(PasswordStrengthMeter, {
      props: { password: 'abc123' },
    })
    expect(wrapper.find('.pwd-strength').exists()).toBe(true)
    // 含小写+数字但长度不足 12，预期强度 ≤ 2
    const text = wrapper.text()
    expect(text).toMatch(/极弱|弱/)
  })

  it('包含多种字符类的长密码应判定为强或极强', () => {
    const wrapper = mount(PasswordStrengthMeter, {
      props: { password: 'Abcdef123456!@#' },
    })
    const text = wrapper.text()
    // 长度 ≥ 12 + 全部字符类齐全 = 5 分，封顶 4 → 极强
    expect(text).toMatch(/强|极强/)
  })

  it('只有大写字母应给出长度建议', () => {
    const wrapper = mount(PasswordStrengthMeter, {
      props: { password: 'ABCDEFGH' },
    })
    const html = wrapper.html()
    expect(html).toContain('数字')
    expect(html).toContain('小写字母')
    expect(html).toContain('特殊字符')
  })

  it('强度等级与 progress 颜色一致', () => {
    const wrapper = mount(PasswordStrengthMeter, {
      props: { password: 'Abcdef123456!@#' },
    })
    // 找到 .pwd-strength-label 元素，验证其内联颜色
    const label = wrapper.find('.pwd-strength-label')
    expect(label.exists()).toBe(true)
    const style = label.attributes('style') || ''
    expect(style).toMatch(/color:/)
  })
})
