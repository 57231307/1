import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import V2Table, { type ColumnDef } from '@/components/V2Table/index.vue'

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

  it('将 ColumnDef 转换为 el-table-v2 列定义', () => {
    const columns: ColumnDef[] = [
      { key: 'id', label: 'ID', width: 80 },
      { key: 'name', label: '名称', width: 200, align: 'center' },
      { key: 'status', label: '状态', width: 100, fixed: 'left' }
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

  it('透传 row-click 事件', async () => {
    const wrapper = mount(V2Table, {
      props: {
        data: [{ id: 1, name: 'A' }],
        columns: [{ key: 'name', label: '名称', width: 200 }]
      }
    })
    await wrapper.findComponent({ name: 'ElTableV2' }).vm.$emit('row-click', {
      rowData: { id: 1 },
      column: { key: 'name' },
      event: new MouseEvent('click')
    })
    expect(wrapper.emitted('row-click')).toBeTruthy()
    expect(wrapper.emitted('row-click')![0]).toEqual([
      { id: 1 },
      { key: 'name' },
      expect.any(MouseEvent)
    ])
  })

  it('renderCell 连续访问同 row+col 返回相同引用（WeakMap 缓存命中）', async () => {
    let callCount = 0
    const columns: ColumnDef[] = [
      {
        key: 'value',
        label: '值',
        width: 100,
        formatter: (v: any) => {
          callCount++
          return `formatted-${v}`
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
})
