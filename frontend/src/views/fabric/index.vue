<!--
  fabric/index.vue - 面料管理主入口（容器组件）
  ----------------------------------------------------------------
  拆分说明（2026-06-15 B3-4）：
  原 729 行"上帝组件"已拆分为以下 3 个 Tab 子组件 + 3 个对话框，
  位于 views/fabric/tabs/ 目录：

  | Tab         | 子组件                              |
  | ----------- | ----------------------------------- |
  | 染色批次    | tabs/DyeTab.vue                     |
  | 坯布管理    | tabs/GreigeTab.vue                  |
  | 染色配方    | tabs/RecipeTab.vue                  |
  | 染色编辑    | tabs/DyeFormDialogTab.vue           |
  | 坯布编辑    | tabs/GreigeFormDialogTab.vue        |
  | 配方编辑    | tabs/RecipeFormDialogTab.vue        |

  本主入口仅承担：Tab 切换 + 坯布入库/出库对话框 + 公共样式。
-->
<template>
  <div class="fabric-page">
    <el-tabs v-model="activeTab">
      <el-tab-pane :label="t('fabric.index.tabDye')" name="dye">
        <DyeTab ref="dyeTabRef" @open-dialog="openDyeDialog" />
      </el-tab-pane>
      <el-tab-pane :label="t('fabric.index.tabGreige')" name="greige">
        <GreigeTab ref="greigeTabRef" @open-dialog="openGreigeDialog" @open-stock="handleStock" />
      </el-tab-pane>
      <el-tab-pane :label="t('fabric.index.tabRecipe')" name="recipe">
        <RecipeTab ref="recipeTabRef" @open-dialog="openRecipeDialog" />
      </el-tab-pane>
    </el-tabs>

    <DyeFormDialogTab
      v-model="dyeDialogVisible"
      :current-row="currentDyeRow"
      :greige-fabrics="greigeFabrics"
      @submitted="handleSubmitted"
    />

    <GreigeFormDialogTab
      v-model="greigeDialogVisible"
      :current-row="currentGreigeRow"
      :suppliers="suppliers"
      @submitted="handleSubmitted"
    />

    <RecipeFormDialogTab
      v-model="recipeDialogVisible"
      :current-row="currentRecipeRow"
      @submitted="handleSubmitted"
    />

    <!--
      坯布入库/出库对话框：字段与后端逐一对应。
      入库 stock_in (handler.rs:110) 必填 warehouse_id + weight_kg + length_m；
      出库 stock_out (handler.rs:123) weight_kg / length_m 至少填一项。
      纺织坯布按重量交易、按长度核米；身份/仓库不给默认值。
    -->
    <el-dialog
      v-model="stockDialogVisible"
      :title="stockType === 'in' ? t('fabric.index.stockInTitle') : t('fabric.index.stockOutTitle')"
      width="480px"
    >
      <el-form ref="stockFormRef" :model="stockForm" :rules="stockRules" label-width="110px">
        <el-form-item
          v-if="stockType === 'in'"
          :label="t('fabric.index.labelWarehouse')"
          prop="warehouse_id"
        >
          <el-select
            v-model="stockForm.warehouse_id"
            :placeholder="t('fabric.index.placeholderWarehouse')"
            style="width: 100%"
          >
            <el-option
              v-for="w in warehouses"
              :key="w.id"
              :label="w.warehouse_name"
              :value="w.id"
            />
          </el-select>
        </el-form-item>
        <el-form-item :label="t('fabric.index.labelWeightKg')" prop="weight_kg">
          <el-input-number
            v-model="stockForm.weight_kg"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
        <el-form-item :label="t('fabric.index.labelLengthM')" prop="length_m">
          <el-input-number
            v-model="stockForm.length_m"
            :min="0"
            :precision="2"
            style="width: 100%"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="stockDialogVisible = false">{{ t('fabric.common.cancel') }}</el-button>
        <el-button type="primary" :loading="stockLoading" @click="submitStock">
          {{ t('fabric.common.confirm') }}
        </el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, reactive, provide, computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  stockInGreigeFabric,
  stockOutGreigeFabric,
  type GreigeFabric,
  type GreigeStockInPayload,
  type GreigeStockOutPayload,
} from '@/api/greige-fabric';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { getWarehouseList, type Warehouse } from '@/api/warehouse';
import type { DyeBatch } from '@/api/dye-batch';
import type { DyeRecipe as ApiDyeRecipe } from '@/api/dye-recipe';
import { logger } from '@/utils/logger';
import DyeTab from './tabs/DyeTab.vue';
import GreigeTab from './tabs/GreigeTab.vue';
import RecipeTab from './tabs/RecipeTab.vue';
import DyeFormDialogTab from './tabs/DyeFormDialogTab.vue';
import GreigeFormDialogTab from './tabs/GreigeFormDialogTab.vue';
import RecipeFormDialogTab from './tabs/RecipeFormDialogTab.vue';

const { t } = useI18n({ useScope: 'global' });

const activeTab = ref('dye');

const dyeDialogVisible = ref(false);
const currentDyeRow = ref<DyeBatch | null>(null);
const greigeFabrics = ref<GreigeFabric[]>([]);

const greigeDialogVisible = ref(false);
const currentGreigeRow = ref<GreigeFabric | null>(null);
const suppliers = ref<Supplier[]>([]);

const recipeDialogVisible = ref(false);
const currentRecipeRow = ref<ApiDyeRecipe | null>(null);

const greigeTabRef = ref<InstanceType<typeof GreigeTab>>();
const dyeTabRef = ref<InstanceType<typeof DyeTab>>();
const recipeTabRef = ref<InstanceType<typeof RecipeTab>>();

// ---- 坯布入库/出库对话框 ----
const stockDialogVisible = ref(false);
const stockType = ref<'in' | 'out'>('in');
const stockRow = ref<GreigeFabric | null>(null);
const warehouses = ref<Warehouse[]>([]);
const stockLoading = ref(false);
const stockFormRef = ref<FormInstance>();
const stockForm = reactive({
  warehouse_id: undefined as number | undefined,
  weight_kg: undefined as number | undefined,
  length_m: undefined as number | undefined,
});

// 入库：warehouse_id + weight_kg + length_m 全部必填（对齐后端 StockInRequest）。
// 出库：后端 weight_kg / length_m 均为 Optional，业务上至少填一项——故此处不加 required
// 规则，改由 submitStock 前置校验"至少一项正数"，避免 form 规则误拦截只填一项的合法出库。
const stockRules = computed<FormRules>(() =>
  stockType.value === 'in'
    ? {
        warehouse_id: [
          {
            required: true,
            message: () => t('fabric.index.messageWarehouseRequired'),
            trigger: 'change',
          },
        ],
        weight_kg: [
          {
            required: true,
            message: () => t('fabric.index.messageWeightMustBePositive'),
            trigger: 'blur',
          },
        ],
        length_m: [
          {
            required: true,
            message: () => t('fabric.index.messageLengthMustBePositive'),
            trigger: 'blur',
          },
        ],
      }
    : {}
);

const fetchGreigeFabrics = async () => {
  try {
    const { getGreigeFabricList } = await import('@/api/greige-fabric');
    const res = await getGreigeFabricList();
    // 后端返回 PaginatedResponse：原写法把整个信封当数组取 ⇒ 恒空
    greigeFabrics.value = res.data.items;
  } catch (error) {
    const err = error as Error;
    logger.error(t('fabric.index.fetchGreigeFabricsFailed'), err.message);
  }
};

const fetchSuppliers = async () => {
  try {
    const res = await getSupplierList();
    suppliers.value = (res.data?.items as Supplier[] | undefined) || [];
  } catch (error) {
    const err = error as Error;
    logger.error(t('fabric.index.fetchSuppliersFailed'), err.message);
  }
};

const fetchWarehouses = async () => {
  try {
    const res = await getWarehouseList();
    warehouses.value = (res.data?.items as Warehouse[] | undefined) || [];
  } catch (error) {
    const err = error as Error;
    logger.error(t('fabric.index.fetchWarehousesFailed'), err.message);
  }
};

const openDyeDialog = (row: DyeBatch | null) => {
  currentDyeRow.value = row;
  dyeDialogVisible.value = true;
  fetchGreigeFabrics();
};

const openGreigeDialog = (row: GreigeFabric | null) => {
  currentGreigeRow.value = row;
  greigeDialogVisible.value = true;
  fetchSuppliers();
};

const openRecipeDialog = (row: ApiDyeRecipe | null) => {
  currentRecipeRow.value = row;
  recipeDialogVisible.value = true;
};

const handleStock = async (type: 'in' | 'out', row: GreigeFabric) => {
  stockType.value = type;
  stockRow.value = row;
  stockForm.warehouse_id = undefined;
  stockForm.weight_kg = undefined;
  stockForm.length_m = undefined;
  stockFormRef.value?.clearValidate();
  if (type === 'in') fetchWarehouses();
  stockDialogVisible.value = true;
};

const positive = (v: number | undefined): v is number => typeof v === 'number' && v > 0;

const submitStock = async () => {
  if (!stockRow.value) return;
  // 出库：weight_kg / length_m 至少填一项（后端两者皆 Optional）
  if (
    stockType.value === 'out' &&
    !positive(stockForm.weight_kg) &&
    !positive(stockForm.length_m)
  ) {
    ElMessage.error(t('fabric.index.messageAtLeastOneOfWeightOrLength'));
    return;
  }
  const ok = await stockFormRef.value?.validate().catch(() => false);
  if (!ok) return;
  if (
    stockType.value === 'in' &&
    (!positive(stockForm.weight_kg) || !positive(stockForm.length_m))
  ) {
    ElMessage.error(t('fabric.index.messageWeightAndLengthMustBePositive'));
    return;
  }
  stockLoading.value = true;
  try {
    if (stockType.value === 'in') {
      // stockForm 已通过上面的正数校验收敛为必填值
      const payload: GreigeStockInPayload = {
        warehouse_id: stockForm.warehouse_id as number,
        weight_kg: stockForm.weight_kg as number,
        length_m: stockForm.length_m as number,
      };
      await stockInGreigeFabric(stockRow.value.id, payload);
      ElMessage.success(t('fabric.index.messageStockInSuccess'));
    } else {
      const payload: GreigeStockOutPayload = {};
      if (positive(stockForm.weight_kg)) payload.weight_kg = stockForm.weight_kg;
      if (positive(stockForm.length_m)) payload.length_m = stockForm.length_m;
      await stockOutGreigeFabric(stockRow.value.id, payload);
      ElMessage.success(t('fabric.index.messageStockOutSuccess'));
    }
    stockDialogVisible.value = false;
    greigeTabRef.value?.fetchFabrics();
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('fabric.common.failed'));
  } finally {
    stockLoading.value = false;
  }
};

const handleSubmitted = () => {
  if (activeTab.value === 'greige') {
    greigeTabRef.value?.fetchFabrics();
  } else if (activeTab.value === 'dye') {
    dyeTabRef.value?.fetchBatches();
  } else if (activeTab.value === 'recipe') {
    recipeTabRef.value?.fetchRecipes();
  }
};

provide('fabricActions', {
  openDyeDialog,
  openGreigeDialog,
  openRecipeDialog,
});
</script>

<style scoped>
.fabric-page {
  padding: 24px;
  background-color: #f5f7fa;
  min-height: 100%;
}
:deep(.page-header) {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
:deep(.page-title) {
  font-size: 20px;
  font-weight: 600;
  color: #303133;
  margin: 0;
}
</style>
