/**
 * V2Table 组件单元测试
 * 任务编号: Wave 4 P2-3（重做对齐 P2-1 API）
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import V2Table from '@/components/V2Table/index.vue'
import type { ColumnDef } from '@/components/V2Table/types'

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
    const columns: ColumnDef[] = [
      { key: 'name', title: '名称', width: 200 }
    ]
    const wrapper = mount(V2Table, {
      props: { data, columns }
    })
    expect(wrapper.findAllComponents({ name: 'ElTableV2' }).length).toBe(1)
  })

  it('将 ColumnDef 转换为 el-table-v2 列定义（title 字段）', () => {
    const columns: ColumnDef[] = [
      { key: 'id', title: 'ID', width: 80 },
      { key: 'name', title: '名称', width: 200, align: 'center' },
      { key: 'status', title: '状态', width: 100, fixed: 'left' }
    ]
    const wrapper = mount(V2Table, {
      props: { data: [{ id: 1, name: 'A', status: 'OK' }], columns }
    })
    // 验证 el-table-v2 接收到正确的 columns prop
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    expect(tableV2.props('columns')).toHaveLength(3)
    expect(tableV2.props('columns')[0]).toMatchObject({
      key: 'id',
      title: 'ID',
      dataKey: 'id',
      width: 80
    })
  })

  it('width 缺省时回退默认 150', () => {
    const columns: ColumnDef[] = [
      { key: 'name', title: '名称' } // width 缺省
    ]
    const wrapper = mount(V2Table, {
      props: { data: [{ id: 1, name: 'A' }], columns }
    })
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    expect(tableV2.props('columns')[0].width).toBe(150)
  })

  it('透传 row-click 事件（仅 rowData）', async () => {
    const wrapper = mount(V2Table, {
      props: {
        data: [{ id: 1, name: 'A' }],
        columns: [{ key: 'name', title: '名称', width: 200 }]
      }
    })
    await wrapper.findComponent({ name: 'ElTableV2' }).vm.$emit('row-click', {
      rowData: { id: 1 },
      column: { key: 'name' },
      event: new MouseEvent('click')
    })
    expect(wrapper.emitted('row-click')).toBeTruthy()
    // P2-1 风格：row-click 仅传 rowData（P2-3 旧版传 3 个参数）
    expect(wrapper.emitted('row-click')![0]).toEqual([{ id: 1 }])
  })

  it('renderCell 连续访问同 row+col 返回相同引用（WeakMap 缓存命中）', async () => {
    let callCount = 0
    const columns: ColumnDef[] = [
      {
        key: 'value',
        title: '值',
        width: 100,
        formatter: (row: any) => {
          callCount++
          return `formatted-${row.value}`
        }
      }
    ]
    const data = [{ id: 1, value: 'A' }]

    // 第一次 mount 触发 render
    const wrapper = mount(V2Table, { props: { data, columns } })
    const initialCount = callCount  // 应该是 1
    expect(initialCount).toBe(1)

    // 触发 el-table-v2 重新渲染（模拟滚动）
    await wrapper.setProps({ data: [...data, { id: 2, value: 'B' }] })
    // 新行触发 1 次 render，但第一行不会重复
    expect(callCount).toBe(2)

    // 恢复原数据
    await wrapper.setProps({ data })
    // 第一行不应重新 render
    expect(callCount).toBe(2)
  })

  it('暴露 window.__renderCellTotal 供性能测试采集', () => {
    const columns: ColumnDef[] = [
      {
        key: 'value',
        title: '值',
        width: 100,
        formatter: (row: any) => `f-${row.value}`
      }
    ]
    const data = [{ id: 1, value: 'A' }]

    mount(V2Table, { props: { data, columns } })

    // 验证 window 上有计数器
    expect((window as any).__renderCellTotal).toBeDefined()
    expect((window as any).__renderCellTotal.value).toBe(1)
  })

  it('未传 total 时不渲染分页', () => {
    const wrapper = mount(V2Table, {
      props: { data: [{ id: 1 }], columns: [{ key: 'id', title: 'ID' }] }
    })
    expect(wrapper.find('.v2-table-pagination').exists()).toBe(false)
  })

  it('传 total 时渲染 el-pagination', () => {
    const wrapper = mount(V2Table, {
      props: {
        data: [{ id: 1 }],
        columns: [{ key: 'id', title: 'ID' }],
        total: 100,
        page: 1,
        pageSize: 20
      }
    })
    expect(wrapper.find('.v2-table-pagination').exists()).toBe(true)
  })

  it('触发 page-change 事件（P2-1 风格）', async () => {
    const wrapper = mount(V2Table, {
      props: {
        data: [{ id: 1 }],
        columns: [{ key: 'id', title: 'ID' }],
        total: 100,
        page: 1,
        pageSize: 20
      }
    })
    const pagination = wrapper.findComponent({ name: 'ElPagination' })
    await pagination.vm.$emit('current-change', 2)
    expect(wrapper.emitted('page-change')).toBeTruthy()
    expect(wrapper.emitted('page-change')![0]).toEqual([2])
  })

  it('触发 size-change 事件（P2-1 风格）', async () => {
    const wrapper = mount(V2Table, {
      props: {
        data: [{ id: 1 }],
        columns: [{ key: 'id', title: 'ID' }],
        total: 100,
        page: 1,
        pageSize: 20
      }
    })
    const pagination = wrapper.findComponent({ name: 'ElPagination' })
    await pagination.vm.$emit('size-change', 50)
    expect(wrapper.emitted('size-change')).toBeTruthy()
    expect(wrapper.emitted('size-change')![0]).toEqual([50])
  })

  it('estimatedRowHeight prop 透传给 el-table-v2', () => {
    const wrapper = mount(V2Table, {
      props: {
        data: [{ id: 1 }],
        columns: [{ key: 'id', title: 'ID' }],
        estimatedRowHeight: 40
      }
    })
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    expect(tableV2.props('estimatedRowHeight')).toBe(40)
  })
})
