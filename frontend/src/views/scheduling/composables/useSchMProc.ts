/**
 * useSchMProc.ts - 排产管理自动排程流程 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/index.vue）
 * 提供排产自动排程的执行流程（参数构造 + API 调用 + 冲突更新）
 * 行为完全保持一致（仅结构重构）
 */
import { reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { schedulingApi, type ConflictItem, type SchedulingParams } from '@/api/scheduling'

// v11 批次 181 P2-1 修复：定义 deps 类型替代 any
// 注：useSchM 返回 reactive 对象，字段会被自动解包
// - 对象/数组类型（scheduleParams, stats）传入解包后的引用，修改字段可反映到原对象
// - 值类型（dateRange）传入解包后的值，仅用于读取
// - 需要重新赋值的字段（conflictList）使用 setter 函数，确保修改反映到原 reactive 对象
interface SchMStats {
  pending: number
  scheduled: number
  running: number
  conflicts: number
}

interface SchMDeps {
  fetchTasks: () => Promise<void>
  dateRange: [Date, Date] | null
  scheduleParams: SchedulingParams
  setConflictList: (v: ConflictItem[]) => void
  stats: SchMStats
}

/**
 * 排产管理自动排程流程 composable
 */
export function useSchMProc(deps: SchMDeps) {
  // 自动排程进行中
  const scheduling = ref(false)

  /** 准备排程参数（基于日期范围或默认 +30 天） */
  const prepareScheduleParams = () => {
    if (deps.dateRange && deps.dateRange.length === 2) {
      deps.scheduleParams.start_date = deps.dateRange[0].toISOString().split('T')[0]
      deps.scheduleParams.end_date = deps.dateRange[1].toISOString().split('T')[0]
    } else {
      const today = new Date()
      const end = new Date()
      end.setDate(end.getDate() + 30)
      deps.scheduleParams.start_date = today.toISOString().split('T')[0]
      deps.scheduleParams.end_date = end.toISOString().split('T')[0]
    }
  }

  /** 执行自动排程 */
  const handleAutoSchedule = async () => {
    scheduling.value = true
    try {
      prepareScheduleParams()
      const res = await schedulingApi.autoSchedule(deps.scheduleParams)
      const result = res.data
      // 安全检查：防止后端返回 data 为 null 时崩溃
      if (!result) return
      ElMessage.success(`排程完成: ${result.scheduled_count} 个任务, ${result.conflict_count} 个冲突`)
      if (result.conflict_count > 0) {
        // v11 批次 181 P2-1 修复：使用 setter 函数正确更新 reactive 对象的 conflictList
        deps.setConflictList(result.conflicts)
        deps.stats.conflicts = result.conflict_count
      }
      await deps.fetchTasks()
    } catch (error: unknown) {
      // v11 批次 181 P2-1 修复：catch (error: any) 改为 catch (error: unknown) + 类型守卫
      const errMsg = error instanceof Error ? error.message : String(error)
      ElMessage.error(errMsg || '自动排程失败')
    } finally {
      scheduling.value = false
    }
  }

  // 使用 reactive 包装
  return reactive({
    scheduling,
    handleAutoSchedule,
  })
}
