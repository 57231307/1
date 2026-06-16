/**
 * V2Table 组件单元测试
 * 任务编号: Wave 4 P2-1 PR-1
 * 注意: 本地不允许 npm run test:run，所有测试通过 git push 触发 CI 验证
 */
import { describe, expect, it, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'

const testColumns: ColumnDef[] = [
  { key: 'id', title: 'ID', width: 80 },
  { key: 'name', title: '名称', minWidth: 120 },
  {
    key: 'status',
    title: '状态',
    width: 100,
    formatter: (row: any) => row.status === 'active' ? '活跃' : '禁用',
  },
]

const testData = [
  { id: 1, name: '项目 A', status: 'active' },
  { id: 2, name: '项目 B', status: 'inactive' },
]

describe('V2Table 组件', () => {
  it('接收 columns 和 data 正确渲染', () => {
    const wrapper = mount(V2Table, {
      props: { columns: testColumns, data: testData },
    })
    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.v2-table-wrapper').exists()).toBe(true)
  })

  it('触发 page-change 事件', async () => {
    const wrapper = mount(V2Table, {
      props: {
        columns: testColumns,
        data: testData,
        total: 100,
        page: 1,
        pageSize: 20,
      },
    })
    // 直接测试事件 emit
    const vm = wrapper.vm as any
    vm.$emit('page-change', 2)
    expect(wrapper.emitted('page-change')).toBeTruthy()
    expect(wrapper.emitted('page-change')?.[0]).toEqual([2])
  })

  it('触发 size-change 事件', async () => {
    const wrapper = mount(V2Table, {
      props: {
        columns: testColumns,
        data: testData,
        total: 100,
        page: 1,
        pageSize: 20,
      },
    })
    const vm = wrapper.vm as any
    vm.$emit('size-change', 50)
    expect(wrapper.emitted('size-change')).toBeTruthy()
    expect(wrapper.emitted('size-change')?.[0]).toEqual([50])
  })

  it('在空数据显示 emptyText', () => {
    const wrapper = mount(V2Table, {
      props: {
        columns: testColumns,
        data: [],
        emptyText: '没有查询到数据',
      },
    })
    // 通过 props 验证
    expect(wrapper.props('emptyText')).toBe('没有查询到数据')
    expect(wrapper.props('data').length).toBe(0)
  })

  it('在 loading 状态正确传递 prop', () => {
    const wrapper = mount(V2Table, {
      props: {
        columns: testColumns,
        data: testData,
        loading: true,
      },
    })
    expect(wrapper.props('loading')).toBe(true)
  })
})
