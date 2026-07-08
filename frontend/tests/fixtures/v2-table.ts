/**
 * V2Table 测试 mock 数据夹具
 * 规则 6：测试 mock 数据禁止硬编码在测试用例中，统一抽取到 fixtures
 */
import type { ColumnDef } from '@/components/V2Table/types'

/** 基础测试行数据类型 */
export interface TestRow {
  id: number
  name: string
  status?: string
  value?: string
}

/// 单行数据（id + name）
export const singleRow: TestRow[] = [{ id: 1, name: 'A' }]

/// 双行数据（用于渲染行数测试）
export const dualRows: TestRow[] = [
  { id: 1, name: 'Item 1' },
  { id: 2, name: 'Item 2' },
]

/// 带状态的单行数据
export const statusRow: TestRow[] = [{ id: 1, name: 'A', status: 'OK' }]

/// 基础名称列定义
export const nameColumns: ColumnDef<TestRow>[] = [{ key: 'name', title: '名称', width: 200 }]

/// 含 ID + 名称 + 状态的列定义（含 fixed/align 字段）
export const fullColumns: ColumnDef<TestRow>[] = [
  { key: 'id', title: 'ID', width: 80 },
  { key: 'name', title: '名称', width: 200, align: 'center' },
  { key: 'status', title: '状态', width: 100, fixed: 'left' },
]

/// 仅 ID 列定义（width 缺省场景）
export const idOnlyColumns: ColumnDef<TestRow>[] = [{ key: 'id', title: 'ID' }]

/// 带格式化器的列定义
export const formatterColumns: ColumnDef<TestRow>[] = [
  {
    key: 'value',
    title: '值',
    width: 100,
    formatter: (row) => `f-${row.value}`,
  },
]
