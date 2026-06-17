<!--
  AdvancedQualityTab.vue - 高级质量预测 Tab
  来源：原 advanced/index.vue 中 quality tab
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <div class="page-header">
      <h2 class="page-title">质量预测与缺陷分析</h2>
    </div>

    <el-row :gutter="20">
      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">质量预测</div>
          </template>
          <el-form :model="qualityForm" label-width="100px">
            <el-form-item label="产品">
              <el-input
                :model-value="qualityForm.product_name"
                @update:model-value="(v: any) => emit('update:qualityForm', { ...qualityForm, product_name: v })"
                placeholder="如 棉布"
              />
            </el-form-item>
            <el-form-item label="批次">
              <el-input
                :model-value="qualityForm.batch_no"
                @update:model-value="(v: any) => emit('update:qualityForm', { ...qualityForm, batch_no: v })"
                placeholder="如 B20240615001"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="qualityLoading" @click="emit('predict')"
                >开始预测</el-button
              >
            </el-form-item>
          </el-form>
          <div v-if="qualityResult" class="mt-20">
            <h4>预测结果</h4>
            <el-divider />
            <el-descriptions :column="2" border>
              <el-descriptions-item label="预测质量等级">{{
                qualityResult.quality_level
              }}</el-descriptions-item>
              <el-descriptions-item label="预测合格率"
                >{{ qualityResult.pass_rate }}%</el-descriptions-item
              >
              <el-descriptions-item label="色差 (ΔE)"
                >{{ qualityResult.color_deviation }}</el-descriptions-item
              >
              <el-descriptions-item label="色牢度">{{
                qualityResult.color_fastness
              }}</el-descriptions-item>
            </el-descriptions>
          </div>
        </el-card>
      </el-col>

      <el-col :span="12">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">缺陷分析</div>
          </template>
          <el-button type="primary" :loading="false" @click="emit('analyze-defect')"
            >分析缺陷数据</el-button
          >
          <div v-if="defectStats" class="mt-20">
            <h4>缺陷统计</h4>
            <el-divider />
            <el-descriptions :column="1" border>
              <el-descriptions-item label="总缺陷数">{{ defectStats.total }}</el-descriptions-item>
              <el-descriptions-item label="色差">{{ defectStats.color_deviation }}</el-descriptions-item>
              <el-descriptions-item label="色渍">{{ defectStats.color_stain }}</el-descriptions-item>
              <el-descriptions-item label="色花">{{ defectStats.color_flower }}</el-descriptions-item>
              <el-descriptions-item label="皱条">{{ defectStats.crinkle }}</el-descriptions-item>
              <el-descriptions-item label="其他">{{ defectStats.other }}</el-descriptions-item>
            </el-descriptions>
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  qualityForm: any
  qualityLoading: boolean
  qualityResult: any
  defectStats: any
  formatMoney: (amount: number) => string
}>()

const emit = defineEmits<{
  predict: []
  'analyze-defect': []
  'update:qualityForm': [value: any]
}>()
</script>
