/**
 * V2Table 组件单元测试
 * 任务编号: Wave 4 P2-3（重做对齐 P2-1 API）
 * 规则 6：mock 数据统一从 fixtures/v2-table.ts 引用，禁止内联硬编码
 */
import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import V2Table from '@/components/V2Table/index.vue'
import {
  singleRow,
  dualRows,
  statusRow,
  nameColumns,
  fullColumns,
  idOnlyColumns,
  formatterColumns,
} from '../fixtures/v2-table'

describe('V2Table', () => {
  it('渲染空数据时显示「暂无数据」', () => {
    const wrapper = mount(V2Table, {
      props: { data: [], columns: [] },
    })
    expect(wrapper.text()).toContain('暂无数据')
  })

  it('渲染正常数据时挂载 el-table-v2', () => {
    const wrapper = mount(V2Table, {
      props: { data: dualRows, columns: nameColumns },
    })
    expect(wrapper.findAllComponents({ name: 'ElTableV2' }).length).toBe(1)
  })

  it('将 ColumnDef 转换为 el-table-v2 列定义（title/dataKey/width 字段）', () => {
    const wrapper = mount(V2Table, {
      props: { data: statusRow, columns: fullColumns },
    })
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    expect(tableV2.props('columns')).toHaveLength(3)
    expect(tableV2.props('columns')[0]).toMatchObject({
      key: 'id',
      title: 'ID',
      dataKey: 'id',
      width: 80,
    })
  })

  it('width 缺省时回退默认 150', () => {
    const wrapper = mount(V2Table, {
      props: { data: singleRow, columns: idOnlyColumns },
    })
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    expect(tableV2.props('columns')[0].width).toBe(150)
  })

  it('通过 rowEventHandlers 接入行点击事件（仅透传 rowData）', async () => {
    const wrapper = mount(V2Table, {
      props: { data: singleRow, columns: nameColumns },
    })
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    /// el-table-v2 通过 rowEventHandlers prop 接入行点击，验证 prop 已透传
    expect(tableV2.props('rowEventHandlers')).toBeTruthy()
    expect(typeof tableV2.props('rowEventHandlers').onClick).toBe('function')
    /// 模拟点击：直接调用 onClick 回调（el-table-v2 内部会传入 RowEventHandlerParams）
    const row = singleRow[0]
    tableV2.props('rowEventHandlers').onClick({ rowData: row, rowIndex: 0, event: new MouseEvent('click') })
    expect(wrapper.emitted('row-click')).toBeTruthy()
    /// row-click 仅传 rowData（P2-1 风格）
    expect(wrapper.emitted('row-click')![0]).toEqual([row])
  })

  it('formatter 列定义正确生成 cellRenderer', () => {
    const wrapper = mount(V2Table, {
      props: { data: singleRow, columns: formatterColumns },
    })
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    /// formatter 列应生成 cellRenderer 函数
    expect(typeof tableV2.props('columns')[0].cellRenderer).toBe('function')
  })

  it('未传 total 时不渲染分页', () => {
    const wrapper = mount(V2Table, {
      props: { data: singleRow, columns: idOnlyColumns },
    })
    expect(wrapper.find('.v2-table-pagination').exists()).toBe(false)
  })

  it('传 total 时渲染 el-pagination', () => {
    const wrapper = mount(V2Table, {
      props: {
        data: singleRow,
        columns: idOnlyColumns,
        total: 100,
        page: 1,
        pageSize: 20,
      },
    })
    expect(wrapper.find('.v2-table-pagination').exists()).toBe(true)
  })

  it('触发 page-change 事件（P2-1 风格）', async () => {
    const wrapper = mount(V2Table, {
      props: {
        data: singleRow,
        columns: idOnlyColumns,
        total: 100,
        page: 1,
        pageSize: 20,
      },
    })
    const pagination = wrapper.findComponent({ name: 'ElPagination' })
    await pagination.vm.$emit('current-change', 2)
    expect(wrapper.emitted('page-change')).toBeTruthy()
    expect(wrapper.emitted('page-change')![0]).toEqual([2])
  })

  it('触发 size-change 事件（P2-1 风格）', async () => {
    const wrapper = mount(V2Table, {
      props: {
        data: singleRow,
        columns: idOnlyColumns,
        total: 100,
        page: 1,
        pageSize: 20,
      },
    })
    const pagination = wrapper.findComponent({ name: 'ElPagination' })
    await pagination.vm.$emit('size-change', 50)
    expect(wrapper.emitted('size-change')).toBeTruthy()
    expect(wrapper.emitted('size-change')![0]).toEqual([50])
  })

  it('estimatedRowHeight prop 透传给 el-table-v2', () => {
    const wrapper = mount(V2Table, {
      props: {
        data: singleRow,
        columns: idOnlyColumns,
        estimatedRowHeight: 40,
      },
    })
    const tableV2 = wrapper.findComponent({ name: 'ElTableV2' })
    expect(tableV2.props('estimatedRowHeight')).toBe(40)
  })
})
