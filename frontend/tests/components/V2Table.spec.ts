import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import V2Table from '@/components/V2Table/index.vue'

describe('V2Table', () => {
  it('渲染空数据时显示「暂无数据」', () => {
    const wrapper = mount(V2Table, {
      props: { data: [], columns: [] }
    })
    expect(wrapper.text()).toContain('暂无数据')
  })

  it('渲染正常数据时显示行数', () => {
    const data = [
      { id: 1, name: 'Item 1' },
      { id: 2, name: 'Item 2' }
    ]
    const columns = [
      { key: 'name', label: '名称', width: 200 }
    ]
    const wrapper = mount(V2Table, {
      props: { data, columns }
    })
    expect(wrapper.findAllComponents({ name: 'ElTableV2' }).length).toBe(1)
  })
})
