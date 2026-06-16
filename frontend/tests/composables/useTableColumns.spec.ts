import { describe, it, expect } from 'vitest'
import { ref, nextTick } from 'vue'
import { useTableColumns } from '@/composables/useTableColumns'
import type { ColumnDef } from '@/components/V2Table/index.vue'

describe('useTableColumns', () => {
  it('返回响应式 columns 数组', () => {
    const defs: ColumnDef[] = [
      { key: 'id', label: 'ID', width: 80 },
      { key: 'name', label: '名称', width: 200 }
    ]
    const { columns } = useTableColumns(defs)
    expect(columns.value).toEqual(defs)
  })

  it('支持 ref 形式的 defs', async () => {
    const defs = ref<ColumnDef[]>([{ key: 'id', label: 'ID', width: 80 }])
    const { columns } = useTableColumns(defs)
    expect(columns.value).toHaveLength(1)

    defs.value = [...defs.value, { key: 'name', label: '名称', width: 200 }]
    await nextTick()
    expect(columns.value).toHaveLength(2)
  })

  it('运行时 addColumn 添加列', () => {
    const { columns, addColumn } = useTableColumns([])
    addColumn({ key: 'new', label: '新列', width: 100 })
    expect(columns.value).toHaveLength(1)
    expect(columns.value[0].key).toBe('new')
  })

  it('运行时 removeColumn 删除列', () => {
    const { columns, removeColumn } = useTableColumns([
      { key: 'id', label: 'ID', width: 80 },
      { key: 'name', label: '名称', width: 200 }
    ])
    removeColumn('id')
    expect(columns.value).toHaveLength(1)
    expect(columns.value[0].key).toBe('name')
  })
})
