/**
 * useRcp - 工艺优化（染色配方）tab 业务逻辑 composable
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 4 个 tab）
 */
import { ref } from 'vue'
import { ElMessage } from 'element-plus'
import { optimizeRecipe, type RecipeOptParams } from '@/api/advanced'
import type { ApiResponse } from '@/types/api'

/**
 * 配方表单数据结构
 */
export interface RecipeFormData {
  color_no: string
  fabric_type: string
  dye_type: string
  color_name: string
  k: number
}

/**
 * 工艺优化 tab 业务逻辑封装
 * 染色工艺参数智能推荐（A2-1）
 */
export function useRcp() {
  const recipeForm = ref<RecipeFormData>({
    color_no: '',
    fabric_type: '棉',
    dye_type: '',
    color_name: '',
    k: 5,
  })
  const recipeLoading = ref(false)
  const recipeResult = ref<unknown>(null)

  /**
   * 执行工艺优化推荐
   */
  const runRecipeOptimization = async () => {
    if (!recipeForm.value.color_no.trim()) {
      ElMessage.warning('请输入色号')
      return
    }
    if (!recipeForm.value.fabric_type) {
      ElMessage.warning('请选择布类')
      return
    }
    recipeLoading.value = true
    try {
      const payload: RecipeOptParams = {
        color_no: recipeForm.value.color_no.trim(),
        fabric_type: recipeForm.value.fabric_type,
        k: recipeForm.value.k,
      }
      if (recipeForm.value.dye_type && recipeForm.value.dye_type.trim()) {
        payload.dye_type = recipeForm.value.dye_type.trim()
      }
      if (recipeForm.value.color_name && recipeForm.value.color_name.trim()) {
        payload.color_name = recipeForm.value.color_name.trim()
      }
      const res = await optimizeRecipe(payload) as ApiResponse<unknown>
      recipeResult.value = res.data!
      ElMessage.success('推荐生成完成')
    } catch (e: unknown) {
      ElMessage.error((e instanceof Error ? e.message : '') || '推荐失败')
    } finally {
      recipeLoading.value = false
    }
  }

  return {
    recipeForm,
    recipeLoading,
    recipeResult,
    runRecipeOptimization,
  }
}
