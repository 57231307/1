<!--
  CostCollectionTab.vue - 成本归集 Tab
  来源：原 cost/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="cost-collection-tab">
    <div class="page-header">
      <h2 class="page-title">成本归集</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>新建归集
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>导出
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm">
        <el-form-item label="归集单号">
          <el-input v-model="queryForm.collection_no" placeholder="单号" clearable />
        </el-form-item>
        <el-form-item label="批号">
          <el-input v-model="queryForm.batch_no" placeholder="批号" clearable />
        </el-form-item>
        <el-form-item label="状态">
          <el-select v-model="queryForm.status" placeholder="选择状态" clearable>
            <el-option label="草稿" value="draft" />
            <el-option label="待审" value="pending" />
            <el-option label="已审" value="approved" />
            <el-option label="驳回" value="rejected" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">查询</el-button>
          <el-button @click="handleReset">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="collectionList" stripe>
        <el-table-column prop="collection_no" label="归集单号" width="140" />
        <el-table-column prop="collection_date" label="归集日期" width="120" />
        <el-table-column prop="batch_no" label="批号" width="120" />
        <el-table-column prop="color_no" label="色号" width="100" />
        <el-table-column label="直接材料" width="120" align="right">
          <template #default="{ row }">¥{{ (row.direct_material || 0).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="直接人工" width="120" align="right">
          <template #default="{ row }">¥{{ (row.direct_labor || 0).toFixed(2) }}</template>
        </el-table-column>
        <el-table-column label="制造费用" width="120" align="right">
          <template #default="{ row }"
            >¥{{ (row.manufacturing_overhead || 0).toFixed(2) }}</template
          >
        </el-table-column>
        <el-table-column label="总成本" width="120" align="right">
          <template #default="{ row }">
            <span class="text-bold">¥{{ (row.total_cost || 0).toFixed(2) }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="status" label="状态" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column prop="remark" label="备注" min-width="150" show-overflow-tooltip />
        <el-table-column label="操作" width="180" fixed="right">
          <template #default="{ row }">
            <el-button v-permission="'cost_collection:update'" type="primary" link size="small" @click="openDialog(row)">编辑</el-button>
            <el-button
              v-permission="'cost_collection:approve'"
              v-if="row.status === 'draft' || row.status === 'pending'"
              type="success"
              link
              size="small"
              @click="auditCollection(row, true)"
              >审核</el-button
            >
            <el-button
              v-permission="'cost_collection:approve'"
              v-if="row.status === 'pending'"
              type="warning"
              link
              size="small"
              @click="auditCollection(row, false)"
              >驳回</el-button
            >
            <el-button v-permission="'cost_collection:delete'" type="danger" link size="small" @click="handleDelete(row)">删除</el-button>
          </template>
        </el-table-column>
      </el-table>

      <div class="pagination-wrapper">
        <el-pagination
          v-model:current-page="page"
          v-model:page-size="pageSize"
          :page-sizes="[10, 20, 50, 100]"
          :total="total"
          layout="total, sizes, prev, pager, next, jumper"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog
      v-model="dialogVisible"
      :title="form.id ? '编辑成本归集' : '新建成本归集'"
      width="600px"
    >
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="归集日期" prop="collection_date">
              <el-date-picker
                v-model="form.collection_date"
                type="date"
                placeholder="选择日期"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="批号">
              <el-input v-model="form.batch_no" placeholder="如 B001" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="色号">
              <el-input v-model="form.color_no" placeholder="如 C001" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="期间">
              <el-input v-model="form.period" placeholder="如 2024-01" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="直接材料" prop="direct_material">
              <el-input-number
                v-model="form.direct_material"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="直接人工" prop="direct_labor">
              <el-input-number
                v-model="form.direct_labor"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="制造费用" prop="manufacturing_overhead">
              <el-input-number
                v-model="form.manufacturing_overhead"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="总成本">
              <span class="text-bold">¥{{ totalCost.toFixed(2) }}</span>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item label="备注">
          <el-input v-model="form.remark" type="textarea" :rows="3" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Download } from '@element-plus/icons-vue'
import {
  createCostCollection,
  updateCostCollection,
  deleteCollection as deleteCollectionApi,
  auditCollection as auditCollectionApi,
  COST_STATUS,
  type CostCollection,
} from '@/api/cost'
import { logger } from '@/utils/logger'
import { exportToExcel } from '@/utils/export'
// 批次 278：迁移到 useTableApi composable，自动管理分页与 loading
import { useTableApi } from '@/composables/useTableApi'

// 批次 34 v9 P1：接入 i18n，替换硬编码中文 ElMessage
const { t } = useI18n({ useScope: 'global' })

const submitLoading = ref(false)
const dialogVisible = ref(false)
const formRef = ref<FormInstance>()

// 批次 278：筛选条件（仅保留业务字段，page/page_size 由 useTableApi 管理）
const queryForm = reactive({
  collection_no: '',
  batch_no: '',
  status: '',
})

// 批次 278：使用 useTableApi 管理成本归集列表分页
const {
  data: collectionList,
  total,
  loading,
  page,
  pageSize,
  setQueryParam,
  refresh: fetchCollections,
} = useTableApi<CostCollection>({
  url: '/production/cost-collections',
  defaultPageSize: 20,
  onError: (err: unknown) => {
    if (err instanceof Error) {
      ElMessage.error(err.message || '获取成本归集列表失败')
    } else {
      ElMessage.error('获取成本归集列表失败')
    }
  },
})

// 批次 278：将筛选字段同步到 queryParams
const syncQueryParams = () => {
  setQueryParam('collection_no', queryForm.collection_no)
  setQueryParam('batch_no', queryForm.batch_no)
  setQueryParam('status', queryForm.status)
}

// 批次 278：分页变化处理函数
const handlePageChange = (_p: number) => {
  // useTableApi 内部 watch page 自动触发刷新
}
const handleSizeChange = (_s: number) => {
  // useTableApi 内部 watch pageSize 自动触发刷新
  page.value = 1
}

const form = reactive<Partial<CostCollection>>({
  id: undefined,
  collection_date: new Date().toISOString().split('T')[0],
  batch_no: '',
  color_no: '',
  period: new Date().toISOString().slice(0, 7),
  direct_material: 0,
  direct_labor: 0,
  manufacturing_overhead: 0,
  remark: '',
})

const rules: FormRules = {
  collection_date: [{ required: true, message: t('cost.validation.collectionDateRequired'), trigger: 'change' }],
  direct_material: [{ required: true, message: t('cost.validation.directMaterialRequired'), trigger: 'blur' }],
  direct_labor: [{ required: true, message: t('cost.validation.directLaborRequired'), trigger: 'blur' }],
  manufacturing_overhead: [{ required: true, message: t('cost.validation.manufacturingOverheadRequired'), trigger: 'blur' }],
}

const totalCost = computed(() => {
  return (form.direct_material || 0) + (form.direct_labor || 0) + (form.manufacturing_overhead || 0)
})

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    [COST_STATUS.DRAFT]: '草稿',
    [COST_STATUS.PENDING]: '待审',
    [COST_STATUS.APPROVED]: '已审',
    [COST_STATUS.REJECTED]: '驳回',
  }
  return map[status] || status
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    [COST_STATUS.DRAFT]: 'info',
    [COST_STATUS.PENDING]: 'warning',
    [COST_STATUS.APPROVED]: 'success',
    [COST_STATUS.REJECTED]: 'danger',
  }
  return map[status] || 'info'
}

const handleSearch = () => {
  // 批次 278：同步筛选条件并重置到第一页
  syncQueryParams()
  page.value = 1
  fetchCollections()
}

const handleReset = () => {
  queryForm.collection_no = ''
  queryForm.batch_no = ''
  queryForm.status = ''
  handleSearch()
}

const openDialog = (row?: CostCollection) => {
  formRef.value?.resetFields()
  if (row) {
    Object.assign(form, row)
  } else {
    form.id = undefined
    form.collection_date = new Date().toISOString().split('T')[0]
    form.batch_no = ''
    form.color_no = ''
    form.period = new Date().toISOString().slice(0, 7)
    form.direct_material = 0
    form.direct_labor = 0
    form.manufacturing_overhead = 0
    form.remark = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      const data: Partial<CostCollection> = {
        ...form,
        total_cost: totalCost.value,
      }
      if (form.id) {
        await updateCostCollection(form.id, data)
        ElMessage.success(t('message.updateSuccess'))
      } else {
        await createCostCollection(data)
        ElMessage.success(t('message.createSuccess'))
      }
      dialogVisible.value = false
      fetchCollections()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: CostCollection) => {
  if (!row.id) return
  try {
    await ElMessageBox.confirm(t('cost.confirmDelete', { name: row.collection_no }), t('message.deleteConfirmTitle'), {
      type: 'warning',
    })
    await deleteCollectionApi(row.id)
    ElMessage.success(t('message.deleteSuccess'))
    fetchCollections()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '删除失败')
    }
  }
}

const auditCollection = async (row: CostCollection, approved: boolean) => {
  if (!row.id) return
  try {
    const text = approved ? '通过' : '驳回'
    await ElMessageBox.confirm(t('cost.confirmAction', { action: text }), t('cost.actionConfirmTitle', { action: text }), { type: 'info' })
    await auditCollectionApi(row.id, approved)
    ElMessage.success(`${text}成功`)
    fetchCollections()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || '操作失败')
    }
  }
}

const handleExport = () => {
  exportToExcel({
    filename: '成本归集',
    format: 'excel',
    data: collectionList.value.map((c): Record<string, unknown> => ({ ...c })),
    columns: [
      { key: 'collection_no', title: '归集单号' },
      { key: 'collection_date', title: '归集日期' },
      {
        key: 'batch_no',
        title: '批号',
        formatter: (value: unknown) => (value ? String(value) : '-'),
      },
      {
        key: 'color_no',
        title: '色号',
        formatter: (value: unknown) => (value ? String(value) : '-'),
      },
      {
        key: 'direct_material',
        title: '直接材料',
        formatter: (value: unknown) => Number(value || 0).toFixed(2),
      },
      {
        key: 'direct_labor',
        title: '直接人工',
        formatter: (value: unknown) => Number(value || 0).toFixed(2),
      },
      {
        key: 'manufacturing_overhead',
        title: '制造费用',
        formatter: (value: unknown) => Number(value || 0).toFixed(2),
      },
      {
        key: 'total_cost',
        title: '总成本',
        formatter: (value: unknown) => Number(value || 0).toFixed(2),
      },
      {
        key: 'status',
        title: '状态',
        formatter: (value: unknown) => getStatusLabel(String(value)),
      },
    ],
  })
  logger.info('成本归集列表已导出')
}
</script>
