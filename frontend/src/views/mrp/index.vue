<template>
  <div class="mrp-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>MRP 物料需求计算</h2>
        <p>根据产品需求自动计算物料需求清单</p>
      </div>
    </el-card>

    <!-- 计算参数表单 -->
    <el-card class="form-card">
      <template #header>
        <div class="card-header">
          <span>计算参数</span>
        </div>
      </template>

      <el-form :model="calcForm" :rules="calcRules" ref="calcFormRef" label-width="120px">
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="产品选择" prop="product_ids">
              <el-select
                v-model="calcForm.product_ids"
                multiple
                filterable
                remote
                reserve-keyword
                placeholder="请输入产品名称搜索"
                :remote-method="searchProducts"
                :loading="productLoading"
                style="width: 100%"
              >
                <el-option
                  v-for="item in productOptions"
                  :key="item.id"
                  :label="`${item.product_code} - ${item.product_name}`"
                  :value="item.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="需求数量" prop="demand_quantity">
              <el-input-number v-model="calcForm.demand_quantity" :min="1" :precision="0" style="width: 100%" />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item label="需求日期" prop="demand_date">
              <el-date-picker
                v-model="calcForm.demand_date"
                type="date"
                placeholder="请选择需求日期"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item label="计算选项">
              <el-checkbox v-model="calcForm.consider_safety_stock">考虑安全库存</el-checkbox>
              <el-checkbox v-model="calcForm.consider_in_transit" style="margin-left: 16px">考虑在途量</el-checkbox>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item>
          <el-button type="primary" :loading="calcLoading" @click="handleCalculate">
            <el-icon><Cpu /></el-icon>触发计算
          </el-button>
          <el-button @click="resetCalcForm">重置</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 计算结果 -->
    <el-card class="result-card" v-if="resultVisible">
      <template #header>
        <div class="card-header">
          <span>物料需求清单</span>
          <div>
            <el-button type="success" :disabled="selectedMaterials.length === 0" @click="handleConvert('purchase')">
              <el-icon><ShoppingCart /></el-icon>转为采购订单
            </el-button>
            <el-button type="primary" :disabled="selectedMaterials.length === 0" @click="handleConvert('production')">
              <el-icon><Document /></el-icon>转为生产订单
            </el-button>
          </div>
        </div>
      </template>

      <el-table
        :data="materialList"
        v-loading="resultLoading"
        stripe
        border
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" />
        <el-table-column prop="material_code" label="物料编码" width="140" />
        <el-table-column prop="material_name" label="物料名称" min-width="160" />
        <el-table-column prop="specification" label="规格" min-width="120" />
        <el-table-column prop="unit" label="单位" width="80" />
        <el-table-column prop="required_quantity" label="需求数量" width="120" align="right" />
        <el-table-column prop="available_stock" label="可用库存" width="120" align="right" />
        <el-table-column prop="in_transit_quantity" label="在途量" width="100" align="right" />
        <el-table-column prop="safety_stock" label="安全库存" width="100" align="right" />
        <el-table-column prop="net_requirement" label="净需求" width="120" align="right">
          <template #default="{ row }">
            <span :class="{ 'highlight-quantity': row.net_requirement > 0 }">{{ row.net_requirement }}</span>
          </template>
        </el-table-column>
        <el-table-column prop="suggested_order_quantity" label="建议订单量" width="130" align="right" />
        <el-table-column prop="suggested_date" label="建议日期" width="130" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { Cpu, ShoppingCart, Document } from '@element-plus/icons-vue'
import {
  calculateMrp,
  convertToOrder,
  getProductsForMrp,
  type MrpProduct,
  type MrpMaterialRequirement,
} from '../../api/mrp'

const calcFormRef = ref<FormInstance>()
const calcLoading = ref(false)
const resultLoading = ref(false)
const productLoading = ref(false)
const resultVisible = ref(false)
const productOptions = ref<MrpProduct[]>([])
const materialList = ref<MrpMaterialRequirement[]>([])
const selectedMaterials = ref<MrpMaterialRequirement[]>([])
const currentCalculationId = ref<number>(0)

const calcForm = reactive({
  product_ids: [] as number[],
  demand_quantity: 1,
  demand_date: '',
  consider_safety_stock: true,
  consider_in_transit: true,
})

const calcRules: FormRules = {
  product_ids: [{ required: true, message: '请选择产品', trigger: 'change', type: 'array' as const }],
  demand_quantity: [{ required: true, message: '请输入需求数量', trigger: 'blur' }],
  demand_date: [{ required: true, message: '请选择需求日期', trigger: 'change' }],
}

const searchProducts = async (query: string) => {
  if (query) {
    productLoading.value = true
    try {
      const res = await getProductsForMrp({ keyword: query })
      productOptions.value = res.data || []
    } catch (e: any) {
      ElMessage.error(e.message || '获取产品列表失败')
    } finally {
      productLoading.value = false
    }
  }
}

const handleCalculate = async () => {
  if (!calcFormRef.value) return

  await calcFormRef.value.validate(async (valid) => {
    if (!valid) return

    calcLoading.value = true
    try {
      const res = await calculateMrp(calcForm)
      materialList.value = res.data.materials || []
      currentCalculationId.value = res.data.calculation_id
      resultVisible.value = true
      selectedMaterials.value = []
      ElMessage.success('MRP 计算完成')
    } catch (e: any) {
      ElMessage.error(e.message || 'MRP 计算失败')
    } finally {
      calcLoading.value = false
    }
  })
}

const resetCalcForm = () => {
  calcForm.product_ids = []
  calcForm.demand_quantity = 1
  calcForm.demand_date = ''
  calcForm.consider_safety_stock = true
  calcForm.consider_in_transit = true
  resultVisible.value = false
  materialList.value = []
  calcFormRef.value?.clearValidate()
}

const handleSelectionChange = (selection: MrpMaterialRequirement[]) => {
  selectedMaterials.value = selection
}

const handleConvert = async (orderType: 'purchase' | 'production') => {
  if (selectedMaterials.value.length === 0) {
    ElMessage.warning('请选择要转换的物料')
    return
  }

  const typeLabel = orderType === 'purchase' ? '采购订单' : '生产订单'

  try {
    await ElMessageBox.confirm(`确认将选中的 ${selectedMaterials.value.length} 项物料转为${typeLabel}吗？`, '确认', {
      type: 'warning',
    })

    const materialIds = selectedMaterials.value.map((item) => item.id)
    const res = await convertToOrder({
      calculation_id: currentCalculationId.value,
      material_ids: materialIds,
      order_type: orderType,
    })

    ElMessage.success(`成功创建 ${res.data.order_ids.length} 个${typeLabel}`)
  } catch (e: any) {
    if (e !== 'cancel') {
      ElMessage.error(e.message || '转换失败')
    }
  }
}

onMounted(() => {
  searchProducts('')
})
</script>

<style scoped>
.mrp-container {
  padding: 20px;
}

.header-card {
  margin-bottom: 20px;
}

.header-content h2 {
  margin: 0 0 8px 0;
  color: #303133;
}

.header-content p {
  margin: 0;
  color: #909399;
}

.form-card {
  margin-bottom: 20px;
}

.result-card {
  margin-bottom: 20px;
}

.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.highlight-quantity {
  color: #e6a23c;
  font-weight: bold;
}
</style>
