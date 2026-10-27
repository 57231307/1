<!--
  ProductFormDialogTab.vue - 产品新建/编辑对话框
  来源：原 product/index.vue 中 新建/编辑对话框
  拆分日期：2026-06-15 B3-4
  D05 Batch 8 Group B：接入 useI18n
-->
<template>
  <el-dialog
    :model-value="visible"
    :title="title"
    width="700px"
    :close-on-click-modal="false"
    :aria-label="title"
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="formRules"
      label-width="100px"
      :disabled="mode === 'view'"
      :aria-label="t('product.productFormDialogTab.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelProductCode')"
            prop="product_code"
          >
            <el-input
              v-model="formData.product_code"
              :placeholder="t('product.productFormDialogTab.placeholderProductCode')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelProductName')"
            prop="product_name"
          >
            <el-input
              v-model="formData.product_name"
              :placeholder="t('product.productFormDialogTab.placeholderProductName')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelCategory')" prop="category_id">
            <el-select
              v-model="formData.category_id"
              :placeholder="t('product.productFormDialogTab.placeholderCategory')"
              style="width: 100%"
            >
              <el-option
                v-for="item in categories"
                :key="item.id"
                :label="item.name"
                :value="item.id"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item prop="specification">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelSpecification') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipSpecificationAuto')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 规格提交串按「幅宽 星号 克重」字面拼接（例 160cm*180g），星号是连接符不是乘法；
                 两项齐备时本框只读显示拼接值，任一缺失时回落手工录入，提交端与显示端同一形态 -->
            <el-input
              :model-value="specificationDisplay"
              :readonly="specificationAuto !== null"
              :placeholder="t('product.productFormDialogTab.placeholderSpecification')"
              @update:model-value="(val: string) => (formData.specification = val)"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelUnit')" prop="unit">
            <!-- 交易单位候选词表（中文稳定 token，禁英文化）；报价行单位即随此值带出 -->
            <el-select
              v-model="formData.unit"
              :placeholder="t('product.productFormDialogTab.placeholderUnit')"
              style="width: 100%"
            >
              <el-option
                v-for="item in unitOptions"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelBarcode')" prop="barcode">
            <el-input
              v-model="formData.barcode"
              :placeholder="t('product.productFormDialogTab.placeholderBarcode')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item prop="width">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelWidth') }}
                <el-tooltip :content="t('product.productFormDialogTab.tipWidth')" placement="top">
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 数字列统一 el-input 加校验规则，禁用带加减按钮的数字输入框 -->
            <el-input
              v-model="formData.width"
              :placeholder="t('product.productFormDialogTab.placeholderWidth')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item prop="gram_weight">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelGramWeight') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipGramWeight')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <el-input
              v-model="formData.gram_weight"
              :placeholder="t('product.productFormDialogTab.placeholderGramWeight')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item prop="meters_per_kg_display">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelMetersPerKg') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipMetersPerKg')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 每公斤出米数是只读派生值：由克重与幅宽即时算出（向下取整一位小数），不入库、不随提交回传 -->
            <el-input
              :model-value="metersPerKgDisplay"
              readonly
              :placeholder="t('product.productFormDialogTab.placeholderMetersPerKgNa')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item prop="product_type">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelProductType') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipProductType')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 产品类型词表与后端 product_type 唯一词表同源（坯布/成品布/辅料），value 为中文落库真相，label 走 i18n -->
            <el-select
              v-model="formData.product_type"
              :placeholder="t('product.productFormDialogTab.placeholderProductType')"
              style="width: 100%"
            >
              <el-option
                v-for="item in productTypeOptions"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item prop="fabric_composition">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelFabricComposition') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipFabricComposition')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <el-input
              v-model="formData.fabric_composition"
              :placeholder="t('product.productFormDialogTab.placeholderFabricComposition')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelYarnCount')" prop="yarn_count">
            <el-input
              v-model="formData.yarn_count"
              :placeholder="t('product.productFormDialogTab.placeholderYarnCount')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelDensity')" prop="density">
            <el-input
              v-model="formData.density"
              :placeholder="t('product.productFormDialogTab.placeholderDensity')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item prop="structure">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelStructure') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipStructure')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 组织词表分梭织与针织两系，针织分纬编与经编（词表源 constants/product-spec-vocab.ts）；
                 落库仍是自由文本列，词表外组织自填，50 字符上限由后端长度校验拦 -->
            <el-select
              v-model="formData.structure"
              filterable
              allow-create
              default-first-option
              clearable
              :placeholder="t('product.productFormDialogTab.placeholderStructure')"
              style="width: 100%"
            >
              <el-option-group
                v-for="group in PRODUCT_STRUCTURE_GROUPS"
                :key="group.label"
                :label="t(group.label)"
              >
                <el-option v-for="term in group.options" :key="term" :label="term" :value="term" />
              </el-option-group>
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelFinish')" prop="finish">
            <el-input
              v-model="formData.finish"
              :placeholder="t('product.productFormDialogTab.placeholderFinish')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelMinOrderQuantity')"
            prop="min_order_quantity"
          >
            <el-input
              v-model="formData.min_order_quantity"
              :placeholder="t('product.productFormDialogTab.placeholderMinOrderQuantity')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelLeadTime')" prop="lead_time">
            <el-input
              v-model="formData.lead_time"
              :placeholder="t('product.productFormDialogTab.placeholderLeadTime')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelExecutionStandard')"
            prop="execution_standard"
          >
            <el-input
              v-model="formData.execution_standard"
              :placeholder="t('product.productFormDialogTab.placeholderExecutionStandard')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelFactoryName')"
            prop="factory_name"
          >
            <el-input
              v-model="formData.factory_name"
              :placeholder="t('product.productFormDialogTab.placeholderFactoryName')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelFactoryAddress')"
            prop="factory_address"
          >
            <el-input
              v-model="formData.factory_address"
              :placeholder="t('product.productFormDialogTab.placeholderFactoryAddress')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item
            :label="t('product.productFormDialogTab.labelProductGrade')"
            prop="product_grade"
          >
            <el-input
              v-model="formData.product_grade"
              :placeholder="t('product.productFormDialogTab.placeholderProductGrade')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item prop="safety_category">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelSafetyCategory') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipSafetyCategory')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 安全类别词表与后端 product_safety_category 同源（GB 18401 A/B/C），value 为字母落库真相，label 走 i18n -->
            <el-select
              v-model="formData.safety_category"
              clearable
              :placeholder="t('product.productFormDialogTab.placeholderSafetyCategory')"
              style="width: 100%"
            >
              <el-option
                v-for="item in safetyCategoryOptions"
                :key="item.value"
                :label="item.label"
                :value="item.value"
              />
            </el-select>
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item prop="width_needle_included_cm">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelWidthNeedleIncluded') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipWidthNeedleIncluded')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 数字列一律 el-input 加校验，禁带加减按钮的数字输入框；空文本提交即 null 走三态清空 -->
            <el-input
              v-model="formData.width_needle_included_cm"
              :placeholder="t('product.productFormDialogTab.placeholderWidthNeedleIncluded')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item prop="weight_tolerance">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelWeightTolerance') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipWeightTolerance')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <el-input
              v-model="formData.weight_tolerance"
              :placeholder="t('product.productFormDialogTab.placeholderWeightTolerance')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item prop="spandex_percent">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelSpandexPercent') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipSpandexPercent')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <el-input
              v-model="formData.spandex_percent"
              :placeholder="t('product.productFormDialogTab.placeholderSpandexPercent')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelUsage')" prop="usage">
            <el-input
              v-model="formData.usage"
              :placeholder="t('product.productFormDialogTab.placeholderUsage')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelPrice')" prop="price">
            <el-input
              v-model="formData.price"
              :placeholder="t('product.productFormDialogTab.placeholderPrice')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelCostPrice')" prop="cost_price">
            <el-input
              v-model="formData.cost_price"
              :placeholder="t('product.productFormDialogTab.placeholderCostPrice')"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('product.productFormDialogTab.labelDescription')" prop="description">
        <el-input
          v-model="formData.description"
          type="textarea"
          :rows="3"
          :placeholder="t('product.productFormDialogTab.placeholderDescription')"
        />
      </el-form-item>
      <el-form-item :label="t('product.productFormDialogTab.labelStatus')" prop="is_active">
        <el-switch
          v-model="formData.is_active"
          :active-text="t('product.productFormDialogTab.statusActive')"
          :inactive-text="t('product.productFormDialogTab.statusInactive')"
        />
      </el-form-item>
    </el-form>
    <template v-if="mode !== 'view'" #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('product.productFormDialogTab.buttonCancel')
      }}</el-button>
      <el-button
        type="primary"
        :loading="submitLoading"
        :disabled="submitLoading"
        @click="handleSubmit"
        >{{ t('product.productFormDialogTab.buttonSave') }}</el-button
      >
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import { QuestionFilled } from '@element-plus/icons-vue';
import type { FormInstance, FormItemRule, FormRules } from 'element-plus';
import type { Product, ProductCategory } from '@/api/product';
import { createProduct, updateProduct } from '@/api/product';
import { QUOTATION_UNIT_LABEL_KEY, QUOTATION_UNIT_VALUES } from '@/constants/quotation-unit';
import {
  PRODUCT_TYPE_VALUES,
  PRODUCT_TYPE_LABEL_KEY,
  isFabricProductType,
} from '@/constants/product-type';
import {
  PRODUCT_SAFETY_CATEGORY_VALUES,
  PRODUCT_SAFETY_CATEGORY_LABEL_KEY,
  PRODUCT_SAFETY_CATEGORY,
  PRODUCT_STRUCTURE_GROUPS,
} from '@/constants/product-spec-vocab';
import { computeMetersPerKg, buildFabricSpecification } from '@/utils/fabric-yardage';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

// 安全类别下拉：value 用字母落库真相（A/B/C），label 走 i18n；词表与后端 product_safety_category 同源
const safetyCategoryOptions = computed(() =>
  PRODUCT_SAFETY_CATEGORY_VALUES.map(value => ({
    value,
    label: t(PRODUCT_SAFETY_CATEGORY_LABEL_KEY[value]),
  }))
);

// 交易单位下拉：value 用库里稳定的中文单位 token，label 走 i18n；报价行单位随此值带出
const unitOptions = computed(() =>
  QUOTATION_UNIT_VALUES.map(value => ({
    value,
    label: t(QUOTATION_UNIT_LABEL_KEY[value]),
  }))
);

// 产品类型下拉：value 用库里稳定的中文类型 token（坯布/成品布/辅料），label 走 i18n；
// 词表与后端 product_type::ALL_TYPES 同源，面料类判定（必填分级）与后端 FABRIC_TYPES 同源
const productTypeOptions = computed(() =>
  PRODUCT_TYPE_VALUES.map(value => ({
    value,
    label: t(PRODUCT_TYPE_LABEL_KEY[value]),
  }))
);

interface Props {
  modelValue: boolean;
  title: string;
  rowData: Product | null;
  categories: ProductCategory[];
  mode: 'create' | 'edit' | 'view';
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const visible = ref(props.modelValue);
const submitLoading = ref(false);
const formRef = ref<FormInstance>();

const formData = reactive({
  id: undefined as number | undefined,
  product_code: '',
  product_name: '',
  category_id: undefined as number | undefined,
  specification: '',
  unit: '',
  barcode: '',
  // 数字列一律以输入框文本形态存（el-input，禁带加减按钮的数字输入框），
  // 提交前经 toSubmitNumber 归一：空文本 → null（三态清空通道），有效数字文本 → Number
  price: '0',
  cost_price: '0',
  width: '',
  gram_weight: '',
  // 主档文本列：产品类型存中文词表值，其余按后端键名直存
  product_type: '',
  fabric_composition: '',
  yarn_count: '',
  density: '',
  structure: '',
  finish: '',
  // m0106 扩的规格列，数字列以输入框文本形态存（el-input，禁带加减按钮），提交前经 toSubmitNumber 归一
  width_needle_included_cm: '',
  weight_tolerance: '',
  spandex_percent: '',
  usage: '',
  safety_category: '',
  min_order_quantity: '',
  lead_time: '',
  execution_standard: '',
  factory_name: '',
  factory_address: '',
  product_grade: '',
  description: '',
  is_active: true,
});

// 数字列提交值归一：null/undefined/空文本 → null（后端三态按显式空清空该列），
// 有效数字文本 → Number（后端 DECIMAL 入参收 JSON 数字与十进制字符串两形态）。
// 调用方：metersPerKgDisplay 即时换算与本组件 buildSubmitPayload 提交载荷。
const toSubmitNumber = (raw: number | string | null | undefined): number | null => {
  const text = String(raw ?? '').trim();
  return text === '' ? null : Number(text);
};

// 数字列出参回显归一：后端 Decimal 序列化为字符串、NULL 为 null，统一转成输入框文本；
// 空值归一为 ''（空），不归 0，避免把未录入显示成 0。
const toInputText = (value: number | string | null | undefined): string =>
  value == null ? '' : String(value);

// 是否面料类（坯布/成品布），判定源与后端 FABRIC_TYPES 同源；新建时据此动态加必填
const isFabricType = computed(() => isFabricProductType(formData.product_type));

// 每公斤出米数即时只读值：读表单当前克重与幅宽，算式与后端 DualUnitConverter::meters_per_kg
// 同源（向下取整一位小数）；缺值或非正返回 null，界面显示占位文案，不显示 0 冒充可算。
const metersPerKgDisplay = computed(() => {
  const value = computeMetersPerKg(
    toSubmitNumber(formData.gram_weight),
    toSubmitNumber(formData.width)
  );
  return value == null ? '' : String(value);
});

// 规格提交串的派生源：按「幅宽 星号 克重」字面拼接（例 160cm*180g），算式单点在 utils/fabric-yardage.ts。
// 读：表单当前幅宽与克重文本（经 toSubmitNumber 归一，空值传 null）。写给：specificationDisplay 展示值
// 与 buildSubmitPayload 的 specification 键。存：拼接结果落 products.specification 文本列，两列数值本身各存各列。
// 两项任一缺失或非法时为 null，规格框回落手工录入形态；禁止把两段值当数字相乘，也禁止只拼显示端不拼提交端。
// 调用方：本组件规格框与提交载荷；无副作用，故用 computed 而不是 watch，回显赋值天然不触发额外同步。
const specificationAuto = computed(() =>
  buildFabricSpecification(toSubmitNumber(formData.width), toSubmitNumber(formData.gram_weight))
);

// 规格框显示值：派生串可用时显示派生串，否则显示手工录入文本
const specificationDisplay = computed(() => specificationAuto.value ?? formData.specification);

// 数字文本校验规则工厂：空值放行（必填性由 required 规则分权判定），非空必须是不带符号的
// 十进制文本且小数位不超上限（模式不含负号，负数直接判不合格）；exclusiveMin 为真时再要求
// 严格大于 0（面料类幅宽、克重口径）。messageKey 指向本组件语言包文案键。
const numericFormatRule = (
  messageKey: string,
  decimals: number,
  exclusiveMin = false
): FormItemRule => ({
  validator: ((_rule: unknown, value: unknown, callback: (error?: Error) => void) => {
    const raw = String(value ?? '').trim();
    if (raw === '') {
      callback();
      return;
    }
    const pattern = decimals === 0 ? /^\d+$/ : new RegExp(`^\\d+(\\.\\d{1,${decimals}})?$`);
    const n = Number(raw);
    if (!pattern.test(raw) || !Number.isFinite(n) || (exclusiveMin && n <= 0)) {
      callback(new Error(t(messageKey)));
      return;
    }
    callback();
  }) as FormItemRule['validator'],
  trigger: 'blur',
});

// 校验规则按模式与产品类型动态生成：新建时产品类型必选；类型落在面料集合时
// 幅宽、克重、面料成分必填且须大于 0（判据与后端 validate_product_create_fields 同源，前端拦录入、后端拒缺键）。
// 数字列格式规则两态恒挂：提交前统一拦掉非数字与超位小数，后端 Decimal 收到必是可解析形态。
// 编辑维持原语义（后端更新入口不做分级必填），不把历史缺档产品改成「不补全就不能保存」。
const formRules = computed<FormRules>(() => {
  const rules: FormRules = {
    product_code: [
      {
        required: true,
        message: t('product.productFormDialogTab.validateProductCodeRequired'),
        trigger: 'blur',
      },
      {
        max: 50,
        message: t('product.productFormDialogTab.validateProductCodeMax'),
        trigger: 'blur',
      },
    ],
    product_name: [
      {
        required: true,
        message: t('product.productFormDialogTab.validateProductNameRequired'),
        trigger: 'blur',
      },
      {
        max: 200,
        message: t('product.productFormDialogTab.validateProductNameMax'),
        trigger: 'blur',
      },
    ],
    category_id: [
      {
        required: true,
        message: t('product.productFormDialogTab.validateCategoryRequired'),
        trigger: 'change',
      },
    ],
    unit: [
      {
        required: true,
        message: t('product.productFormDialogTab.validateUnitRequired'),
        trigger: 'change',
      },
    ],
    width: [numericFormatRule('product.productFormDialogTab.validateWidthFormat', 2)],
    gram_weight: [numericFormatRule('product.productFormDialogTab.validateGramWeightFormat', 2)],
    // 带针幅宽严格为正（口径同后端 CHECK），克重公差与氨纶含量非负（0 到 100 上限由后端 CHECK 拒）
    width_needle_included_cm: [
      numericFormatRule('product.productFormDialogTab.validateWidthNeedleFormat', 2, true),
    ],
    weight_tolerance: [
      numericFormatRule('product.productFormDialogTab.validateWeightToleranceFormat', 2),
    ],
    spandex_percent: [
      numericFormatRule('product.productFormDialogTab.validateSpandexPercentFormat', 2),
    ],
    min_order_quantity: [
      numericFormatRule('product.productFormDialogTab.validateMinOrderQuantityFormat', 2),
    ],
    lead_time: [numericFormatRule('product.productFormDialogTab.validateLeadTimeFormat', 0)],
    price: [numericFormatRule('product.productFormDialogTab.validatePriceFormat', 2)],
    cost_price: [numericFormatRule('product.productFormDialogTab.validatePriceFormat', 2)],
    // GB 18401-2010 的 A 类为婴幼儿纺织产品，GB 31701-2015 是其强制并标考核标准；
    // 录入面拦「选了 A 类但执行标准没列婴幼儿标准」这一跨字段缺口，判据只查用户自己提交的两个字段。
    // 标准号形态允许空格与年份后缀差异（GB 31701-2015、GB31701-2015 等），年份不参与判据。
    execution_standard: [
      {
        validator: (_rule: unknown, value: unknown, callback: (error?: Error) => void) => {
          if (formData.safety_category !== PRODUCT_SAFETY_CATEGORY.classA) {
            callback();
            return;
          }
          const standard = String(value ?? '');
          if (/GB\s*31701/.test(standard)) {
            callback();
            return;
          }
          callback(new Error(t('product.productFormDialogTab.validateInfantStandardRequired')));
        },
        trigger: 'blur',
      },
    ],
  };
  if (props.mode === 'create') {
    rules.product_type = [
      {
        required: true,
        message: t('product.productFormDialogTab.validateProductTypeRequired'),
        trigger: 'change',
      },
    ];
    if (isFabricType.value) {
      rules.width = [
        {
          required: true,
          message: t('product.productFormDialogTab.validateWidthRequiredFabric'),
          trigger: 'blur',
        },
        numericFormatRule('product.productFormDialogTab.validateWidthFormat', 2, true),
      ];
      rules.gram_weight = [
        {
          required: true,
          message: t('product.productFormDialogTab.validateGramWeightRequiredFabric'),
          trigger: 'blur',
        },
        numericFormatRule('product.productFormDialogTab.validateGramWeightFormat', 2, true),
      ];
      rules.fabric_composition = [
        {
          required: true,
          message: t('product.productFormDialogTab.validateCompositionRequiredFabric'),
          trigger: 'blur',
        },
      ];
    }
  }
  return rules;
});

const resetForm = () => {
  formData.id = undefined;
  formData.product_code = '';
  formData.product_name = '';
  formData.category_id = undefined;
  formData.specification = '';
  formData.unit = '';
  formData.barcode = '';
  formData.price = '0';
  formData.cost_price = '0';
  formData.width = '';
  formData.gram_weight = '';
  formData.product_type = '';
  formData.fabric_composition = '';
  formData.yarn_count = '';
  formData.density = '';
  formData.structure = '';
  formData.finish = '';
  formData.width_needle_included_cm = '';
  formData.weight_tolerance = '';
  formData.spandex_percent = '';
  formData.usage = '';
  formData.safety_category = '';
  formData.min_order_quantity = '';
  formData.lead_time = '';
  formData.execution_standard = '';
  formData.factory_name = '';
  formData.factory_address = '';
  formData.product_grade = '';
  formData.description = '';
  formData.is_active = true;
  formRef.value?.clearValidate();
};

watch(
  () => props.modelValue,
  val => {
    visible.value = val;
    if (val) {
      resetForm();
      if (props.rowData) {
        Object.assign(formData, props.rowData);
        // 数字列后端出参为 Decimal 字符串（NULL 为 null），统一归一成输入框文本；
        // 空值归一为 ''，不写 0，避免历史未录入在产品编辑里被显示成 0 再落回库里
        formData.price = toInputText(props.rowData.price);
        formData.cost_price = toInputText(props.rowData.cost_price);
        formData.width = toInputText(props.rowData.width);
        formData.gram_weight = toInputText(props.rowData.gram_weight);
        formData.width_needle_included_cm = toInputText(props.rowData.width_needle_included_cm);
        formData.weight_tolerance = toInputText(props.rowData.weight_tolerance);
        formData.spandex_percent = toInputText(props.rowData.spandex_percent);
        formData.usage = props.rowData.usage ?? '';
        formData.safety_category = props.rowData.safety_category ?? '';
        formData.min_order_quantity = toInputText(props.rowData.min_order_quantity);
        formData.lead_time = toInputText(props.rowData.lead_time);
      }
    }
  }
);

watch(visible, val => {
  emit('update:modelValue', val);
});

// 提交载荷只收本表单采集的字段，不把回显进来的整行原样回传（行里的 id、分类名称、
// 时间戳与后端派生的 meters_per_kg 等由服务端掌管，客户端发了也会被反序列化丢弃，
// 留着只会掩盖契约漂移）。
// 数值主档键把空值显式序列化为 null，走后端三态更新里的清空通道；键缺席的语义是
// 保持原值，不能把用户的清空做成省略键。
// 米数两键（每匹米数与每卷米数）已从本表单的采集面与载荷摘除：每匹米数是库存实收口径的
// 换算默认值，不由产品表单采集；每卷米数按裁定整列废弃，按卷换算共用每匹米数单源。
const buildSubmitPayload = (): Partial<Product> => ({
  product_code: formData.product_code,
  product_name: formData.product_name,
  category_id: formData.category_id,
  // 规格提交串：幅宽与克重齐备时提交字面拼接值（显示端与提交端同一形态），
  // 任一缺失时提交手工录入文本，禁止把两段值当数字相乘
  specification: specificationAuto.value ?? formData.specification,
  unit: formData.unit,
  barcode: formData.barcode,
  price: toSubmitNumber(formData.price),
  cost_price: toSubmitNumber(formData.cost_price),
  // 幅宽与克重是报价单换算米数与公斤数的取数源，也是每公斤出米数的算式入参，
  // 界面清空必须落得回库里，故经 toSubmitNumber 显式发 null 走三态清空通道
  width: toSubmitNumber(formData.width),
  gram_weight: toSubmitNumber(formData.gram_weight),
  // 主档文本列按采集值直发；null 即回显的历史空值，语义为不改动
  product_type: formData.product_type,
  fabric_composition: formData.fabric_composition,
  yarn_count: formData.yarn_count,
  density: formData.density,
  structure: formData.structure,
  finish: formData.finish,
  // 带针幅宽/克重公差/氨纶含量为 DECIMAL，经 toSubmitNumber 归一：空文本显式发 null 走后端三态清空通道
  width_needle_included_cm: toSubmitNumber(formData.width_needle_included_cm),
  weight_tolerance: toSubmitNumber(formData.weight_tolerance),
  spandex_percent: toSubmitNumber(formData.spandex_percent),
  // 用途为自由文本单层条件更新，直发采集值（空串即清空文本，与其余文本列同口径）
  usage: formData.usage,
  // 安全类别为受 DB CHECK 约束的封闭枚举，空值发 undefined 让该键从载荷摘除（后端单层更新键缺席即不改），
  // 不能发空串撞 CHECK；清空定级交后端词表判据，界面清空即回未定级由下次显式选择覆盖
  safety_category: formData.safety_category || undefined,
  min_order_quantity: toSubmitNumber(formData.min_order_quantity),
  lead_time: toSubmitNumber(formData.lead_time),
  execution_standard: formData.execution_standard,
  factory_name: formData.factory_name,
  factory_address: formData.factory_address,
  product_grade: formData.product_grade,
  description: formData.description,
  is_active: formData.is_active,
});

const handleSubmit = async () => {
  if (submitLoading.value) return;
  if (!formRef.value) return;
  // 锁先于异步校验落下，避免连点两次各自通过校验后重复建档
  submitLoading.value = true;
  try {
    await formRef.value.validate();
  } catch {
    submitLoading.value = false;
    return;
  }
  try {
    if (props.mode === 'create') {
      await createProduct(buildSubmitPayload());
      ElMessage.success(t('product.productFormDialogTab.messageCreateSuccess'));
    } else {
      await updateProduct(formData.id as number, buildSubmitPayload());
      ElMessage.success(t('product.productFormDialogTab.messageUpdateSuccess'));
    }
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('product.productFormDialogTab.messageOperationFailed'));
    logger.error(t('product.productFormDialogTab.messageSaveFailed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>

<style scoped>
.form-label-with-tip {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.form-label-tip {
  color: var(--el-text-color-secondary);
  cursor: help;
}
</style>
