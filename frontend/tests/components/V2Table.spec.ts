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
})
