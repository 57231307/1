import { computed, ref, type Ref } from 'vue'
// 复用 V2Table 中定义的 ColumnDef 类型（单一来源原则）
export type { ColumnDef } from '@/components/V2Table/types'
import type { ColumnDef } from '@/components/V2Table/types'

/// 响应式表格列定义 composable，支持数组或 ref 输入，提供 addColumn / removeColumn 运行时增删 API
export function useTableColumns<T extends Record<string, unknown> = Record<string, unknown>>(
  defs: ColumnDef<T>[] | Ref<ColumnDef<T>[]>
) {
  const sourceRef = Array.isArray(defs) ? ref(defs) : defs

  const columns = computed<ColumnDef<T>[]>(() => sourceRef.value)

  function addColumn(def: ColumnDef<T>) {
    sourceRef.value = [...sourceRef.value, def]
  }

  function removeColumn(key: string) {
    sourceRef.value = sourceRef.value.filter(c => c.key !== key)
  }

  return { columns, addColumn, removeColumn }
}
