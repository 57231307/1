<!--
  CurrencyListTab.vue - 币种管理 Tab
  来源：原 currency/index.vue 主体内容
  拆分日期：2026-06-15 B3-2
-->
<template>
  <div class="currency-list-tab">
    <div class="page-header">
      <h2 class="page-title">币种管理</h2>
      <div>
        <el-button type="primary" @click="openDialog()">
          <el-icon><Plus /></el-icon>新建币种
        </el-button>
        <el-button @click="openRateDialog()">
          <el-icon><Plus /></el-icon>新增汇率
        </el-button>
      </div>
    </div>

    <el-card shadow="hover">
      <el-table v-loading="loading" :data="currencyList" stripe aria-label="币种列表">
        <el-table-column prop="code" label="编码" width="80" />
        <el-table-column prop="name" label="名称" width="120" />
        <el-table-column prop="symbol" label="符号" width="60" align="center" />
        <el-table-column prop="precision" label="精度" width="80" align="center" />
        <el-table-column prop="isBase" label="基准币种" width="100" align="center">
          <template #default="{ row }">
            <el-tag v-if="row.isBase" type="success" size="small">基准</el-tag>
            <span v-else>-</span>
          </template>
        </el-table-column>
        <el-table-column prop="isActive" label="状态" width="80" align="center">
          <template #default="{ row }">
            <el-tag :type="row.isActive ? 'success' : 'info'" size="small">
              {{ row.isActive ? '启用' : '禁用' }}
            </el-tag>
          </template>
        </el-table-column>
        <el-table-column label="操作" width="160" fixed="right">
          <template #default="{ row }">
            <el-button type="primary" link size="small" @click="openRateDialog(row.code)"
              >汇率</el-button
            >
            <el-button v-if="!row.isBase" type="warning" link size="small" @click="setBase(row)"
              >设为基准</el-button
            >
          </template>
        </el-table-column>
      </el-table>
    </el-card>

    <el-dialog v-model="dialogVisible" title="新建币种" width="500px" aria-label="新建币种对话框">
      <el-form ref="formRef" :model="form" :rules="rules" label-width="100px" aria-label="新建币种表单">
        <el-form-item label="币种编码" prop="code">
          <el-input v-model="form.code" placeholder="如 USD" />
        </el-form-item>
        <el-form-item label="币种名称" prop="name">
          <el-input v-model="form.name" placeholder="如 美元" />
        </el-form-item>
        <el-form-item label="币种符号">
          <el-input v-model="form.symbol" placeholder="如 $" />
        </el-form-item>
        <el-form-item label="精度">
          <el-input-number v-model="form.precision" :min="0" :max="6" style="width: 100%" />
        </el-form-item>
        <el-form-item label="基准币种">
          <el-switch v-model="form.isBase" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="dialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSubmit">确定</el-button>
      </template>
    </el-dialog>

    <el-dialog v-model="rateDialogVisible" title="新增汇率" width="500px" aria-label="新增汇率对话框">
      <el-form ref="rateFormRef" :model="rateForm" :rules="rateRules" label-width="100px" aria-label="新增汇率表单">
        <el-form-item label="源币种" prop="fromCurrency">
          <el-select v-model="rateForm.fromCurrency" placeholder="选择源币种" style="width: 100%">
            <el-option
              v-for="c in currencyList"
              :key="c.code"
              :label="`${c.code} - ${c.name}`"
              :value="c.code"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="目标币种" prop="toCurrency">
          <el-select v-model="rateForm.toCurrency" placeholder="选择目标币种" style="width: 100%">
            <el-option
              v-for="c in currencyList"
              :key="c.code"
              :label="`${c.code} - ${c.name}`"
              :value="c.code"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="汇率" prop="rate">
          <el-input-number
            v-model="rateForm.rate"
            :min="0"
            :precision="6"
            :step="0.0001"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="生效日期" prop="effectiveDate">
          <el-date-picker
            v-model="rateForm.effectiveDate"
            type="date"
            placeholder="选择日期"
            value-format="YYYY-MM-DD"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item label="来源">
          <el-input v-model="rateForm.source" placeholder="如 央行" />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="rateDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="rateSubmitLoading" @click="handleRateSubmit"
          >确定</el-button
        >
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Plus } from '@element-plus/icons-vue'
import {
  listCurrencies,
  createCurrency,
  createExchangeRate,
  setBaseCurrency,
  type Currency,
  type CreateCurrencyRequest,
  type CreateExchangeRateRequest,
} from '@/api/currency'

const loading = ref(false)
const submitLoading = ref(false)
const rateSubmitLoading = ref(false)
const dialogVisible = ref(false)
const rateDialogVisible = ref(false)
const currencyList = ref<Currency[]>([])
const formRef = ref<FormInstance>()
const rateFormRef = ref<FormInstance>()

const form = reactive<CreateCurrencyRequest>({
  code: '',
  name: '',
  symbol: '',
  isBase: false,
  precision: 2,
})

const rateForm = reactive<CreateExchangeRateRequest>({
  fromCurrency: '',
  toCurrency: '',
  rate: 1,
  effectiveDate: new Date().toISOString().split('T')[0],
  source: '',
})

const rules: FormRules = {
  code: [{ required: true, message: '请输入币种编码', trigger: 'blur' }],
  name: [{ required: true, message: '请输入币种名称', trigger: 'blur' }],
}

const rateRules: FormRules = {
  fromCurrency: [{ required: true, message: '请选择源币种', trigger: 'change' }],
  toCurrency: [{ required: true, message: '请选择目标币种', trigger: 'change' }],
  rate: [{ required: true, message: '请输入汇率', trigger: 'blur' }],
  effectiveDate: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
}

const fetchCurrencies = async () => {
  loading.value = true
  try {
    const res = await listCurrencies()
    const d = (res as { data?: unknown }).data as
      | Currency[]
      | { items?: Currency[]; data?: Currency[]; list?: Currency[] }
    currencyList.value = Array.isArray(d) ? d : d?.items || d?.data || d?.list || []
  } catch (e) {
    const err = e as Error
    ElMessage.error(err.message || '获取币种列表失败')
  } finally {
    loading.value = false
  }
}

const openDialog = () => {
  formRef.value?.resetFields()
  form.code = ''
  form.name = ''
  form.symbol = ''
  form.isBase = false
  form.precision = 2
  dialogVisible.value = true
}

const handleSubmit = async () => {
  if (!formRef.value) return
  await formRef.value.validate(async valid => {
    if (!valid) return
    submitLoading.value = true
    try {
      await createCurrency(form)
      ElMessage.success('创建成功')
      dialogVisible.value = false
      fetchCurrencies()
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '创建失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const openRateDialog = (defaultFromCode?: string) => {
  rateFormRef.value?.resetFields()
  rateForm.fromCurrency = defaultFromCode || ''
  rateForm.toCurrency = ''
  rateForm.rate = 1
  rateForm.effectiveDate = new Date().toISOString().split('T')[0]
  rateForm.source = ''
  rateDialogVisible.value = true
}

const handleRateSubmit = async () => {
  if (!rateFormRef.value) return
  await rateFormRef.value.validate(async valid => {
    if (!valid) return
    rateSubmitLoading.value = true
    try {
      await createExchangeRate(rateForm)
      ElMessage.success('汇率创建成功')
      rateDialogVisible.value = false
    } catch (e) {
      const err = e as Error
      ElMessage.error(err.message || '汇率创建失败')
    } finally {
      rateSubmitLoading.value = false
    }
  })
}

// 批次 157d-1 修复：接入 setBaseCurrency API
const setBase = async (row: Currency) => {
  if (!row.id) {
    ElMessage.warning('币种 ID 不存在')
    return
  }
  try {
    await ElMessageBox.confirm(
      `确认将 "${row.code} - ${row.name}" 设为基础币种（本位币）吗？此操作会取消其他币种的基础标记。`,
      '设置基础币种',
      { type: 'warning' }
    )
    await setBaseCurrency(row.id)
    ElMessage.success('设置基础币种成功')
    fetchCurrencies()
  } catch (error) {
    if (error !== 'cancel') {
      const err = error as Error
      ElMessage.error(err.message || '设置基础币种失败')
    }
  }
}

onMounted(() => {
  fetchCurrencies()
})
</script>
