<script setup lang="ts">
/**
 * RcpPanel - 工艺优化（染色配方）tab 视图组件
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 4 个 tab）
 * 染色工艺参数智能推荐（A2-1）
 * 数据与函数全部由父组件通过 props 传入
 * P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
 */
import { ref, watch, nextTick } from 'vue'
import type { RecipeFormData, RecipeResult } from '../composables/useRcp'

const props = defineProps<{
  // 工艺推荐表单（由父组件管理，子组件通过 emit('update:recipeForm') 回写）
  recipeForm: RecipeFormData
  recipeLoading: boolean
  recipeResult: RecipeResult | null
  runRecipeOptimization: () => Promise<void>
}>()

const emit = defineEmits<{
  // 整体回写表单
  (e: 'update:recipeForm', form: RecipeFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<RecipeFormData>({ ...props.recipeForm })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.recipeForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    localForm.value = { ...newForm }
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)

// 本地变化时通知父组件
watch(
  localForm,
  (newForm) => {
    if (syncing) return
    syncing = true
    emit('update:recipeForm', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

<template>
  <div class="page-header">
    <h2 class="page-title">染色工艺参数智能推荐</h2>
  </div>

  <el-row :gutter="20">
    <el-col :span="8">
      <el-card shadow="hover" class="mb-20">
        <template #header><div class="card-header">推荐条件</div></template>
        <el-form :model="localForm" label-width="100px" aria-label="染色工艺推荐条件表单">
          <el-form-item label="色号" required>
            <el-input v-model="localForm.color_no" placeholder="如 BL-301" />
          </el-form-item>
          <el-form-item label="布类" required>
            <el-select
              v-model="localForm.fabric_type"
              placeholder="请选择布类"
              style="width: 100%"
            >
              <el-option label="棉" value="棉" />
              <el-option label="涤纶" value="涤纶" />
              <el-option label="丝绸" value="丝绸" />
              <el-option label="羊毛" value="羊毛" />
              <el-option label="化纤" value="化纤" />
            </el-select>
          </el-form-item>
          <el-form-item label="染料类型">
            <el-input
              v-model="localForm.dye_type"
              placeholder="可选，如 活性染料"
            />
          </el-form-item>
          <el-form-item label="颜色名称">
            <el-input
              v-model="localForm.color_name"
              placeholder="可选，如 宝蓝"
            />
          </el-form-item>
          <el-form-item label="K 值">
            <el-input-number
              v-model="localForm.k"
              :min="0"
              :max="20"
              :step="1"
              style="width: 100%"
            />
          </el-form-item>
          <el-form-item>
            <el-button
              type="primary"
              :loading="recipeLoading"
              @click="runRecipeOptimization"
              >生成推荐</el-button
            >
          </el-form-item>
        </el-form>
      </el-card>
    </el-col>

    <el-col :span="16">
      <el-card shadow="hover" class="mb-20">
        <template #header>
          <div class="card-header">推荐结果</div>
        </template>
        <el-empty
          v-if="!recipeResult"
          description="请填写色号与布类后生成推荐"
        />
        <div v-else>
          <el-descriptions :column="2" border>
            <el-descriptions-item label="推荐温度">
              {{ recipeResult.recommended_params.temperature }} °C
            </el-descriptions-item>
            <el-descriptions-item label="推荐时间">
              {{ recipeResult.recommended_params.time_minutes }} 分钟
            </el-descriptions-item>
            <el-descriptions-item label="推荐 pH">
              {{ recipeResult.recommended_params.ph_value }}
            </el-descriptions-item>
            <el-descriptions-item label="推荐浴比">
              1 : {{ recipeResult.recommended_params.liquor_ratio }}
            </el-descriptions-item>
            <el-descriptions-item label="置信度">
              {{ Math.round(recipeResult.confidence * 100) }}%
            </el-descriptions-item>
            <el-descriptions-item label="相似案例数">
              {{ recipeResult.similar_cases }}
            </el-descriptions-item>
            <el-descriptions-item label="推荐来源">
              <el-tag
                :type="recipeResult.source === 'knn' ? 'success' : 'info'"
                size="small"
              >
                {{
                  recipeResult.source === 'knn' ? 'k-NN 匹配' : '退化兜底'
                }}
              </el-tag>
            </el-descriptions-item>
          </el-descriptions>

          <el-alert
            class="mt-12"
            :title="recipeResult.reason"
            type="info"
            :closable="false"
            show-icon
          />

          <h4 class="mb-10" style="margin-top: 16px">相似候选案例</h4>
          <el-table
            v-if="recipeResult.candidates && recipeResult.candidates.length > 0"
            :data="recipeResult.candidates"
            stripe
            size="small"
            border
            aria-label="相似候选案例列表"
          >
            <el-table-column prop="recipe_no" label="配方编号" width="160" />
            <el-table-column prop="color_no" label="色号" width="120" />
            <el-table-column prop="fabric_type" label="布类" width="100" />
            <el-table-column prop="dye_type" label="染料" width="120" />
            <el-table-column label="温度" width="80">
              <template #default="{ row }">
                {{ row.temperature ?? '-' }} °C
              </template>
            </el-table-column>
            <el-table-column label="时间" width="80">
              <template #default="{ row }">
                {{ row.time_minutes ?? '-' }} 分
              </template>
            </el-table-column>
            <el-table-column label="pH" width="80">
              <template #default="{ row }">
                {{ row.ph_value ?? '-' }}
              </template>
            </el-table-column>
            <el-table-column label="浴比" width="80">
              <template #default="{ row }">
                1:{{ row.liquor_ratio ?? '-' }}
              </template>
            </el-table-column>
            <el-table-column label="相似度" width="100">
              <template #default="{ row }">
                {{ Math.round(row.similarity * 100) }}%
              </template>
            </el-table-column>
          </el-table>
          <el-empty
            v-else
            description="暂无候选案例"
            :image-size="60"
          />
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>
