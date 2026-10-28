<template>
  <el-dialog
    :model-value="visible"
    :title="dialogTitle"
    width="880px"
    top="6vh"
    :close-on-click-modal="false"
    :aria-label="dialogTitle"
    @update:model-value="emit('update:visible', $event)"
    @closed="handleClosed"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="120px"
      :aria-label="dialogTitle"
      data-testid="greige-form"
    >
      <el-divider content-position="left">{{ t('greigeFabrics.form.sectionBasic') }}</el-divider>
      <el-row :gutter="16">
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colFabricCode')" prop="fabric_no">
            <!-- 编辑态编号不可改：后端 UpdateGreigeFabricRequest 无 fabric_no 键 -->
            <el-input
              v-model="formData.fabric_no"
              :disabled="isEdit"
              :placeholder="t('greigeFabrics.index.placeholderFabricNoAuto')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colFabricName')" prop="fabric_name">
            <el-input
              v-model="formData.fabric_name"
              :placeholder="t('greigeFabrics.index.placeholderFabricName')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colFabricType')" prop="fabric_type">
            <el-input
              v-model="formData.fabric_type"
              :placeholder="t('greigeFabrics.index.placeholderFabricType')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colStatus')" prop="status">
            <el-select
              v-model="formData.status"
              :placeholder="t('greigeFabrics.index.placeholderSelectStatus')"
              style="width: 100%"
            >
              <el-option :label="GREIGE_STATUS.IN_STOCK" :value="GREIGE_STATUS.IN_STOCK" />
              <el-option :label="GREIGE_STATUS.OUT_OF_STOCK" :value="GREIGE_STATUS.OUT_OF_STOCK" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelColorCode')" prop="color_code">
            <el-input v-model="formData.color_code" clearable />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider content-position="left">
        {{ t('greigeFabrics.form.sectionSource') }}
      </el-divider>
      <el-row :gutter="16">
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colSupplier')" prop="supplier_id">
            <el-select
              v-model="formData.supplier_id"
              filterable
              remote
              clearable
              :remote-show-suffix="true"
              :loading="supplierLoading"
              :remote-method="searchSuppliers"
              :placeholder="t('greigeFabrics.form.placeholderRemoteSearch')"
              :no-data-text="t('greigeFabrics.picker.noData')"
              style="width: 100%"
              @change="onSupplierChange"
            >
              <el-option
                v-for="item in supplierOptions"
                :key="item.id"
                :label="item.label"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelProduct')" prop="product_id">
            <el-select
              v-model="formData.product_id"
              filterable
              remote
              clearable
              :disabled="isEdit"
              :loading="productLoading"
              :remote-method="searchProducts"
              :remote-show-suffix="true"
              :placeholder="productPlaceholder"
              :no-data-text="t('greigeFabrics.picker.noData')"
              style="width: 100%"
              @change="formData.product_id = typeof $event === 'number' ? $event : null"
            >
              <el-option
                v-for="item in productOptions"
                :key="item.id"
                :label="item.label"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.labelWarehouse')" prop="warehouse_id">
            <el-select
              v-model="formData.warehouse_id"
              filterable
              remote
              clearable
              :loading="warehouseLoading"
              :remote-method="searchWarehouses"
              :remote-show-suffix="true"
              :placeholder="t('greigeFabrics.form.placeholderRemoteSearch')"
              :no-data-text="t('greigeFabrics.picker.noData')"
              style="width: 100%"
              @change="onWarehouseChange"
            >
              <el-option
                v-for="item in warehouseOptions"
                :key="item.id"
                :label="item.label"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelLocation')" prop="location">
            <el-input v-model="formData.location" clearable />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider content-position="left">
        {{ t('greigeFabrics.form.sectionSpec') }}
        <span v-if="isEdit" class="section-hint">{{ t('greigeFabrics.form.lockedHint') }}</span>
      </el-divider>
      <el-row :gutter="16">
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colComposition')" prop="composition">
            <el-input v-model="formData.composition" :disabled="isEdit" clearable />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelYarnCount')" prop="yarn_count">
            <el-input v-model="formData.yarn_count" :disabled="isEdit" clearable />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelDensity')" prop="density">
            <el-input v-model="formData.density" :disabled="isEdit" clearable />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelStructure')" prop="structure">
            <el-input v-model="formData.structure" :disabled="isEdit" clearable />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelWidthM')" prop="width">
            <el-input
              v-model="formData.width"
              :disabled="isEdit"
              :placeholder="t('greigeFabrics.form.placeholderUnit', { unit: 'm' })"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelWidthCm')" prop="width_cm">
            <el-input
              v-model="formData.width_cm"
              :placeholder="t('greigeFabrics.form.placeholderUnit', { unit: 'cm' })"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelGramWeight')" prop="gram_weight">
            <el-input
              v-model="formData.gram_weight"
              :disabled="isEdit"
              :placeholder="t('greigeFabrics.form.placeholderUnit', { unit: 'g/m²' })"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelProductionDate')" prop="production_date">
            <el-date-picker
              v-model="formData.production_date"
              type="date"
              value-format="YYYY-MM-DD"
              :disabled="isEdit"
              clearable
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelPurchaseDate')" prop="purchase_date">
            <el-date-picker
              v-model="formData.purchase_date"
              type="date"
              value-format="YYYY-MM-DD"
              :disabled="isEdit"
              clearable
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider content-position="left">
        {{ t('greigeFabrics.form.sectionStock') }}
      </el-divider>
      <el-row :gutter="16">
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colWeightKg')" prop="weight_kg">
            <el-input v-model="formData.weight_kg" :placeholder="unitKg" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.index.colLengthM')" prop="length_m">
            <el-input v-model="formData.length_m" :placeholder="unitM" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelQuantityKg')" prop="quantity_kg">
            <el-input v-model="formData.quantity_kg" :disabled="isEdit" :placeholder="unitKg" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelQuantityMeters')" prop="quantity_meters">
            <el-input v-model="formData.quantity_meters" :disabled="isEdit" :placeholder="unitM" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelQualityGrade')" prop="quality_grade">
            <el-select
              v-model="formData.quality_grade"
              clearable
              :placeholder="t('greigeFabrics.form.placeholderSelectQualityGrade')"
              style="width: 100%"
            >
              <el-option v-for="g in QUALITY_GRADES" :key="g" :label="g" :value="g" />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelBatchNo')" prop="batch_no">
            <el-input v-model="formData.batch_no" clearable />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider content-position="left">
        {{ t('greigeFabrics.form.sectionPolicy') }}
      </el-divider>
      <el-row :gutter="16">
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelSafetyStock')" prop="safety_stock">
            <el-input v-model="formData.safety_stock" :placeholder="unitKg" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelReorderPoint')" prop="reorder_point">
            <el-input v-model="formData.reorder_point" :placeholder="unitKg" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelMaxStockPoint')" prop="max_stock_point">
            <el-input v-model="formData.max_stock_point" :placeholder="unitKg" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('greigeFabrics.form.labelReorderQuantity')"
            prop="reorder_quantity"
          >
            <el-input v-model="formData.reorder_quantity" :placeholder="unitKg" />
          </el-form-item>
        </el-col>
      </el-row>

      <el-divider content-position="left">
        {{ t('greigeFabrics.form.sectionTrace') }}
      </el-divider>
      <el-row :gutter="16">
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelColorNo')" prop="color_no">
            <el-input
              v-model="formData.color_no"
              :placeholder="t('greigeFabrics.form.placeholderColorNo')"
              clearable
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('greigeFabrics.form.labelDyeLotNo')" prop="dye_lot_no">
            <el-input
              v-model="formData.dye_lot_no"
              :placeholder="t('greigeFabrics.form.placeholderDyeLotNo')"
              clearable
            />
          </el-form-item>
        </el-col>
      </el-row>

      <template v-if="!isEdit">
        <el-divider content-position="left">
          {{ t('greigeFabrics.form.sectionPurchaseRef') }}
        </el-divider>
        <el-row :gutter="16">
          <el-col :span="12">
            <el-form-item
              :label="t('greigeFabrics.form.labelPurchaseOrder')"
              prop="purchase_order_id"
            >
              <el-select
                v-model="formData.purchase_order_id"
                filterable
                remote
                clearable
                :loading="purchaseOrderLoading"
                :remote-method="searchPurchaseOrders"
                :remote-show-suffix="true"
                :placeholder="t('greigeFabrics.form.placeholderRemoteSearch')"
                :no-data-text="t('greigeFabrics.picker.noData')"
                style="width: 100%"
                @change="formData.purchase_order_id = typeof $event === 'number' ? $event : null"
              >
                <el-option
                  v-for="item in purchaseOrderOptions"
                  :key="item.id"
                  :label="item.label"
                  :value="item.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
          <el-col :span="12">
            <el-form-item
              :label="t('greigeFabrics.form.labelPurchaseReceipt')"
              prop="purchase_receipt_id"
            >
              <el-select
                v-model="formData.purchase_receipt_id"
                filterable
                clearable
                :loading="receiptLoading"
                :placeholder="receiptPlaceholder"
                :no-data-text="t('greigeFabrics.picker.noData')"
                style="width: 100%"
                @change="formData.purchase_receipt_id = typeof $event === 'number' ? $event : null"
              >
                <el-option
                  v-for="item in receiptOptions"
                  :key="item.id"
                  :label="item.label"
                  :value="item.id"
                />
              </el-select>
            </el-form-item>
          </el-col>
        </el-row>
      </template>

      <el-divider content-position="left">{{ t('greigeFabrics.form.sectionRemarks') }}</el-divider>
      <el-form-item :label="t('greigeFabrics.form.labelRemarks')" prop="remarks">
        <el-input v-model="formData.remarks" type="textarea" :rows="2" clearable />
      </el-form-item>
    </el-form>

    <template #footer>
      <el-button @click="emit('update:visible', false)">
        {{ t('greigeFabrics.index.buttonCancel') }}
      </el-button>
      <el-button type="primary" :loading="submitting" @click="handleSubmit">
        {{ t('greigeFabrics.index.buttonConfirm') }}
      </el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import type { FormInstance, FormRules } from 'element-plus';

import {
  GREIGE_STATUS,
  createGreigeFabric,
  updateGreigeFabric,
  type CreateGreigeFabricPayload,
  type GreigeFabric,
  type GreigeStatusValue,
  type UpdateGreigeFabricPayload,
} from '@/api/greige-fabric';
import { getSupplierById, getSupplierList } from '@/api/supplier';
import { getWarehouseById, getWarehouseList } from '@/api/warehouse';
import { getProductById, getProductList } from '@/api/product';
import { getPurchaseOrderList } from '@/api/purchase';
import { getPurchaseReceipt, getPurchaseReceiptList } from '@/api/purchase-receipt';
import { logger } from '@/utils/logger';
import { decimalWireToNumber } from '@/utils/money';
import { ElMessage } from 'element-plus';

import { useGreigePicker, PICKER_PAGE_SIZE, type PickerOption } from '../useGreigePicker';
import { optionalNonNegativeDecimal, toPayloadNumber } from '../greige-rules';

/**
 * 坯布建档与编辑对话框。
 * 功能：采集后端 CreateGreigeFabricRequest（30 键）与 UpdateGreigeFabricRequest（19 键）
 * 的全部入参；编辑态把只在建单 DTO 出现的列锁成只读，并让可空列的三态送达后端
 * （清空输入框=显式 null=清空为 NULL，不改=保持原值）。
 * 调用方：views/greige-fabrics/index.vue 的新建与编辑入口。
 * 入参：visible 控制显隐，mode 区分建单与编辑，row 是待编辑的坯布行（建单态为 null）。
 * 传给谁：createGreigeFabric / updateGreigeFabric，即 POST 与 PUT /production/greige-fabrics。
 * 存什么：只存本对话框的表单值与各选择器的候选，成功提交后 emit saved 由父级刷新列表。
 * 存哪里：全部组件内响应式状态，不落 store 与浏览器存储。
 */
const props = defineProps<{
  visible: boolean;
  mode: 'create' | 'edit';
  row: GreigeFabric | null;
}>();

const emit = defineEmits<{
  (e: 'update:visible', value: boolean): void;
  (e: 'saved'): void;
}>();

const { t } = useI18n({ useScope: 'global' });

/** 质量等级候选：库里该列无 CHECK，取值域只在建表列注释里声明（A/B/C） */
const QUALITY_GRADES = ['A', 'B', 'C'] as const;

/**
 * 采购单据候选与回查的单页条数。后端列表对 page_size 做 clamp(1,100)
 * （见 handlers/purchase_receipt_handler.rs 的分页钳制），取上限值本身而不是超过它的数，
 * 避免拿到被静默削过的结果还以为取全了。
 */
const REFERENCE_PAGE_SIZE = 100;

const formRef = ref<FormInstance>();
const submitting = ref(false);

const isEdit = computed(() => props.mode === 'edit');
const dialogTitle = computed(() =>
  isEdit.value ? t('greigeFabrics.index.titleEdit') : t('greigeFabrics.index.titleCreate')
);
const unitKg = computed(() => t('greigeFabrics.form.placeholderUnit', { unit: 'kg' }));
const unitM = computed(() => t('greigeFabrics.form.placeholderUnit', { unit: 'm' }));
const productPlaceholder = computed(() =>
  isEdit.value
    ? t('greigeFabrics.form.lockedValue')
    : t('greigeFabrics.form.placeholderRemoteSearch')
);
const receiptPlaceholder = computed(() =>
  formData.supplier_id === null
    ? t('greigeFabrics.form.placeholderPickSupplierFirst')
    : t('greigeFabrics.form.placeholderSelectReceipt')
);

/** 表单值：数值列一律用字符串承载（录入面是 el-input，禁步进控件），提交前按载荷归一 */
interface GreigeFormState {
  fabric_no: string;
  fabric_name: string;
  fabric_type: string;
  color_code: string;
  status: GreigeStatusValue;
  supplier_id: number | null;
  product_id: number | null;
  warehouse_id: number | null;
  location: string;
  composition: string;
  yarn_count: string;
  density: string;
  structure: string;
  width: string;
  width_cm: string;
  gram_weight: string;
  production_date: string;
  purchase_date: string;
  weight_kg: string;
  length_m: string;
  quantity_kg: string;
  quantity_meters: string;
  quality_grade: string;
  batch_no: string;
  safety_stock: string;
  reorder_point: string;
  max_stock_point: string;
  reorder_quantity: string;
  dye_lot_no: string;
  color_no: string;
  remarks: string;
  purchase_order_id: number | null;
  purchase_receipt_id: number | null;
}

const emptyForm = (): GreigeFormState => ({
  fabric_no: '',
  fabric_name: '',
  fabric_type: '',
  color_code: '',
  status: GREIGE_STATUS.IN_STOCK,
  supplier_id: null,
  product_id: null,
  warehouse_id: null,
  location: '',
  composition: '',
  yarn_count: '',
  density: '',
  structure: '',
  width: '',
  width_cm: '',
  gram_weight: '',
  production_date: '',
  purchase_date: '',
  weight_kg: '',
  length_m: '',
  quantity_kg: '',
  quantity_meters: '',
  quality_grade: '',
  batch_no: '',
  safety_stock: '',
  reorder_point: '',
  max_stock_point: '',
  reorder_quantity: '',
  dye_lot_no: '',
  color_no: '',
  remarks: '',
  purchase_order_id: null,
  purchase_receipt_id: null,
});

const formData = reactive<GreigeFormState>(emptyForm());

/** 数值录入校验：可空列允许留空（留空即清空声明），负数与超两位小数一律拦下 */
const decimalRule = (fieldKey: string) => ({
  validator: optionalNonNegativeDecimal(fieldKey),
  trigger: 'blur',
});

const formRules = computed<FormRules>(() => ({
  fabric_name: [
    {
      required: true,
      message: t('greigeFabrics.rules.required', {
        field: t('greigeFabrics.index.colFabricName'),
      }),
      trigger: 'blur',
    },
  ],
  fabric_type: [
    {
      required: true,
      message: t('greigeFabrics.index.ruleFabricTypeRequired'),
      trigger: 'blur',
    },
  ],
  status: [
    {
      required: true,
      message: t('greigeFabrics.index.ruleStatusRequired'),
      trigger: 'change',
    },
  ],
  // 纺织四维口径（与 dye-batch 页同规则）：色号非空即染色布，缸号必填；色号留空表示白坯
  dye_lot_no: [
    {
      validator: (_rule: unknown, value: unknown, callback: (err?: Error) => void) => {
        if (formData.color_no.trim() !== '' && String(value ?? '').trim() === '') {
          callback(new Error(t('greigeFabrics.rules.dyeLotNoRequiredWhenColored')));
          return;
        }
        callback();
      },
      trigger: 'blur',
    },
  ],
  width: decimalRule('greigeFabrics.form.labelWidthM'),
  width_cm: decimalRule('greigeFabrics.form.labelWidthCm'),
  gram_weight: decimalRule('greigeFabrics.form.labelGramWeight'),
  weight_kg: decimalRule('greigeFabrics.index.colWeightKg'),
  length_m: decimalRule('greigeFabrics.index.colLengthM'),
  quantity_kg: decimalRule('greigeFabrics.form.labelQuantityKg'),
  quantity_meters: decimalRule('greigeFabrics.form.labelQuantityMeters'),
  safety_stock: decimalRule('greigeFabrics.form.labelSafetyStock'),
  reorder_point: decimalRule('greigeFabrics.form.labelReorderPoint'),
  max_stock_point: decimalRule('greigeFabrics.form.labelMaxStockPoint'),
  reorder_quantity: decimalRule('greigeFabrics.form.labelReorderQuantity'),
}));

// ---- 实体选择器（按名称远程搜索，取代手填数字主键）----
const supplierPicker = useGreigePicker(
  async (keyword): Promise<PickerOption[]> => {
    const res = await getSupplierList({ keyword, page: 1, page_size: PICKER_PAGE_SIZE });
    return res.data.items.map(item => ({
      id: item.id,
      label: `${item.supplier_name}（${item.supplier_code}）`,
    }));
  },
  async (id): Promise<PickerOption | null> => {
    const res = await getSupplierById(id);
    const row = res.data;
    if (!row) return null;
    return { id, label: `${row.supplier_name ?? ''}（${row.supplier_code ?? ''}）` };
  }
);
const {
  options: supplierOptions,
  loading: supplierLoading,
  search: searchSuppliers,
  syncPinned: syncSupplierPinned,
  restore: restoreSupplier,
} = supplierPicker;

const warehousePicker = useGreigePicker(
  async (keyword): Promise<PickerOption[]> => {
    const res = await getWarehouseList({
      search: keyword,
      page: 1,
      page_size: PICKER_PAGE_SIZE,
    });
    return res.data.items.map(item => ({
      id: item.id,
      label: `${item.warehouse_name}（${item.warehouse_code}）`,
    }));
  },
  async (id): Promise<PickerOption | null> => {
    const res = await getWarehouseById(id);
    const row = res.data;
    if (!row) return null;
    return { id, label: `${row.warehouse_name}（${row.warehouse_code}）` };
  }
);
const {
  options: warehouseOptions,
  loading: warehouseLoading,
  search: searchWarehouses,
  syncPinned: syncWarehousePinned,
  restore: restoreWarehouse,
} = warehousePicker;

const productPicker = useGreigePicker(
  async (keyword): Promise<PickerOption[]> => {
    const res = await getProductList({
      search: keyword,
      page: 1,
      page_size: PICKER_PAGE_SIZE,
    });
    return res.data.items.map(item => ({
      id: item.id,
      label: `${item.product_name}（${item.product_code}）`,
    }));
  },
  async (id): Promise<PickerOption | null> => {
    const res = await getProductById(id);
    const row = res.data;
    if (!row) return null;
    return { id, label: `${row.product_name}（${row.product_code}）` };
  }
);
const {
  options: productOptions,
  loading: productLoading,
  search: searchProducts,
  restore: restoreProduct,
} = productPicker;

const purchaseOrderPicker = useGreigePicker(
  async (keyword): Promise<PickerOption[]> => {
    const res = await getPurchaseOrderList({
      keyword,
      page: 1,
      page_size: PICKER_PAGE_SIZE,
    });
    return res.data.items.map(item => ({ id: item.id, label: item.order_no }));
  },
  async (id): Promise<PickerOption | null> => {
    // 单据号是唯一可展示标识，按列表回查取号（单页取后端 clamp 上限以内，避免被静默削页）
    const res = await getPurchaseOrderList({ page: 1, page_size: REFERENCE_PAGE_SIZE });
    const found = res.data.items.find(item => item.id === id);
    return found ? { id, label: found.order_no } : null;
  }
);
const {
  options: purchaseOrderOptions,
  loading: purchaseOrderLoading,
  search: searchPurchaseOrders,
  restore: restorePurchaseOrder,
} = purchaseOrderPicker;

/**
 * 采购入库单候选：后端 PurchaseReceiptQueryParams 只支持 page/page_size/status/supplier_id/
 * order_id，没有关键字键，因此按已选供应商做服务端收窄（单页取 100，后端 clamp 上限），
 * 控件的 filterable 只在这一批已按供应商过滤的结果上做本地缩小。
 */
const receiptPicker = useGreigePicker(
  async (): Promise<PickerOption[]> => {
    if (formData.supplier_id === null) return [];
    const res = await getPurchaseReceiptList({
      supplier_id: formData.supplier_id,
      page: 1,
      page_size: REFERENCE_PAGE_SIZE,
    });
    return (res.data?.items ?? [])
      .filter(item => typeof item.id === 'number')
      .map(item => ({ id: item.id as number, label: item.receipt_no }));
  },
  async (id): Promise<PickerOption | null> => {
    const res = await getPurchaseReceipt(id);
    return res.data?.receipt_no ? { id, label: res.data.receipt_no } : null;
  }
);
const {
  options: receiptOptions,
  loading: receiptLoading,
  search: searchReceipts,
  restore: restoreReceipt,
  reset: resetReceipts,
} = receiptPicker;

/** 仓库选择变更：值随控件回传，钉住项同步，清空即把 warehouse_id 交回 null（编辑态提交即清空） */
const onWarehouseChange = (value: unknown): void => {
  formData.warehouse_id = typeof value === 'number' ? value : null;
  syncWarehousePinned(formData.warehouse_id);
};

/** 换供应商即入库单候选失效：清空已选入库单并重新取候选，避免留下与他厂不符的引用 */
const onSupplierChange = async (value: unknown): Promise<void> => {
  formData.supplier_id = typeof value === 'number' ? value : null;
  syncSupplierPinned(formData.supplier_id);
  if (formData.purchase_receipt_id !== null) {
    formData.purchase_receipt_id = null;
    resetReceipts();
  }
  await loadReceiptCandidates();
};

const loadReceiptCandidates = async (): Promise<void> => {
  if (isEdit.value) return;
  await searchReceipts('');
};

/** 行数据回填表单：Decimal 出参是字符串，Number 归一后回到带两位小数的字符串录入态 */
const fillFromRow = (row: GreigeFabric): void => {
  const numText = (value: string | null | undefined): string =>
    value === null || value === undefined ? '' : String(decimalWireToNumber(value));
  Object.assign(formData, emptyForm(), {
    fabric_no: row.fabric_no ?? '',
    fabric_name: row.fabric_name ?? '',
    fabric_type: row.fabric_type,
    color_code: row.color_code ?? '',
    status: (row.status as GreigeStatusValue) ?? GREIGE_STATUS.IN_STOCK,
    supplier_id: row.supplier_id ?? null,
    product_id: row.product_id ?? null,
    warehouse_id: row.warehouse_id ?? null,
    location: row.location ?? '',
    composition: row.composition ?? '',
    yarn_count: row.yarn_count ?? '',
    density: row.density ?? '',
    structure: row.structure ?? '',
    width: numText(row.width),
    width_cm: numText(row.width_cm),
    gram_weight: numText(row.gram_weight),
    production_date: row.production_date ?? '',
    purchase_date: row.purchase_date ?? '',
    weight_kg: numText(row.weight_kg),
    length_m: numText(row.length_m),
    quantity_kg: numText(row.quantity_kg),
    quantity_meters: numText(row.quantity_meters),
    quality_grade: row.quality_grade ?? '',
    batch_no: row.batch_no ?? '',
    safety_stock: numText(row.safety_stock),
    reorder_point: numText(row.reorder_point),
    max_stock_point: numText(row.max_stock_point),
    reorder_quantity: numText(row.reorder_quantity),
    dye_lot_no: row.dye_lot_no ?? '',
    color_no: row.color_no ?? '',
    remarks: row.remarks ?? '',
    purchase_order_id: row.purchase_order_id ?? null,
    purchase_receipt_id: row.purchase_receipt_id ?? null,
  });
};

/** 打开时回填并按主键取回各选择器的名称（行里只有主键，名称必须回源，不猜不拼） */
watch(
  () => props.visible,
  async visible => {
    if (!visible) return;
    if (props.row) fillFromRow(props.row);
    else Object.assign(formData, emptyForm());
    formRef.value?.clearValidate();
    await Promise.all([
      restoreSupplier(formData.supplier_id),
      restoreWarehouse(formData.warehouse_id),
      restoreProduct(formData.product_id),
      restorePurchaseOrder(formData.purchase_order_id),
      restoreReceipt(formData.purchase_receipt_id),
      loadReceiptCandidates(),
    ]);
  }
);

const handleClosed = (): void => {
  formRef.value?.clearValidate();
};

/** 建单载荷：可空列留空即整键省略（后端 Option 缺省为 NULL），空串不提交 */
const buildCreatePayload = (): CreateGreigeFabricPayload => {
  const text = (value: string): string | undefined =>
    value.trim() === '' ? undefined : value.trim();
  const num = (value: string): number | undefined => {
    const parsed = toPayloadNumber(value);
    return typeof parsed === 'number' ? parsed : undefined;
  };
  return {
    fabric_no: text(formData.fabric_no),
    fabric_name: text(formData.fabric_name),
    fabric_type: formData.fabric_type.trim(),
    color_code: text(formData.color_code),
    width_cm: num(formData.width_cm),
    weight_kg: num(formData.weight_kg),
    length_m: num(formData.length_m),
    supplier_id: formData.supplier_id ?? undefined,
    batch_no: text(formData.batch_no),
    warehouse_id: formData.warehouse_id ?? undefined,
    location: text(formData.location),
    status: formData.status,
    quality_grade: text(formData.quality_grade),
    purchase_date: formData.purchase_date || undefined,
    remarks: text(formData.remarks),
    product_id: formData.product_id ?? undefined,
    composition: text(formData.composition),
    yarn_count: text(formData.yarn_count),
    density: text(formData.density),
    width: num(formData.width),
    gram_weight: num(formData.gram_weight),
    structure: text(formData.structure),
    production_date: formData.production_date || undefined,
    quantity_meters: num(formData.quantity_meters),
    quantity_kg: num(formData.quantity_kg),
    purchase_order_id: formData.purchase_order_id ?? undefined,
    purchase_receipt_id: formData.purchase_receipt_id ?? undefined,
    safety_stock: num(formData.safety_stock),
    reorder_point: num(formData.reorder_point),
    max_stock_point: num(formData.max_stock_point),
    reorder_quantity: num(formData.reorder_quantity),
    dye_lot_no: text(formData.dye_lot_no),
    color_no: text(formData.color_no),
  };
};

/**
 * 编辑载荷：只发 UpdateGreigeFabricRequest 声明的 19 个键。
 * 可空列一律显式给值（有值覆盖、留空提交 null 清空），键缺席只用于「编辑态不可改」的列，
 * 这样用户在表单里清空一个框就是真要清空，而不是被后端当成没提交。
 */
const buildUpdatePayload = (rowId: number): { id: number; payload: UpdateGreigeFabricPayload } => {
  const textOrNull = (value: string): string | null => {
    const trimmed = value.trim();
    return trimmed === '' ? null : trimmed;
  };
  const numOrNull = (value: string): number | null => {
    const parsed = toPayloadNumber(value);
    return typeof parsed === 'number' ? parsed : null;
  };
  return {
    id: rowId,
    payload: {
      fabric_name: formData.fabric_name.trim(),
      fabric_type: formData.fabric_type.trim(),
      color_code: textOrNull(formData.color_code),
      width_cm: numOrNull(formData.width_cm),
      weight_kg: numOrNull(formData.weight_kg),
      length_m: numOrNull(formData.length_m),
      supplier_id: formData.supplier_id,
      batch_no: textOrNull(formData.batch_no),
      warehouse_id: formData.warehouse_id,
      location: textOrNull(formData.location),
      status: formData.status,
      quality_grade: textOrNull(formData.quality_grade),
      remarks: textOrNull(formData.remarks),
      safety_stock: numOrNull(formData.safety_stock),
      reorder_point: numOrNull(formData.reorder_point),
      max_stock_point: numOrNull(formData.max_stock_point),
      reorder_quantity: numOrNull(formData.reorder_quantity),
      dye_lot_no: textOrNull(formData.dye_lot_no),
      color_no: textOrNull(formData.color_no),
    },
  };
};

const handleSubmit = async (): Promise<void> => {
  if (!formRef.value) return;
  const valid = await formRef.value.validate().catch(() => false);
  if (!valid) return;
  if (isEdit.value && !props.row?.id) {
    // 编辑态没有主键即上游传参错误，显式报出来，不静默退回建单路径
    logger.error('[greige] 编辑提交缺少坯布主键', { mode: props.mode });
    ElMessage.error(t('greigeFabrics.messages.messageEditTargetMissing'));
    return;
  }

  submitting.value = true;
  try {
    if (isEdit.value) {
      const { id, payload } = buildUpdatePayload(props.row?.id as number);
      await updateGreigeFabric(id, payload);
      ElMessage.success(t('greigeFabrics.index.messageUpdateSuccess'));
    } else {
      await createGreigeFabric(buildCreatePayload());
      ElMessage.success(t('greigeFabrics.index.messageCreateSuccess'));
    }
    emit('update:visible', false);
    emit('saved');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(
      err.message ||
        (isEdit.value
          ? t('greigeFabrics.index.messageUpdateFailed')
          : t('greigeFabrics.index.messageCreateFailed'))
    );
  } finally {
    submitting.value = false;
  }
};
</script>

<style scoped>
.section-hint {
  margin-left: 12px;
  font-size: 12px;
  color: var(--el-text-color-secondary);
}
</style>
