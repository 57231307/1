import { computed, ref, type Ref } from 'vue'
// 复用 V2Table 中定义的 ColumnDef 类型（单一来源原则）
export type { ColumnDef } from '@/components/V2Table/types'
import type { ColumnDef } from '@/components/V2Table/types'

/**
 * 响应式表格列定义 composable
 * 支持数组或 ref 输入，提供 addColumn / removeColumn 运行时增删 API
 *
 * 说明：P2-3 重做后 ColumnDef 改用 title 字段（对齐 P2-1），不再使用 label
 */
export function useTableColumns(defs: ColumnDef[] | Ref<ColumnDef[]>) {
  const sourceRef = Array.isArray(defs) ? ref(defs) : defs

  const columns = computed<ColumnDef[]>(() => sourceRef.value)

  function addColumn(def: ColumnDef) {
    sourceRef.value = [...sourceRef.value, def]
  }

  function removeColumn(key: string) {
    sourceRef.value = sourceRef.value.filter(c => c.key !== key)
  }

  return { columns, addColumn, removeColumn }
}
