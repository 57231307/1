/**
 * useSchGProc.ts - 排产甘特图业务流程 composable
 * 任务编号: P14 批 2 I-3 第 2 批（拆分原 scheduling/gantt.vue）
 * 提供排产自动排程的执行 + 冲突展示流程
 * 行为完全保持一致（仅结构重构）
 */
import { reactive, ref } from 'vue'
import { ElMessage } from 'element-plus'
import { schedulingApi, type SchedulingParams, type ConflictItem } from '@/api/scheduling'

// v11 批次 181 P2-1 修复：定义 deps 类型替代 any
// 注：useSchG 返回 reactive 对象，字段会被自动解包
// - 对象类型（scheduleForm）传入解包后的引用，修改字段可反映到原对象
// - 需要重新赋值的字段（conflictList, conflictDialogVisible, autoScheduleDialogVisible）
//   使用 setter 函数，确保修改反映到原 reactive 对象
interface SchGDeps {
  fetchGanttData: () => Promise<void>
  scheduleForm: SchedulingParams
  setAutoScheduleDialogVisible: (v: boolean) => void
  setConflictList: (v: ConflictItem[]) => void
  setConflictDialogVisible: (v: boolean) => void
}

/**
 * 排产自动排程流程 composable
 * 集中管理"执行自动排程"和"展示冲突列表"两个流程
 */
export function useSchGProc(deps: SchGDeps) {
  // 自动排程进行中
  const scheduling = ref(false)

  /** 确认执行自动排程 */
  const confirmAutoSchedule = async () => {
    scheduling.value = true
    try {
      const res = await schedulingApi.autoSchedule(deps.scheduleForm)
      const result = res.data!
      ElMessage.success(
        `排程完成: ${result.scheduled_count} 个任务已排程, ${result.conflict_count} 个冲突`
      )
      // v11 批次 181 P2-1 修复：使用 setter 函数正确更新 reactive 对象的字段
      deps.setAutoScheduleDialogVisible(false)
      if (result.conflict_count > 0) {
        deps.setConflictList(result.conflicts)
        deps.setConflictDialogVisible(true)
      }
      await deps.fetchGanttData()
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
    confirmAutoSchedule,
  })
}
