<script setup lang="ts">
/**
 * P2-4 工艺优化列表 + 创建
 */
import { reactive, ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox } from 'element-plus'
import {
  createProcessOptimization,
  deleteProcessOptimization,
  SOURCE_LABELS,
  type AiProcessOptimization,
  type ProcessOptRequest,
} from '@/api/ai-extend'
// 批次 280：接入 useTableApi，消除手写 items/loading/total/page/pageSize/load 重复
import { useTableApi } from '@/composables/useTableApi'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const router = useRouter()

// 批次 280：filter 仅保留筛选字段，分页字段由 useTableApi 管理
const filter = reactive({
  color_no: '',
  fabric_type: '',
  is_applied: undefined as boolean | undefined,
  source: undefined as string | undefined,
})

// 批次 280：useTableApi 自动管理分页状态、数据加载，自动 watch page/pageSize 变化触发重载
// listProcessOptimizations 返回 PageResult<T>（{ items, total }），useTableApi detectList 会取 obj.items
const {
  data: items,
  loading,
  page,
  pageSize,
  total,
  refresh: load,
  setQueryParam,
} = useTableApi<AiProcessOptimization>({
  url: '/ai/process-optimizations',
  onError: () => ElMessage.error(t('message.loadFailed')),
})

const dialogVisible = ref(false)
const form = reactive<ProcessOptRequest>({
  color_no: '',
  color_name: '',
  fabric_type: '',
  dye_type: '',
  k: 5,
})
const submitting = ref(false)

// 批次 280：同步筛选条件到 useTableApi.queryParams 并刷新
function syncQueryParams() {
  setQueryParam('color_no', filter.color_no || undefined)
  setQueryParam('fabric_type', filter.fabric_type || undefined)
  setQueryParam('is_applied', filter.is_applied)
  setQueryParam('source', filter.source)
}

// 批次 280：查询时先同步筛选条件再刷新
function handleSearch() {
  syncQueryParams()
  page.value = 1
  load()
}

function openCreate() {
  form.color_no = ''
  form.color_name = ''
  form.fabric_type = ''
  form.dye_type = ''
  form.k = 5
  dialogVisible.value = true
}

async function submitCreate() {
  if (!form.color_no.trim()) {
    ElMessage.warning('请填写色号')
    return
  }
  if (!form.fabric_type.trim()) {
    ElMessage.warning('请填写布类')
    return
  }
  submitting.value = true
  try {
    const resp = await createProcessOptimization({ ...form })
    ElMessage.success(`推荐生成成功（来源：${SOURCE_LABELS[resp.response.source]}，置信度 ${resp.response.confidence.toFixed(2)}）`)
    dialogVisible.value = false
    page.value = 1
    await load()
    router.push(`/ai-extend/process-detail/${resp.id}`)
  } catch (e) {
    ElMessage.error(t('message.createFailed'))
  } finally {
    submitting.value = false
  }
}

async function handleDelete(row: AiProcessOptimization) {
  await ElMessageBox.confirm(`确定删除色号 ${row.color_no} 的工艺优化记录？`, t('message.confirmTitle'), { type: 'warning' })
  try {
    await deleteProcessOptimization(row.id)
    ElMessage.success('已删除')
    await load()
  } catch (e) {
    ElMessage.error(t('message.deleteFailed'))
  }
}

function goDetail(row: AiProcessOptimization) {
  router.push(`/ai-extend/process-detail/${row.id}`)
}

function resetFilter() {
  filter.color_no = ''
  filter.fabric_type = ''
  filter.is_applied = undefined
  filter.source = undefined
  syncQueryParams()
  page.value = 1
  load()
}

const sourceOptions = [
  { value: '', label: '全部' },
  { value: 'knn', label: 'k-NN' },
  { value: 'fallback', label: '兜底' },
]
const appliedOptions = [
  { value: undefined, label: '全部' },
  { value: true, label: '已应用' },
  { value: false, label: '未应用' },
]
</script>

<template>
  <div class="proc-page">
    <div class="page-header">
      <h2>工艺优化历史</h2>
      <div class="header-right">
        <el-button type="primary" @click="openCreate">+ 触发新推荐</el-button>
      </div>
    </div>

    <el-card class="filter-card">
      <el-form :inline="true" :model="filter" aria-label="工艺优化筛选表单">
        <el-form-item label="色号">
          <el-input v-model="filter.color_no" placeholder="如 BL-301" clearable style="width: 160px" />
        </el-form-item>
        <el-form-item label="布类">
          <el-input v-model="filter.fabric_type" placeholder="如 棉" clearable style="width: 140px" />
        </el-form-item>
        <el-form-item label="来源">
          <el-select v-model="filter.source" clearable style="width: 140px">
            <el-option v-for="o in sourceOptions" :key="o.value" :label="o.label" :value="o.value || undefined" />
          </el-select>
        </el-form-item>
        <el-form-item label="应用状态">
          <el-select v-model="filter.is_applied" clearable style="width: 140px">
            <el-option v-for="o in appliedOptions" :key="String(o.value)" :label="o.label" :value="o.value" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="resetFilter">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card>
      <el-table v-loading="loading" :data="items" stripe border aria-label="工艺优化列表">
        <el-table-column prop="id" label="ID" width="70" />
        <el-table-column prop="color_no" label="色号" width="100" />
        <el-table-column prop="color_name" label="色名" width="120" show-overflow-tooltip />
        <el-table-column prop="fabric_type" label="布类" width="80" />
        <el-table-column prop="dye_type" label="染料" width="100" />
        <el-table-column prop="source" label="来源" width="100">
          <template #default="{ row }">{{ SOURCE_LABELS[row.source] || row.source }}</template>
        </el-table-column>
        <el-table-column prop="similar_cases" label="相似" width="70" align="center" />
        <el-table-column prop="confidence" label="置信度" width="100">
          <template #default="{ row }">{{ Number(row.confidence).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="推荐参数" min-width="200">
          <template #default="{ row }">
            <span class="mono">{{ row.recommended_temperature }}°C</span> ·
            <span class="mono">{{ row.recommended_time_minutes }}min</span> ·
            <span class="mono">pH {{ row.recommended_ph_value }}</span> ·
            <span class="mono">1:{{ row.recommended_liquor_ratio }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="is_applied" label="应用" width="80">
          <template #default="{ row }">
            <el-tag :type="row.is_applied ? 'success' : 'info'" size="small">
              {{ row.is_applied ? '已应用' : '未应用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="feedback_score" label="反馈" width="80" align="center">
          <template #default="{ row }">
            <el-rate v-if="row.feedback_score" :model-value="row.feedback_score" disabled :max="5" />
            <span v-else class="muted">—</span>
          </template>
        </el-table-column>
        <el-table-column prop="created_at" label="创建时间" min-width="160">
          <template #default="{ row }">{{ new Date(row.created_at).toLocaleString('zh-CN') }}</template>
        </el-table-column>
        <el-table-column label="操作" width="150" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" size="small" @click="goDetail(row)">详情</el-button>
            <el-button v-permission="'ai_process_optimization:delete'" type="danger" size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>
      <el-pagination
        v-model:current-page="page"
        v-model:page-size="pageSize"
        :total="total"
        :page-sizes="[10, 20, 50, 100]"
        layout="total, sizes, prev, pager, next, jumper"
        style="margin-top: 16px; justify-content: flex-end"
        aria-label="工艺优化列表分页"
      />
    </el-card>

    <!-- 创建弹窗 -->
    <el-dialog v-model="dialogVisible" title="触发 AI 工艺优化" width="540px" aria-label="工艺优化创建对话框">
      <el-form :model="form" label-width="100px" aria-label="工艺优化表单">
        <el-form-item label="色号" required>
          <el-input v-model="form.color_no" placeholder="如 BL-301" maxlength="64" />
        </el-form-item>
        <el-form-item label="色名">
          <el-input v-model="form.color_name" placeholder="如 雾霾蓝" maxlength="128" />
        </el-form-item>
        <el-form-item label="布类" required>
          <el-input v-model="form.fabric_type" placeholder="如 棉 / 涤纶 / 麻" maxlength="64" />
        </el-form-item>
        <el-form-item label="染料类型">
          <el-select v-model="form.dye_type" placeholder="可选" clearable style="width: 100%">
            <el-option label="活性染料" value="活性染料" />
            <el-option label="分散染料" value="分散染料" />
            <el-option label="酸性染料" value="酸性染料" />
            <el-option label="还原染料" value="还原染料" />
            <el-option label="直接染料" value="直接染料" />
          </el-select>
        </el-form-item>
        <el-form-item label="k-NN k 值">
          <el-input-number v-model="form.k" :min="1" :max="20" />
          <span class="hint">推荐 3-10</span>
        </el-form-item>
        <el-alert
          title="历史相似案例 ≥ 3 条时走 k-NN 加权；不足时走典型参数表兜底"
          type="info"
          :closable="false"
          show-icon
        />
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitting" @click="submitCreate">生成推荐</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<style scoped>
.proc-page {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.page-header h2 {
  margin: 0;
  font-size: 20px;
}
.header-right {
  display: flex;
  gap: 8px;
}
.filter-card {
  margin-bottom: 0;
}
.mono {
  font-family: 'SFMono-Regular', Consolas, monospace;
  color: #303133;
}
.muted {
  color: #c0c4cc;
}
.hint {
  margin-left: 8px;
  font-size: 12px;
  color: #909399;
}
</style>
