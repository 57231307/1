<!--
  AssetListTab.vue - 固定资产 Tab
  来源：原 fixed-assets/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
  D05 Batch 1：接入 useI18n，所有硬编码中文迁移到 locales/zh-CN.ts + en-US.ts
-->
<template>
  <div class="asset-list-tab">
    <div class="page-header">
      <h2 class="page-title">{{ $t('fixedAssets.title') }}</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>{{ $t('fixedAssets.create') }}
        </el-button>
        <el-button @click="handleDepreciateAll">
          <el-icon><Refresh /></el-icon>{{ $t('fixedAssets.depreciate') }}
        </el-button>
        <el-button @click="handleExport">
          <el-icon><Download /></el-icon>{{ $t('fixedAssets.export') }}
        </el-button>
      </div>
    </div>

    <el-card shadow="hover" class="filter-card">
      <el-form :inline="true" :model="queryForm" :aria-label="$t('fixedAssets.filter.ariaLabel')">
        <el-form-item :label="$t('fixedAssets.filter.assetCode')">
          <el-input v-model="queryForm.asset_code" :placeholder="$t('fixedAssets.filter.assetCodePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('fixedAssets.filter.assetName')">
          <el-input v-model="queryForm.asset_name" :placeholder="$t('fixedAssets.filter.assetNamePlaceholder')" clearable />
        </el-form-item>
        <el-form-item :label="$t('fixedAssets.filter.category')">
          <el-select v-model="queryForm.category" :placeholder="$t('fixedAssets.filter.categoryPlaceholder')" clearable>
            <el-option :label="$t('fixedAssets.category.building')" value="building" />
            <el-option :label="$t('fixedAssets.category.equipment')" value="equipment" />
            <el-option :label="$t('fixedAssets.category.vehicle')" value="vehicle" />
            <el-option :label="$t('fixedAssets.category.electronic')" value="electronic" />
            <el-option :label="$t('fixedAssets.category.furniture')" value="furniture" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('fixedAssets.filter.status')">
          <el-select v-model="queryForm.status" :placeholder="$t('fixedAssets.filter.statusPlaceholder')" clearable>
            <el-option :label="$t('fixedAssets.status.inUse')" value="in_use" />
            <el-option :label="$t('fixedAssets.status.idle')" value="idle" />
            <el-option :label="$t('fixedAssets.status.disposed')" value="disposed" />
          </el-select>
        </el-form-item>
        <el-form-item>
          <el-button type="primary" @click="handleSearch">{{ $t('fixedAssets.filter.query') }}</el-button>
          <el-button @click="handleReset">{{ $t('fixedAssets.filter.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="assetList" stripe :aria-label="$t('fixedAssets.table.ariaLabel')">
        <el-table-column prop="asset_code" :label="$t('fixedAssets.table.assetCode')" width="120" />
        <el-table-column prop="asset_name" :label="$t('fixedAssets.table.assetName')" min-width="150" />
        <el-table-column prop="category" :label="$t('fixedAssets.table.category')" width="100">
          <template #default="{ row }">{{ getCategoryLabel(row.category) }}</template>
        </el-table-column>
        <el-table-column prop="department_name" :label="$t('fixedAssets.table.department')" width="120" />
        <el-table-column prop="purchase_date" :label="$t('fixedAssets.table.purchaseDate')" width="120" />
        <el-table-column :label="$t('fixedAssets.table.originalValue')" width="120" align="right">
          <template #default="{ row }">¥{{ row.purchase_amount.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column :label="$t('fixedAssets.table.accumulatedDepreciation')" width="120" align="right">
          <template #default="{ row }">¥{{ row.accumulated_depreciation.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column :label="$t('fixedAssets.table.netValue')" width="120" align="right">
          <template #default="{ row }">¥{{ row.net_value.toFixed(2) }}</template>
        </el-table-column>
        <el-table-column prop="status" :label="$t('fixedAssets.table.status')" width="100">
          <template #default="{ row }">
            <el-tag :type="getStatusType(row.status)">{{ getStatusLabel(row.status) }}</el-tag>
          </template>
        </el-table-column>
        <el-table-column :label="$t('fixedAssets.table.operation')" width="240" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openDialog(row)">{{ $t('fixedAssets.table.edit') }}</el-button>
            <el-button type="success" link size="small" @click="handleDepreciate(row)"
              >{{ $t('fixedAssets.table.depreciate') }}</el-button
            >
            <!-- v3 复审 P1-2：在用资产可处置，处置后状态置为 disposed -->
            <el-button
              v-if="row.status === 'active'"
              type="warning"
              link
              size="small"
              @click="handleDispose(row)"
              >{{ $t('fixedAssets.table.dispose') }}</el-button
            >
            <el-button type="danger" link size="small" @click="handleDelete(row)">{{ $t('fixedAssets.table.delete') }}</el-button>
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
          :aria-label="$t('fixedAssets.table.paginationAriaLabel')"
          @size-change="handleSizeChange"
          @current-change="handlePageChange"
        />
      </div>
    </el-card>

    <el-dialog v-model="dialogVisible" :title="form.id ? $t('fixedAssets.dialog.editTitle') : $t('fixedAssets.dialog.createTitle')" width="600px" :aria-label="$t('fixedAssets.dialog.ariaLabel')">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" :aria-label="$t('fixedAssets.dialog.formAriaLabel')">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.assetCode')" prop="asset_code">
              <el-input v-model="form.asset_code" :disabled="!!form.id" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.assetName')" prop="asset_name">
              <el-input v-model="form.asset_name" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.category')" prop="category">
              <el-select v-model="form.category" :placeholder="$t('fixedAssets.dialog.categoryPlaceholder')" style="width: 100%">
                <el-option :label="$t('fixedAssets.category.building')" value="building" />
                <el-option :label="$t('fixedAssets.category.equipment')" value="equipment" />
                <el-option :label="$t('fixedAssets.category.vehicle')" value="vehicle" />
                <el-option :label="$t('fixedAssets.category.electronic')" value="electronic" />
                <el-option :label="$t('fixedAssets.category.furniture')" value="furniture" />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.purchaseDate')" prop="purchase_date">
              <el-date-picker
                v-model="form.purchase_date"
                type="date"
                :placeholder="$t('fixedAssets.dialog.purchaseDatePlaceholder')"
                value-format="YYYY-MM-DD"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.originalValue')" prop="purchase_amount">
              <el-input-number
                v-model="form.purchase_amount"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.salvageValue')" prop="salvage_value">
              <el-input-number
                v-model="form.salvage_value"
                :min="0"
                :precision="2"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.usefulLifeMonths')" prop="useful_life_months">
              <el-input-number v-model="form.useful_life_months" :min="1" style="width: 100%" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.depreciationMethod')" prop="depreciation_method">
              <el-select v-model="form.depreciation_method" style="width: 100%">
                <el-option :label="$t('fixedAssets.depreciationMethod.straightLine')" value="straight_line" />
                <el-option :label="$t('fixedAssets.depreciationMethod.workload')" value="workload" />
                <el-option :label="$t('fixedAssets.depreciationMethod.doubleDeclining')" value="double_declining" />
                <el-option :label="$t('fixedAssets.depreciationMethod.sumOfYears')" value="sum_of_years" />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.location')">
              <el-input v-model="form.location" />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="$t('fixedAssets.dialog.custodian')">
              <el-input v-model="form.custodian" />
            </el-form-item>
          </el-col>
        </el-row>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">{{ $t('fixedAssets.dialog.cancel') }}</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{ $t('fixedAssets.dialog.confirm') }}</el-button>
      </template>
    </el-dialog>

    <!-- v3 复审 P1-2：资产处置对话框 -->
    <el-dialog v-model="disposalDialogVisible" :title="$t('fixedAssets.disposal.title')" width="520px" :aria-label="$t('fixedAssets.disposal.ariaLabel')">
      <el-form
        ref="disposalFormRef"
        :model="disposalForm"
        :rules="disposalRules"
        label-width="100px"
        :aria-label="$t('fixedAssets.disposal.formAriaLabel')"
      >
        <el-form-item :label="$t('fixedAssets.disposal.type')" prop="disposal_type">
          <el-select v-model="disposalForm.disposal_type" :placeholder="$t('fixedAssets.disposal.typePlaceholder')" style="width: 100%">
            <el-option :label="$t('fixedAssets.disposal.typeSale')" value="SALE" />
            <el-option :label="$t('fixedAssets.disposal.typeScrap')" value="SCRAP" />
            <el-option :label="$t('fixedAssets.disposal.typeTransfer')" value="TRANSFER" />
          </el-select>
        </el-form-item>
        <el-form-item :label="$t('fixedAssets.disposal.value')" prop="disposal_value">
          <el-input-number
            v-model="disposalForm.disposal_value"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('fixedAssets.disposal.date')" prop="disposal_date">
          <el-date-picker
            v-model="disposalForm.disposal_date"
            type="date"
            :placeholder="$t('fixedAssets.disposal.datePlaceholder')"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="$t('fixedAssets.disposal.reason')" prop="reason">
          <el-input
            v-model="disposalForm.reason"
            type="textarea"
            :rows="3"
            :placeholder="$t('fixedAssets.disposal.reasonPlaceholder')"
          />
        </el-form-item>
        <el-form-item :label="$t('fixedAssets.disposal.buyerInfo')">
          <el-input v-model="disposalForm.buyer_info" :placeholder="$t('fixedAssets.disposal.buyerInfoPlaceholder')" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="disposalDialogVisible = false">{{ $t('fixedAssets.disposal.cancel') }}</el-button>
        <el-button type="warning" :loading="disposalSubmitting" @click="submitDisposal"
          >{{ $t('fixedAssets.disposal.confirm') }}</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive } from 'vue'
import { useI18n } from 'vue-i18n'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus, Refresh, Download } from '@element-plus/icons-vue'
import {
  createAsset,
  updateAsset,
  deleteAsset as deleteAssetApi,
  depreciateAsset,
  batchDepreciateAssets,
  disposeAsset,
  type FixedAsset,
  type FixedAssetCreateRequest,
  type FixedAssetUpdateRequest,
  type DisposalRequest,
} from '@/api/asset'
import { useUserStore } from '@/store/user'
import { logger } from '@/utils/logger'
import { exportFromBackend } from '@/utils/export'
// 批次 278：迁移到 useTableApi composable，自动管理分页与 loading
import { useTableApi } from '@/composables/useTableApi'

const { t } = useI18n({ useScope: 'global' })

const submitLoading = ref(false)
const dialogVisible = ref(false)
const formRef = ref<FormInstance>()

// v3 复审 P1-2：资产处置对话框相关状态
const disposalDialogVisible = ref(false)
const disposalSubmitting = ref(false)
const disposalFormRef = ref<FormInstance>()
const disposalTargetId = ref<number | undefined>(undefined)
const disposalForm = reactive<DisposalRequest>({
  disposal_type: 'SALE',
  disposal_value: 0,
  disposal_date: new Date().toISOString().split('T')[0],
  reason: '',
  buyer_info: '',
})

const disposalRules: FormRules = {
  disposal_type: [{ required: true, message: t('fixedAssets.validation.disposalTypeRequired'), trigger: 'change' }],
  disposal_value: [{ required: true, message: t('fixedAssets.validation.disposalValueRequired'), trigger: 'blur' }],
  disposal_date: [{ required: true, message: t('fixedAssets.validation.disposalDateRequired'), trigger: 'change' }],
  reason: [{ required: true, message: t('fixedAssets.validation.disposalReasonRequired'), trigger: 'blur' }],
}

// 批次 278：筛选条件（仅保留业务字段，page/page_size 由 useTableApi 管理）
const queryForm = reactive({
  asset_code: '',
  asset_name: '',
  category: '',
  status: '',
})

// 批次 278：使用 useTableApi 管理资产列表分页
const {
  data: assetList,
  total,
  loading,
  page,
  pageSize,
  queryParams,
  setQueryParam,
  refresh: fetchAssets,
} = useTableApi<FixedAsset>({
  url: '/fixed-assets',
  defaultPageSize: 20,
  onError: (err: unknown) => {
    if (err instanceof Error) {
      ElMessage.error(err.message || t('fixedAssets.message.loadListFailed'))
    } else {
      ElMessage.error(t('fixedAssets.message.loadListFailed'))
    }
  },
})

// 批次 278：将筛选字段同步到 queryParams
const syncQueryParams = () => {
  setQueryParam('asset_code', queryForm.asset_code)
  setQueryParam('asset_name', queryForm.asset_name)
  setQueryParam('category', queryForm.category)
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

const form = reactive<FixedAssetCreateRequest & { id?: number }>({
  id: undefined,
  asset_code: '',
  asset_name: '',
  category: 'equipment',
  purchase_date: new Date().toISOString().split('T')[0],
  purchase_amount: 0,
  salvage_value: 0,
  useful_life_months: 60,
  depreciation_method: 'straight_line',
  location: '',
  custodian: '',
})

const rules: FormRules = {
  asset_code: [{ required: true, message: t('fixedAssets.validation.assetCodeRequired'), trigger: 'blur' }],
  asset_name: [{ required: true, message: t('fixedAssets.validation.assetNameRequired'), trigger: 'blur' }],
  category: [{ required: true, message: t('fixedAssets.validation.categoryRequired'), trigger: 'change' }],
  purchase_date: [{ required: true, message: t('fixedAssets.validation.purchaseDateRequired'), trigger: 'change' }],
  purchase_amount: [{ required: true, message: t('fixedAssets.validation.purchaseAmountRequired'), trigger: 'blur' }],
  useful_life_months: [{ required: true, message: t('fixedAssets.validation.usefulLifeRequired'), trigger: 'blur' }],
  depreciation_method: [{ required: true, message: t('fixedAssets.validation.depreciationMethodRequired'), trigger: 'change' }],
}

const getCategoryLabel = (category: string) => {
  const map: Record<string, string> = {
    building: t('fixedAssets.category.building'),
    equipment: t('fixedAssets.category.equipment'),
    vehicle: t('fixedAssets.category.vehicle'),
    electronic: t('fixedAssets.category.electronic'),
    furniture: t('fixedAssets.category.furniture'),
  }
  return map[category] || category
}

const getStatusLabel = (status: string) => {
  const map: Record<string, string> = {
    in_use: t('fixedAssets.status.inUse'),
    idle: t('fixedAssets.status.idle'),
    disposed: t('fixedAssets.status.disposed'),
  }
  return map[status] || status
}

const getStatusType = (status: string) => {
  const map: Record<string, string> = {
    in_use: 'success',
    idle: 'warning',
    disposed: 'info',
  }
  return map[status] || 'info'
}

const handleSearch = () => {
  // 批次 278：同步筛选条件并重置到第一页
  syncQueryParams()
  page.value = 1
  fetchAssets()
}

const handleReset = () => {
  queryForm.asset_code = ''
  queryForm.asset_name = ''
  queryForm.category = ''
  queryForm.status = ''
  handleSearch()
}

const openDialog = (row?: FixedAsset) => {
  formRef.value?.resetFields()
  if (row) {
    form.id = row.id
    form.asset_code = row.asset_code
    form.asset_name = row.asset_name
    form.category = row.category
    form.purchase_date = row.purchase_date
    form.purchase_amount = row.purchase_amount
    form.salvage_value = row.salvage_value
    form.useful_life_months = row.useful_life_months
    form.depreciation_method = row.depreciation_method
    form.location = row.location
    form.custodian = row.custodian
  } else {
    form.id = undefined
    form.asset_code = ''
    form.asset_name = ''
    form.category = 'equipment'
    form.purchase_date = new Date().toISOString().split('T')[0]
    form.purchase_amount = 0
    form.salvage_value = 0
    form.useful_life_months = 60
    form.depreciation_method = 'straight_line'
    form.location = ''
    form.custodian = ''
  }
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      if (form.id) {
        const updateData: FixedAssetUpdateRequest = {
          asset_name: form.asset_name,
          location: form.location,
          custodian: form.custodian,
        }
        await updateAsset(form.id, updateData)
        ElMessage.success(t('fixedAssets.message.updateSuccess'))
      } else {
        await createAsset(form)
        ElMessage.success(t('fixedAssets.message.createSuccess'))
      }
      dialogVisible.value = false
      fetchAssets()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || t('fixedAssets.message.operationFailed'))
    } finally {
      submitLoading.value = false
    }
  })
}

const handleDelete = async (row: FixedAsset) => {
  try {
    await ElMessageBox.confirm(
      t('fixedAssets.message.deleteConfirm', { name: row.asset_name }),
      t('fixedAssets.message.deleteConfirmTitle'),
      {
        type: 'warning',
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
      }
    )
    await deleteAssetApi(row.id)
    ElMessage.success(t('fixedAssets.message.deleteSuccess'))
    fetchAssets()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('fixedAssets.message.deleteFailed'))
    }
  }
}

const handleDepreciate = async (row: FixedAsset) => {
  try {
    await ElMessageBox.confirm(
      t('fixedAssets.message.depreciateConfirm'),
      t('fixedAssets.message.depreciateTitle'),
      {
        type: 'info',
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
      }
    )
    // 批次 88 PH-2：补传当前期间（YYYY-MM 格式），后端按期间记录折旧明细
    const currentPeriod = new Date().toISOString().slice(0, 7)
    await depreciateAsset(row.id, currentPeriod)
    ElMessage.success(t('fixedAssets.message.depreciateSuccess'))
    fetchAssets()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('fixedAssets.message.depreciateFailed'))
    }
  }
}

// v3 复审 P1-2：打开资产处置对话框，记录待处置资产 ID 并重置表单
const handleDispose = (row: FixedAsset) => {
  disposalTargetId.value = row.id
  disposalForm.disposal_type = 'SALE'
  disposalForm.disposal_value = 0
  disposalForm.disposal_date = new Date().toISOString().split('T')[0]
  disposalForm.reason = ''
  disposalForm.buyer_info = ''
  disposalDialogVisible.value = true
}

// v3 复审 P1-2：提交资产处置请求，成功后刷新列表
const submitDisposal = async () => {
  if (!disposalFormRef.value || !disposalTargetId.value) return
  // 提取到局部变量，避免闭包内 ref.value 重新推断为 number | undefined
  const assetId = disposalTargetId.value
  await disposalFormRef.value.validate(async valid => {
    if (!valid) return
    disposalSubmitting.value = true
    try {
      await disposeAsset(assetId, { ...disposalForm })
      ElMessage.success(t('fixedAssets.message.disposeSuccess'))
      disposalDialogVisible.value = false
      fetchAssets()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || t('fixedAssets.message.disposeFailed'))
    } finally {
      disposalSubmitting.value = false
    }
  })
}

// 批次 157a P1-1 修复：接入 batchDepreciateAssets API 实现批量计提折旧
const handleDepreciateAll = async () => {
  if (assetList.value.length === 0) {
    ElMessage.warning(t('fixedAssets.message.noDepreciableAsset'))
    return
  }
  const currentPeriod = new Date().toISOString().slice(0, 7)
  try {
    const { value: inputPeriod } = await ElMessageBox.prompt(
      t('fixedAssets.message.batchDepreciatePrompt'),
      t('fixedAssets.message.batchDepreciateTitle'),
      {
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
        inputValue: currentPeriod,
        inputPattern: /^\d{4}-\d{2}$/,
        inputErrorMessage: t('fixedAssets.message.invalidPeriod'),
      }
    )
    const userStore = useUserStore()
    const userId = userStore.userInfo?.id
    if (!userId) {
      ElMessage.error(t('fixedAssets.message.userNotFound'))
      return
    }
    const assetIds = assetList.value
      .filter(a => a.status === 'in_use' || a.status === 'active')
      .map(a => a.id)
    if (assetIds.length === 0) {
      ElMessage.warning(t('fixedAssets.message.noInUseAsset'))
      return
    }
    await ElMessageBox.confirm(
      t('fixedAssets.message.batchDepreciateConfirm', { count: assetIds.length, period: inputPeriod }),
      t('fixedAssets.message.batchDepreciateConfirmTitle'),
      {
        type: 'warning',
        confirmButtonText: t('common.confirm'),
        cancelButtonText: t('common.cancel'),
      }
    )
    await batchDepreciateAssets({
      asset_ids: assetIds,
      calculation_date: inputPeriod,
      user_id: userId,
    })
    ElMessage.success(t('fixedAssets.message.batchDepreciateSuccess', { count: assetIds.length }))
    fetchAssets()
  } catch (e) {
    if (e !== 'cancel') {
      const err = e as Error
      ElMessage.error(err.message || t('fixedAssets.message.batchDepreciateFailed'))
    }
  }
}

// V15 P0-S12 修复（Batch 475e）：迁移到后端导出，注入水印 + 审计日志
const handleExport = async () => {
  const params: Record<string, unknown> = {
    keyword: queryParams.value.keyword as string | undefined,
    status: queryParams.value.status as string | undefined,
    asset_category: queryParams.value.asset_category as string | undefined,
  }
  await exportFromBackend('/fixed-assets/export', params, 'fixed_assets_export')
  logger.info('固定资产列表已导出')
}
</script>
