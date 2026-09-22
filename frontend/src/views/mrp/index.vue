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
                  :label="`${item.code} - ${item.name}`"
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
          <span>
            {{ t('mrp.calc.materialList') }}
            <el-tag v-if="calculationNo" type="info" size="small">{{ calculationNo }}</el-tag>
          </span>
          <div>
            <el-button
              type="success"
              :disabled="selectedResults.length === 0"
              @click="handleConvert('PURCHASE')"
            >
              <el-icon><ShoppingCart /></el-icon>{{ t('mrp.calc.convertToPurchase') }}
            </el-button>
            <el-button
              type="primary"
              :disabled="selectedResults.length === 0"
              @click="handleConvert('PRODUCTION')"
            >
              <el-icon><Document /></el-icon>{{ t('mrp.calc.convertToProduction') }}
            </el-button>
          </div>
        </div>
      </template>

      <el-table
        v-loading="resultLoading"
        :data="resultRows"
        row-key="id"
        stripe
        border
        :aria-label="t('mrp.calc.resultAriaLabel')"
        @selection-change="handleSelectionChange"
      >
        <el-table-column type="selection" width="55" reserve-selection />
        <el-table-column prop="productCode" :label="t('mrp.calc.materialCode')" width="140" />
        <el-table-column prop="productName" :label="t('mrp.calc.materialName')" min-width="160" />
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
          prop="on_hand_quantity"
          :label="t('mrp.calc.onHandQuantity')"
          width="120"
          align="right"
        />
        <el-table-column
          prop="available_quantity"
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
          prop="shortage_quantity"
          :label="t('mrp.calc.netRequirement')"
          width="120"
          align="right"
        >
          <template #default="{ row }">
            <span :class="{ 'highlight-quantity': row.shortage_quantity > 0 }">{{
              row.shortage_quantity
            }}</span>
          </template>
        </el-table-column>
        <el-table-column
          prop="planned_order_quantity"
          :label="t('mrp.calc.suggestedOrderQuantity')"
          width="130"
          align="right"
        />
        <el-table-column
          prop="planned_order_date"
          :label="t('mrp.calc.suggestedDate')"
          width="130"
        />
      </el-table>
    </el-card>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, computed, onMounted } from 'vue';
import { ElMessage, ElMessageBox, type FormInstance, type FormRules } from 'element-plus';
import { useI18n } from 'vue-i18n';
import { Cpu, ShoppingCart, Document } from '@element-plus/icons-vue';
import {
  calculateMrp,
  convertToOrder,
  getProductsForMrp,
  type MrpProductOption,
  type MrpResultResponse,
  type MrpRequirementRow,
  type MrpOrderType,
} from '../../api/mrp';

const { t } = useI18n({ useScope: 'global' });

/**
 * 结果表展示行：以可转单的 mrp_result 行为主，按 product_id 关联需求行与产品主数据。
 * 后端不返回产品编码/名称，故这些列由 /mrp/products 已加载数据映射补全。
 */
interface ResultRow {
  id: number;
  product_id: number;
  productCode: string;
  productName: string;
  unit: string;
  specification: string;
  required_quantity: number;
  on_hand_quantity: number;
  available_quantity: number;
  in_transit_quantity: number;
  safety_stock: number;
  shortage_quantity: number;
  planned_order_quantity: number | null;
  planned_order_date: string | null;
}

const calcFormRef = ref<FormInstance>();
const calcLoading = ref(false);
const resultLoading = ref(false);
const productLoading = ref(false);
const resultVisible = ref(false);
const calculationNo = ref('');
const productOptions = ref<MrpProductOption[]>([]);
const results = ref<MrpResultResponse[]>([]);
const requirements = ref<MrpRequirementRow[]>([]);
const selectedResults = ref<ResultRow[]>([]);

const calcForm = reactive({
  product_ids: [] as number[],
  demand_quantity: 1,
  demand_date: '',
  consider_safety_stock: true,
  consider_in_transit: true,
});

const calcRules: FormRules = {
  product_ids: [
    { required: true, message: t('mrp.calc.productRequired'), trigger: 'change', type: 'array' },
  ],
  demand_quantity: [
    { required: true, message: t('mrp.calc.demandQuantityRequired'), trigger: 'blur' },
  ],
  demand_date: [{ required: true, message: t('mrp.calc.demandDateRequired'), trigger: 'change' }],
};

/** 累积合并搜索结果：选中产品后再次搜索不会丢失已有选项，保证结果页可按 id 回查主数据 */
const upsertProducts = (incoming: MrpProductOption[]) => {
  const byId = new Map(productOptions.value.map(p => [p.id, p]));
  for (const p of incoming) byId.set(p.id, p);
  productOptions.value = Array.from(byId.values());
};

const searchProducts = async (query: string) => {
  productLoading.value = true;
  try {
    const res = await getProductsForMrp({ keyword: query });
    upsertProducts(res.data);
  } catch (e: unknown) {
    ElMessage.error(
      (e instanceof Error ? e.message : String(e)) || t('mrp.calc.fetchProductsError')
    );
  } finally {
    productLoading.value = false;
  }
};

const resultRows = computed<ResultRow[]>(() => {
  const reqByProduct = new Map<number, MrpRequirementRow>();
  for (const req of requirements.value) {
    if (!reqByProduct.has(req.product_id)) reqByProduct.set(req.product_id, req);
  }
  const productById = new Map(productOptions.value.map(p => [p.id, p]));

  return results.value.map(res => {
    const req = reqByProduct.get(res.product_id);
    const product = productById.get(res.product_id);
    return {
      id: res.id,
      product_id: res.product_id,
      productCode: product?.code ?? `#${res.product_id}`,
      productName: product?.name ?? '',
      unit: product?.unit ?? '',
      specification: product?.specification ?? '',
      required_quantity: Number(res.required_quantity),
      on_hand_quantity: req ? Number(req.on_hand_quantity) : 0,
      available_quantity: req ? Number(req.available_quantity) : 0,
      in_transit_quantity: req ? Number(req.in_transit_quantity) : 0,
      safety_stock: req ? Number(req.safety_stock) : 0,
      shortage_quantity: req ? Number(req.shortage_quantity) : 0,
      planned_order_quantity:
        res.planned_order_quantity != null ? Number(res.planned_order_quantity) : null,
      planned_order_date: res.planned_order_date,
    };
  });
});

const handleCalculate = async () => {
  if (!calcFormRef.value) return;

  await calcFormRef.value.validate(async valid => {
    if (!valid) return;

    calcLoading.value = true;
    try {
      const res = await calculateMrp({ ...calcForm });
      calculationNo.value = res.data.calculation_no;
      results.value = res.data.results;
      requirements.value = res.data.requirements;
      selectedResults.value = [];
      resultVisible.value = true;
      ElMessage.success(t('mrp.calc.calcSuccess'));
    } catch (e: unknown) {
      ElMessage.error((e instanceof Error ? e.message : String(e)) || t('mrp.calc.calcFailed'));
    } finally {
      calcLoading.value = false;
    }
  });
};

const resetCalcForm = () => {
  calcForm.product_ids = [];
  calcForm.demand_quantity = 1;
  calcForm.demand_date = '';
  calcForm.consider_safety_stock = true;
  calcForm.consider_in_transit = true;
  resultVisible.value = false;
  calculationNo.value = '';
  results.value = [];
  requirements.value = [];
  selectedResults.value = [];
  calcFormRef.value?.clearValidate();
};

const handleSelectionChange = (selection: ResultRow[]) => {
  selectedResults.value = selection;
};

const getOrderTypeLabel = (orderType: MrpOrderType) => {
  return orderType === 'PURCHASE' ? t('mrp.calc.purchaseOrder') : t('mrp.calc.productionOrder');
};

const handleConvert = async (orderType: MrpOrderType) => {
  if (selectedResults.value.length === 0) {
    ElMessage.warning(t('mrp.calc.selectMaterialFirst'));
    return;
  }

  const typeLabel = getOrderTypeLabel(orderType);

  try {
    await ElMessageBox.confirm(
      t('mrp.calc.convertConfirmMessage', {
        count: selectedResults.value.length,
        type: typeLabel,
      }),
      t('mrp.calc.confirmTitle'),
      { type: 'warning' }
    );

    const res = await convertToOrder({
      result_ids: selectedResults.value.map(row => row.id),
      order_type: orderType,
    });

    ElMessage.success(t('mrp.calc.convertSuccess', { count: res.data.length, type: typeLabel }));
  } catch (e: unknown) {
    if (e !== 'cancel') {
      ElMessage.error((e instanceof Error ? e.message : String(e)) || t('mrp.calc.convertFailed'));
    }
  }
};

onMounted(() => {
  searchProducts('');
});
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
