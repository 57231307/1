<script setup lang="ts">
/**
 * AdvancedRecipePanel - 工艺优化（染色配方）tab 视图组件
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 4 个 tab）
 * 染色工艺参数智能推荐（A2-1）
 * 数据与函数全部由父组件通过 props 传入
 * P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
 */
import { ref, watch, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import type { RecipeFormData, RecipeResult } from '../composables/useRcp'

const props = defineProps<{
  // 工艺推荐表单（由父组件管理，子组件通过 emit('update:recipeForm') 回写）
  recipeForm: RecipeFormData
  recipeLoading: boolean
  recipeResult: RecipeResult | null
  runRecipeOptimization: () => Promise<void>
}>()

const { t } = useI18n({ useScope: 'global' })

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

function sourceLabel(source: string): string {
  return source === 'knn' ? t('advancedModule.recipe.sourceKnn') : t('advancedModule.recipe.sourceFallback')
}
</script>

<template>
  <div class="page-header">
    <h2 class="page-title">{{ $t('advancedModule.recipe.title') }}</h2>
  </div>

  <el-row :gutter="20">
    <el-col :span="8">
      <el-card shadow="hover" class="mb-20">
        <template #header><div class="card-header">{{ $t('advancedModule.recipe.conditions') }}</div></template>
        <el-form :model="localForm" label-width="100px" aria-label="染色工艺推荐条件表单">
          <el-form-item :label="$t('advancedModule.recipe.colorNo')" required>
            <el-input v-model="localForm.color_no" :placeholder="$t('advancedModule.recipe.colorNoPlaceholder')" />
          </el-form-item>
          <el-form-item :label="$t('advancedModule.recipe.fabricType')" required>
            <el-select
              v-model="localForm.fabric_type"
              :placeholder="$t('advancedModule.recipe.fabricPlaceholder')"
              style="width: 100%"
            >
              <el-option :label="$t('advancedModule.recipe.fabricCotton')" value="棉" />
              <el-option :label="$t('advancedModule.recipe.fabricPolyester')" value="涤纶" />
              <el-option :label="$t('advancedModule.recipe.fabricSilk')" value="丝绸" />
              <el-option :label="$t('advancedModule.recipe.fabricWool')" value="羊毛" />
              <el-option :label="$t('advancedModule.recipe.fabricSynthetic')" value="化纤" />
            </el-select>
          </el-form-item>
          <el-form-item :label="$t('advancedModule.recipe.dyeType')">
            <el-input
              v-model="localForm.dye_type"
              :placeholder="$t('advancedModule.recipe.dyeTypePlaceholder')"
            />
          </el-form-item>
          <el-form-item :label="$t('advancedModule.recipe.colorName')">
            <el-input
              v-model="localForm.color_name"
              :placeholder="$t('advancedModule.recipe.colorNamePlaceholder')"
            />
          </el-form-item>
          <el-form-item :label="$t('advancedModule.recipe.kValue')">
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
              >{{ $t('advancedModule.recipe.generate') }}</el-button
            >
          </el-form-item>
        </el-form>
      </el-card>
    </el-col>

    <el-col :span="16">
      <el-card shadow="hover" class="mb-20">
        <template #header>
          <div class="card-header">{{ $t('advancedModule.recipe.result') }}</div>
        </template>
        <el-empty
          v-if="!recipeResult"
          :description="$t('advancedModule.recipe.empty')"
        />
        <div v-else>
          <el-descriptions :column="2" border>
            <el-descriptions-item :label="$t('advancedModule.recipe.recTemperature')">
              {{ recipeResult.recommended_params.temperature }} °C
            </el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.recipe.recTime')">
              {{ recipeResult.recommended_params.time_minutes }} {{ $t('advancedModule.recipe.unitMinutes') }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.recipe.recPh')">
              {{ recipeResult.recommended_params.ph_value }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.recipe.recLiquorRatio')">
              1 : {{ recipeResult.recommended_params.liquor_ratio }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.recipe.confidence')">
              {{ Math.round(recipeResult.confidence * 100) }}%
            </el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.recipe.similarCases')">
              {{ recipeResult.similar_cases }}
            </el-descriptions-item>
            <el-descriptions-item :label="$t('advancedModule.recipe.recSource')">
              <el-tag
                :type="recipeResult.source === 'knn' ? 'success' : 'info'"
                size="small"
              >
                {{ sourceLabel(recipeResult.source) }}
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

          <h4 class="mb-10" style="margin-top: 16px">{{ $t('advancedModule.recipe.similarCandidates') }}</h4>
          <el-table
            v-if="recipeResult.candidates && recipeResult.candidates.length > 0"
            :data="recipeResult.candidates"
            stripe
            size="small"
            border
            aria-label="相似候选案例列表"
          >
            <el-table-column prop="recipe_no" :label="$t('advancedModule.recipe.colRecipeNo')" width="160" />
            <el-table-column prop="color_no" :label="$t('advancedModule.recipe.colColorNo')" width="120" />
            <el-table-column prop="fabric_type" :label="$t('advancedModule.recipe.colFabricType')" width="100" />
            <el-table-column prop="dye_type" :label="$t('advancedModule.recipe.colDyeType')" width="120" />
            <el-table-column :label="$t('advancedModule.recipe.colTemperature')" width="80">
              <template #default="{ row }">
                {{ row.temperature ?? '-' }} °C
              </template>
            </el-table-column>
            <el-table-column :label="$t('advancedModule.recipe.colTime')" width="80">
              <template #default="{ row }">
                {{ row.time_minutes ?? '-' }} {{ $t('advancedModule.recipe.unitMinutesShort') }}
              </template>
            </el-table-column>
            <el-table-column :label="$t('advancedModule.recipe.colPh')" width="80">
              <template #default="{ row }">
                {{ row.ph_value ?? '-' }}
              </template>
            </el-table-column>
            <el-table-column :label="$t('advancedModule.recipe.colLiquorRatio')" width="80">
              <template #default="{ row }">
                1:{{ row.liquor_ratio ?? '-' }}
              </template>
            </el-table-column>
            <el-table-column :label="$t('advancedModule.recipe.colSimilarity')" width="100">
              <template #default="{ row }">
                {{ Math.round(row.similarity * 100) }}%
              </template>
            </el-table-column>
          </el-table>
          <el-empty
            v-else
            :description="$t('advancedModule.recipe.emptyNoCandidates')"
            :image-size="60"
          />
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>
