<template>
  <div class="currency">
    <el-card>
      <template #header>
        <div class="card-header">
          <span>多币种管理</span>
        </div>
      </template>

      <el-tabs v-model="activeTab">
        <el-tab-pane label="币种列表" name="currencies">
          <div class="toolbar">
            <el-button type="primary" @click="handleCreateCurrency">新建币种</el-button>
          </div>

          <el-table :data="currencyList" border stripe>
            <el-table-column prop="code" label="币种代码" width="120">
              <template #default="{ row }">
                <strong>{{ row.code }}</strong>
              </template>
            </el-table-column>
            <el-table-column prop="name" label="币种名称" />
            <el-table-column prop="symbol" label="符号" width="100" />
            <el-table-column prop="isBase" label="是否本位币" width="120">
              <template #default="{ row }">
                <el-tag v-if="row.isBase" type="success">是</el-tag>
                <span v-else>否</span>
              </template>
            </el-table-column>
            <el-table-column prop="precision" label="精度" width="100" />
            <el-table-column prop="isActive" label="状态" width="100">
              <template #default="{ row }">
                <el-tag v-if="row.isActive" type="success">启用</el-tag>
                <el-tag v-else type="danger">禁用</el-tag>
              </template>
            </el-table-column>
          </el-table>
        </el-tab-pane>

        <el-tab-pane label="汇率管理" name="exchange-rates">
          <div class="toolbar">
            <el-button type="primary" @click="handleCreateExchangeRate">新建汇率</el-button>
          </div>

          <div class="rate-query">
            <el-form inline>
              <el-form-item label="源币种">
                <el-select v-model="rateQuery.fromCurrency" placeholder="请选择">
                  <el-option
                    v-for="curr in currencyList"
                    :key="curr.code"
                    :label="curr.name"
                    :value="curr.code"
                  />
                </el-select>
              </el-form-item>
              <el-form-item label="目标币种">
                <el-select v-model="rateQuery.toCurrency" placeholder="请选择">
                  <el-option
                    v-for="curr in currencyList"
                    :key="curr.code"
                    :label="curr.name"
                    :value="curr.code"
                  />
                </el-select>
              </el-form-item>
              <el-form-item label="日期">
                <el-date-picker
                  v-model="rateQuery.date"
                  type="date"
                  placeholder="选择日期"
                  value-format="YYYY-MM-DD"
                />
              </el-form-item>
              <el-form-item>
                <el-button type="primary" @click="fetchExchangeRate">查询汇率</el-button>
              </el-form-item>
            </el-form>
          </div>

          <el-card v-if="currentRate" class="rate-display">
            <div class="rate-content">
              <div class="rate-pair">
                <span class="currency-code">{{ currentRate.fromCurrency }}</span>
                <span class="arrow">→</span>
                <span class="currency-code">{{ currentRate.toCurrency }}</span>
              </div>
              <div class="rate-value">{{ currentRate.rate }}</div>
              <div class="rate-date">生效日期：{{ currentRate.effectiveDate }}</div>
            </div>
          </el-card>
        </el-tab-pane>
      </el-tabs>
    </el-card>

    <!-- 新建币种对话框 -->
    <el-dialog v-model="currencyDialogVisible" title="新建币种" width="500px">
      <el-form
        ref="currencyFormRef"
        :model="currencyForm"
        :rules="currencyRules"
        label-width="120px"
      >
        <el-form-item label="币种代码" prop="code">
          <el-input v-model="currencyForm.code" placeholder="例如：USD, CNY, EUR" maxlength="10" />
        </el-form-item>
        <el-form-item label="币种名称" prop="name">
          <el-input v-model="currencyForm.name" placeholder="例如：美元, 人民币, 欧元" />
        </el-form-item>
        <el-form-item label="符号" prop="symbol">
          <el-input v-model="currencyForm.symbol" placeholder="例如：$, ¥, €" />
        </el-form-item>
        <el-form-item label="是否本位币" prop="isBase">
          <el-switch v-model="currencyForm.isBase" />
        </el-form-item>
        <el-form-item label="精度" prop="precision">
          <el-input-number
            v-model="currencyForm.precision"
            :min="0"
            :max="10"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="currencyDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSaveCurrency"
          >保存</el-button
        >
      </template>
    </el-dialog>

    <!-- 新建汇率对话框 -->
    <el-dialog v-model="rateDialogVisible" title="新建汇率" width="500px">
      <el-form ref="rateFormRef" :model="rateForm" :rules="rateRules" label-width="120px">
        <el-form-item label="源币种" prop="fromCurrency">
          <el-select v-model="rateForm.fromCurrency" placeholder="请选择" style="width: 100%">
            <el-option
              v-for="curr in currencyList"
              :key="curr.code"
              :label="curr.name"
              :value="curr.code"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="目标币种" prop="toCurrency">
          <el-select v-model="rateForm.toCurrency" placeholder="请选择" style="width: 100%">
            <el-option
              v-for="curr in currencyList"
              :key="curr.code"
              :label="curr.name"
              :value="curr.code"
            />
          </el-select>
        </el-form-item>
        <el-form-item label="汇率" prop="rate">
          <el-input-number v-model="rateForm.rate" :min="0" :precision="6" style="width: 100%" />
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
        <el-form-item label="来源" prop="source">
          <el-input v-model="rateForm.source" placeholder="可选" />
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="rateDialogVisible = false">取消</el-button>
        <el-button type="primary" :loading="submitLoading" @click="handleSaveRate">保存</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage } from 'element-plus'
import {
  listCurrencies,
  createCurrency,
  createExchangeRate,
  getExchangeRate,
  type Currency,
  type ExchangeRate,
} from '@/api/currency'

const activeTab = ref('currencies')
const currencyList = ref<Currency[]>([])
const currentRate = ref<ExchangeRate | null>(null)

const currencyDialogVisible = ref(false)
const rateDialogVisible = ref(false)
const submitLoading = ref(false)
const currencyFormRef = ref()
const rateFormRef = ref()

const currencyForm = reactive({
  code: '',
  name: '',
  symbol: '',
  isBase: false,
  precision: 2,
})

const rateForm = reactive({
  fromCurrency: '',
  toCurrency: '',
  rate: 0,
  effectiveDate: '',
  source: '',
})

const rateQuery = reactive({
  fromCurrency: '',
  toCurrency: '',
  date: '',
})

const currencyRules = {
  code: [{ required: true, message: '请输入币种代码', trigger: 'blur' }],
  name: [{ required: true, message: '请输入币种名称', trigger: 'blur' }],
  precision: [{ required: true, message: '请输入精度', trigger: 'blur' }],
}

const rateRules = {
  fromCurrency: [{ required: true, message: '请选择源币种', trigger: 'change' }],
  toCurrency: [{ required: true, message: '请选择目标币种', trigger: 'change' }],
  rate: [{ required: true, message: '请输入汇率', trigger: 'blur' }],
  effectiveDate: [{ required: true, message: '请选择生效日期', trigger: 'change' }],
}

const fetchCurrencies = async () => {
  try {
    const res: any = await listCurrencies()
    if (res.data) {
      currencyList.value = res.data! || []
    }
  } catch (e) {
    ElMessage.error('获取币种列表失败')
  }
}

const fetchExchangeRate = async () => {
  if (!rateQuery.fromCurrency || !rateQuery.toCurrency) {
    ElMessage.warning('请选择源币种和目标币种')
    return
  }

  try {
    const res: any = await getExchangeRate({
      fromCurrency: rateQuery.fromCurrency,
      toCurrency: rateQuery.toCurrency,
      date: rateQuery.date,
    })
    if (res.data) {
      currentRate.value = res.data!
    }
  } catch (e) {
    ElMessage.error('获取汇率失败')
  }
}

const handleCreateCurrency = () => {
  Object.assign(currencyForm, { code: '', name: '', symbol: '', isBase: false, precision: 2 })
  currencyDialogVisible.value = true
}

const handleSaveCurrency = async () => {
  if (!currencyFormRef.value) return

  await currencyFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      await createCurrency(currencyForm as any)
      ElMessage.success('创建成功')
      currencyDialogVisible.value = false
      fetchCurrencies()
    } catch (e: any) {
      ElMessage.error(e.message || '创建失败')
    } finally {
      submitLoading.value = false
    }
  })
}

const handleCreateExchangeRate = () => {
  Object.assign(rateForm, { fromCurrency: '', toCurrency: '', rate: 0, effectiveDate: '' })
  rateDialogVisible.value = true
}

const handleSaveRate = async () => {
  if (!rateFormRef.value) return

  await rateFormRef.value.validate(async (valid: boolean) => {
    if (!valid) return

    submitLoading.value = true
    try {
      await createExchangeRate(rateForm as any)
      ElMessage.success('创建成功')
      rateDialogVisible.value = false
    } catch (e: any) {
      ElMessage.error(e.message || '创建失败')
    } finally {
      submitLoading.value = false
    }
  })
}

onMounted(() => {
  fetchCurrencies()
})
</script>

<style scoped>
.currency .card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.currency .toolbar {
  margin-bottom: 16px;
}

.currency .rate-query {
  margin-bottom: 20px;
}

.currency .rate-display .rate-content {
  text-align: center;
  padding: 20px;
}

.currency .rate-display .rate-content .rate-pair {
  font-size: 18px;
  margin-bottom: 16px;
}

.currency .rate-display .rate-content .rate-pair .currency-code {
  font-weight: bold;
  font-size: 24px;
}

.currency .rate-display .rate-content .rate-pair .arrow {
  margin: 0 16px;
  color: #909399;
}

.currency .rate-display .rate-content .rate-value {
  font-size: 36px;
  font-weight: bold;
  color: #409eff;
  margin-bottom: 12px;
}

.currency .rate-display .rate-content .rate-date {
  color: #909399;
  font-size: 14px;
}
</style>
