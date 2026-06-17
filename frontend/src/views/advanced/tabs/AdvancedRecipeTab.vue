<!--
  AdvancedRecipeTab.vue - 高级工艺优化 Tab
  来源：原 advanced/index.vue 中 recipe tab
  拆分日期：2026-06-17 P1-3-Batch-6
-->
<template>
  <div>
    <div class="page-header">
      <h2 class="page-title">染色工艺参数智能推荐</h2>
    </div>

    <el-row :gutter="20">
      <el-col :span="8">
        <el-card shadow="hover" class="mb-20">
          <template #header>
            <div class="card-header">推荐条件</div>
          </template>
          <el-form :model="recipeForm" label-width="100px">
            <el-form-item label="色号" required>
              <el-input
                :model-value="recipeForm.color_no"
                @update:model-value="(v: any) => emit('update:recipeForm', { ...recipeForm, color_no: v })"
                placeholder="如 BL-301"
              />
            </el-form-item>
            <el-form-item label="布类" required>
              <el-select
                :model-value="recipeForm.fabric_type"
                @update:model-value="(v: any) => emit('update:recipeForm', { ...recipeForm, fabric_type: v })"
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
                :model-value="recipeForm.dye_type"
                @update:model-value="(v: any) => emit('update:recipeForm', { ...recipeForm, dye_type: v })"
                placeholder="可选，如 活性染料"
              />
            </el-form-item>
            <el-form-item label="颜色名称">
              <el-input
                :model-value="recipeForm.color_name"
                @update:model-value="(v: any) => emit('update:recipeForm', { ...recipeForm, color_name: v })"
                placeholder="可选，如 宝蓝"
              />
            </el-form-item>
            <el-form-item label="K 值">
              <el-input-number
                :model-value="recipeForm.k"
                @update:model-value="(v: any) => emit('update:recipeForm', { ...recipeForm, k: v })"
                :min="0"
                :max="20"
                :step="1"
                style="width: 100%"
              />
            </el-form-item>
            <el-form-item>
              <el-button type="primary" :loading="recipeLoading" @click="emit('recommend')"
                >获取推荐</el-button
              >
              <el-button @click="emit('load-template')">加载模板</el-button>
              <el-button type="success" @click="emit('save-template')">保存为模板</el-button>
            </el-form-item>
          </el-form>
        </el-card>
      </el-col>

      <el-col :span="16">
        <el-card shadow="hover">
          <template #header>
            <div class="card-header">推荐结果</div>
          </template>
          <el-empty v-if="!recipeResult" description="请填写条件后获取推荐" />
          <div v-else>
            <h4>推荐工艺参数</h4>
            <el-divider />
            <el-descriptions :column="2" border>
              <el-descriptions-item label="温度 (℃)">{{ recipeResult.temperature }}</el-descriptions-item>
              <el-descriptions-item label="时间 (分钟)">{{ recipeResult.duration }}</el-descriptions-item>
              <el-descriptions-item label="pH 值">{{ recipeResult.ph_value }}</el-descriptions-item>
              <el-descriptions-item label="浴比">{{ recipeResult.bath_ratio }}</el-descriptions-item>
              <el-descriptions-item label="染料用量 (g/L)">{{ recipeResult.dye_amount }}</el-descriptions-item>
              <el-descriptions-item label="助剂">{{ recipeResult.auxiliaries }}</el-descriptions-item>
              <el-descriptions-item label="推荐设备" :span="2">{{
                recipeResult.equipment
              }}</el-descriptions-item>
            </el-descriptions>

            <h4 class="mt-20">推荐机器</h4>
            <el-divider />
            <el-tag
              v-for="m in recommendMachines"
              :key="m.id"
              :type="m.recommended ? 'success' : 'info'"
              class="mr-10"
            >
              {{ m.name }}
            </el-tag>
            <el-button class="ml-20" type="primary" @click="emit('recommend-machine')"
              >智能推荐机器</el-button
            >
          </div>
        </el-card>
      </el-col>
    </el-row>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  recipeForm: any
  recipeLoading: boolean
  recipeResult: any
  recommendMachines: any[]
}>()

const emit = defineEmits<{
  recommend: []
  'load-template': []
  'save-template': []
  'recommend-machine': []
  'update:recipeForm': [value: any]
}>()
</script>
