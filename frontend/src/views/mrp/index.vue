<template>
  <div class="mrp-container">
    <el-card class="header-card">
      <div class="header-content">
        <h2>{{ t('mrp.calc.title') }}</h2>
        <p>{{ t('mrp.calc.subtitle') }}</p>
      </div>
    </el-card>

    <!-- 计算参数表单 -->
    <el-card class="form-card">
      <template #header>
        <div class="card-header">
          <span>{{ t('mrp.calc.paramsTitle') }}</span>
        </div>
      </template>

      <el-form
        ref="calcFormRef"
        :model="calcForm"
        :rules="calcRules"
        label-width="120px"
        :aria-label="t('mrp.calc.formAriaLabel')"
      >
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('mrp.calc.productSelect')" prop="product_ids">
              <el-select
                v-model="calcForm.product_ids"
                multiple
                filterable
                remote
                reserve-keyword
                :placeholder="t('mrp.calc.productPlaceholder')"
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
            <el-form-item :label="t('mrp.calc.demandQuantity')" prop="demand_quantity">
              <el-input-number
                v-model="calcForm.demand_quantity"
                :min="1"
                :precision="0"
                style="width: 100%"
              />
            </el-form-item>
          </el-col>
        </el-row>
        <el-row :gutter="20">
          <el-col :span="12">
            <el-form-item :label="t('mrp.calc.demandDate')" prop="demand_date">
              <el-date-picker
                v-model="calcForm.demand_date"
                type="date"
                :placeholder="t('mrp.calc.demandDatePlaceholder')"
                style="width: 100%"
                value-format="YYYY-MM-DD"
              />
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item :label="t('mrp.calc.calcOptions')">
              <el-checkbox v-model="calcForm.consider_safety_stock">{{
                t('mrp.calc.considerSafetyStock')
              }}</el-checkbox>
              <el-checkbox v-model="calcForm.consider_in_transit" style="margin-left: 16px">{{
                t('mrp.calc.considerInTransit')
              }}</el-checkbox>
            </el-form-item>
          </el-col>
        </el-row>
        <el-form-item>
          <el-button type="primary" :loading="calcLoading" @click="handleCalculate">
            <el-icon><Cpu /></el-icon>{{ t('mrp.calc.triggerCalc') }}
          </el-button>
          <el-button @click="resetCalcForm">{{ t('mrp.calc.reset') }}</el-button>
        </el-form-item>
      </el-form>
    </el-card>

    <!-- 计算结果 -->
    <el-card v-if="resultVisible" class="result-card">
      <template #header>
        <div class="card-header">
          <span>{{ t('mrp.calc.materialList') }}</span>
          <div>
            <el-button
              type="success"
              :disabled="selectedMaterials.length === 0"
              @click="handleConvert('purchase')"
            >
              <el-icon><ShoppingCart /></el-icon>{{ t('mrp.calc.convertToPurchase') }}
            </el-button>
            <el-button
              type="primary"
              :disabled="selectedMaterials.length === 0"
              @click="handleConvert('production')"
            >
              <el-icon><Document /></el-icon>{{ t('mrp.calc.convertToProduction') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-table
        v-loading="resultLoading"
        :data="materialList"
        stripe
        border
        :aria-label="t('mrp.calc.resultAriaLabel')"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" />
        <el-table-column prop="material_code" :label="t('mrp.calc.materialCode')" width="140" />
        <el-table-column prop="material_name" :label="t('mrp.calc.materialName')" min-width="160" />
        <el-table-column
          prop="specification"
          :label="t('mrp.calc.specification')"
          min-width="120"
        />
        <el-table-column prop="unit" :label="t('mrp.calc.unit')" width="80" />
        <el-table-column
          prop="required_quantity"
          :label="t('mrp.calc.demandQuantity')"
          width="120"
          align="right"
        />
        <el-table-column
          prop="available_stock"
          :label="t('mrp.calc.availableStock')"
          width="120"
          align="right"
        />
        <el-table-column
          prop="in_transit_quantity"
          :label="t('mrp.calc.inTransitQuantity')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="safety_stock"
          :label="t('mrp.calc.safetyStock')"
          width="100"
          align="right"
        />
        <el-table-column
          prop="net_requirement"
          :label="t('mrp.calc.netRequirement')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            <span :class="{ 'highlight-quantity': row.net_requirement > 0 }">{{
              row.net_requirement
            }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="suggested_order_quantity"
          :label="t('mrp.calc.suggestedOrderQuantity')"
          width="130"
          align="right"
        />
        <el-table-column prop="suggested_date" :label="t('mrp.calc.suggestedDate')" width="130" />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, onMounted } from 'vue'
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus'
import { useI18n } from 'vue-i18n'
import { Cpu, ShoppingCart, Document } from '@element-plus/icons-vue'
import {
  calculateMrp,
  convertToOrder,
  getProductsForMrp,
  type MrpProduct,
  type MrpMaterialRequirement,
} from '../../api/mrp'

const { t } = useI18n({ useScope: 'global' })

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
  product_ids: [
    { required: true, message: t('mrp.calc.productRequired'), trigger: 'change', type: 'array' },
  ],
  demand_quantity: [
    { required: true, message: t('mrp.calc.demandQuantityRequired'), trigger: 'blur' },
  ],
  demand_date: [{ required: true, message: t('mrp.calc.demandDateRequired'), trigger: 'change' }],
}

const searchProducts = async (query: string) => {
  if (query) {
    productLoading.value = true
    try {
      const res = await getProductsForMrp({ keyword: query })
      productOptions.value = res.data || []
    } catch (e: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error(
        (e instanceof Error ? e.message : String(e)) || t('mrp.calc.fetchProductsError')
      )
    } finally {
      productLoading.value = false
    }
  }
}

const handleCalculate = async () => {
  if (!calcFormRef.value) return

  await calcFormRef.value.validate(async valid => {
    if (!valid) return

    calcLoading.value = true
    try {
      const res = await calculateMrp(calcForm)
      materialList.value = res.data.materials || []
      currentCalculationId.value = res.data.calculation_id
      resultVisible.value = true
      selectedMaterials.value = []
      ElMessage.success(t('mrp.calc.calcSuccess'))
    } catch (e: unknown) {
      // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
      ElMessage.error((e instanceof Error ? e.message : String(e)) || t('mrp.calc.calcFailed'))
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

/**
 * 转换订单类型标签
 */
const getOrderTypeLabel = (orderType: 'purchase' | 'production') => {
  return orderType === 'purchase' ? t('mrp.calc.purchaseOrder') : t('mrp.calc.productionOrder')
}

const handleConvert = async (orderType: 'purchase' | 'production') => {
  if (selectedMaterials.value.length === 0) {
    ElMessage.warning(t('mrp.calc.selectMaterialFirst'))
    return
  }

  const typeLabel = getOrderTypeLabel(orderType)

  try {
    await ElMessageBox.confirm(
      t('mrp.calc.convertConfirmMessage', {
        count: selectedMaterials.value.length,
        type: typeLabel,
      }),
      t('mrp.calc.confirmTitle'),
      {
        type: 'warning',
      }
    )

    const materialIds = selectedMaterials.value.map(item => item.id)
    const res = await convertToOrder({
      calculation_id: currentCalculationId.value,
      material_ids: materialIds,
      order_type: orderType,
    })

    ElMessage.success(
      t('mrp.calc.convertSuccess', { count: res.data.order_ids.length, type: typeLabel })
    )
  } catch (e: unknown) {
    // 批次 98 P2-D 修复（v5 复审）：原 catch (e: any) 改为 unknown + 类型守卫
    if (e !== 'cancel') {
      ElMessage.error((e instanceof Error ? e.message : String(e)) || t('mrp.calc.convertFailed'))
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
