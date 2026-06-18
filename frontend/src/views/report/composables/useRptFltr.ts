/**
 * useRptFltr - 报表筛选条件 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 report/templates.vue）
 */
import { ref } from 'vue'
import type { ReportFilterCondition } from '@/api/report-enhanced'

/**
 * 筛选条件操作符选项
 */
const operatorOptions = [
  { label: '等于', value: 'eq' },
  { label: '不等于', value: 'ne' },
  { label: '大于', value: 'gt' },
  { label: '小于', value: 'lt' },
  { label: '大于等于', value: 'gte' },
  { label: '小于等于', value: 'lte' },
  { label: '包含', value: 'contains' },
  { label: '在...中', value: 'in' },
  { label: '区间', value: 'between' },
]

/**
 * 报表筛选条件 composable
 */
export function useRptFltr() {
  const filterConditions = ref<ReportFilterCondition[]>([])

  /**
   * 重置筛选条件为初始值
   */
  const reset = () => {
    filterConditions.value = []
  }

  const addFilter = () => {
    filterConditions.value.push({
      field: '',
      operator: 'eq',
      value: '',
    })
  }

  const removeFilter = (index: number) => {
    filterConditions.value.splice(index, 1)
  }

  return {
    filterConditions,
    operatorOptions,
    reset,
    addFilter,
    removeFilter,
  }
}
