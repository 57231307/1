<script setup lang="ts">
/**
 * QltPanel - 质量预测 tab 视图组件
 * 任务编号: P13 批 1 B3 I-1（拆分 advanced/index.vue 第 5 个 tab）
 * 质量预测（A2-2）：基于历史检验记录
 * 数据与函数全部由父组件通过 props 传入
 * P9-3 批次 F 重构：移除 vue/no-mutating-props 抑制，改用本地 ref 镜像 + watch 防循环
 */
import { ref, watch, nextTick } from 'vue'

interface QualityFormData {
  product_id: number | null
  inspection_type: string
  window_days: number
}

const props = defineProps<{
  // 质量预测表单（由父组件管理，子组件通过 emit('update:qualityForm') 回写）
  qualityForm: QualityFormData
  qualityLoading: boolean
  qualityResult: any
  runQualityPrediction: () => Promise<void>
}>()

const emit = defineEmits<{
  // 整体回写表单
  (e: 'update:qualityForm', form: QualityFormData): void
}>()

// 本地镜像：避免直接修改 prop 触发 vue/no-mutating-props
const localForm = ref<QualityFormData>({ ...props.qualityForm })

// 同步标志位：防止 prop → local 与 local → emit 形成循环
let syncing = false

// 外部 prop 变化时同步到 local
watch(
  () => props.qualityForm,
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
    emit('update:qualityForm', { ...newForm })
    nextTick(() => {
      syncing = false
    })
  },
  { deep: true },
)
</script>

<template>
  <div class="page-header">
    <h2 class="page-title">质量预测（基于历史检验记录）</h2>
  </div>

  <el-row :gutter="20">
    <el-col :span="8">
      <el-card shadow="hover" class="mb-20">
        <template #header><div class="card-header">预测条件</div></template>
        <el-form :model="localForm" label-width="100px">
          <el-form-item label="产品 ID">
            <el-input-number
              v-model="localForm.product_id"
              :min="0"
              :step="1"
              placeholder="可选，不填则全产品"
              style="width: 100%"
            />
          </el-form-item>
          <el-form-item label="检验类型">
            <el-select
              v-model="localForm.inspection_type"
              placeholder="可选，默认为全部"
              clearable
              style="width: 100%"
            >
              <el-option label="全部" value="" />
              <el-option label="进货检验" value="进货检验" />
              <el-option label="过程检验" value="过程检验" />
              <el-option label="成品检验" value="成品检验" />
              <el-option label="出货检验" value="出货检验" />
            </el-select>
          </el-form-item>
          <el-form-item label="时间窗口">
            <el-input-number
              v-model="localForm.window_days"
              :min="1"
              :max="365"
              :step="1"
              style="width: 100%"
            />
            <span class="form-hint">默认 90 天，范围 1-365</span>
          </el-form-item>
          <el-form-item>
            <el-button
              type="primary"
              :loading="qualityLoading"
              @click="runQualityPrediction"
              >开始预测</el-button
            >
          </el-form-item>
        </el-form>
      </el-card>
    </el-col>

    <el-col :span="16">
      <el-card shadow="hover" class="mb-20">
        <template #header>
          <div class="card-header">预测结果</div>
        </template>
        <el-empty
          v-if="!qualityResult"
          description="请填写条件后开始预测"
        />
        <div v-else>
          <!-- 关键指标卡片 -->
          <el-row :gutter="12" class="mb-12">
            <el-col :span="6">
              <el-statistic title="总检验次数" :value="qualityResult.total_inspections" />
            </el-col>
            <el-col :span="6">
              <el-statistic
                title="平均合格率"
                :value="qualityResult.avg_qualification_rate"
                :precision="2"
                suffix="%"
              />
              <el-progress
                :percentage="qualityResult.avg_qualification_rate"
                :stroke-width="6"
                :show-text="false"
                :status="qualityResult.avg_qualification_rate >= 95 ? 'success' : qualityResult.avg_qualification_rate >= 85 ? 'warning' : 'exception'"
                style="margin-top: 4px"
              />
            </el-col>
            <el-col :span="6">
              <div class="metric-label">趋势</div>
              <el-tag
                :type="qualityResult.trend === '上升' ? 'success' : qualityResult.trend === '下降' ? 'danger' : qualityResult.trend === '平稳' ? 'info' : 'warning'"
                size="large"
              >
                {{ qualityResult.trend }}
                <span v-if="qualityResult.trend_rate !== 0 && qualityResult.trend !== '无数据'" style="margin-left: 4px">
                  ({{ qualityResult.trend_rate > 0 ? '+' : '' }}{{ qualityResult.trend_rate }}pp)
                </span>
              </el-tag>
            </el-col>
            <el-col :span="6">
              <div class="metric-label">风险等级</div>
              <el-tag
                :type="qualityResult.risk_level === '高' ? 'danger' : qualityResult.risk_level === '中' ? 'warning' : 'success'"
                size="large"
              >
                {{ qualityResult.risk_level }}（{{ qualityResult.risk_score }}）
              </el-tag>
              <div class="metric-sub">置信度 {{ Math.round(qualityResult.confidence * 100) }}%</div>
            </el-col>
          </el-row>

          <!-- 主要问题归因 -->
          <h4 class="mb-10" style="margin-top: 8px">主要问题归因（Top 3）</h4>
          <el-table
            v-if="qualityResult.top_issues && qualityResult.top_issues.length > 0"
            :data="qualityResult.top_issues"
            stripe
            size="small"
            border
          >
            <el-table-column prop="issue_type" label="问题类型" min-width="160" />
            <el-table-column prop="occurrences" label="出现次数" width="120" align="right" />
            <el-table-column label="占比" width="200">
              <template #default="{ row }">
                <el-progress
                  :percentage="row.percentage"
                  :stroke-width="8"
                  :show-text="true"
                />
              </template>
            </el-table-column>
          </el-table>
          <el-empty
            v-else
            description="暂无不合格记录"
            :image-size="60"
          />

          <!-- 建议措施 -->
          <h4 class="mb-10" style="margin-top: 16px">建议措施</h4>
          <ul class="rec-list">
            <li v-for="(r, i) in qualityResult.recommendations" :key="i">
              <el-alert :title="r" type="info" :closable="false" show-icon />
            </li>
          </ul>

          <!-- 周期明细 -->
          <h4 class="mb-10" style="margin-top: 16px">周期明细（按月）</h4>
          <el-table
            v-if="qualityResult.period_breakdown && qualityResult.period_breakdown.length > 0"
            :data="qualityResult.period_breakdown"
            stripe
            size="small"
            border
          >
            <el-table-column prop="period" label="周期" width="120" />
            <el-table-column prop="inspections" label="检验次数" width="120" align="right" />
            <el-table-column label="平均合格率" min-width="200">
              <template #default="{ row }">
                {{ row.avg_qualification_rate.toFixed(2) }}%
              </template>
            </el-table-column>
          </el-table>
          <el-empty
            v-else
            description="暂无周期数据"
            :image-size="60"
          />

          <el-alert
            class="mt-12"
            :title="`数据来源：${qualityResult.source === 'history' ? '历史真实数据' : '保守默认值（历史不足 5 条）'}`"
            :type="qualityResult.source === 'history' ? 'success' : 'warning'"
            :closable="false"
            show-icon
          />
        </div>
      </el-card>
    </el-col>
  </el-row>
</template>
