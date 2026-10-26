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
          <el-form-item
            :label="t('product.productFormDialogTab.labelSpecification')"
            prop="specification"
          >
            <el-input
              v-model="formData.specification"
              :placeholder="t('product.productFormDialogTab.placeholderSpecification')"
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
          <el-form-item prop="meters_per_piece">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelMetersPerPiece') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipMetersPerPiece')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <el-input
              v-model="formData.meters_per_piece"
              :placeholder="t('product.productFormDialogTab.placeholderMetersPerPiece')"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item prop="meters_per_roll">
            <template #label>
              <span class="form-label-with-tip">
                {{ t('product.productFormDialogTab.labelMetersPerRoll') }}
                <el-tooltip
                  :content="t('product.productFormDialogTab.tipMetersPerRoll')"
                  placement="top"
                >
                  <el-icon class="form-label-tip"><QuestionFilled /></el-icon>
                </el-tooltip>
              </span>
            </template>
            <!-- 每卷米数是每匹米数的只读带出值，随每匹米数同值提交 -->
            <el-input v-model="formData.meters_per_roll" readonly />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
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
        <el-col :span="12">
          <el-form-item prop="meters_per_kg">
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
          <el-form-item :label="t('product.productFormDialogTab.labelStructure')" prop="structure">
            <el-input
              v-model="formData.structure"
              :placeholder="t('product.productFormDialogTab.placeholderStructure')"
            />
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
import { computeMetersPerKg } from '@/utils/fabric-yardage';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

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
  meters_per_piece: '',
  meters_per_roll: '',
  width: '',
  gram_weight: '',
  // 主档文本列：产品类型存中文词表值，其余按后端键名直存
  product_type: '',
  fabric_composition: '',
  yarn_count: '',
  density: '',
  structure: '',
  finish: '',
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

// 每卷米数是每匹米数的派生只读项：一卷即一匹，两者米数同源，界面只留每匹米数一个入口。
// 读：用户在每匹米数输入框里改动的值。写给：每卷米数的只读展示值，随提交载荷一并发给后端。
// 存：不单独落库，由后端按同值写入 products 的每匹米数与每卷米数两列。
// 回显时两列各按后端真实值展示，历史产品若两值不等如实可见；只有用户真正改动每匹米数才带出同值，
// 因此打开编辑再保存不会改写历史每卷米数。echoing 抑制回显赋值误触发带出。
// 调用方：本组件的每匹米数输入框；同步执行是因为回显与带出处在同一次赋值栈内。
const echoing = ref(false);

watch(
  () => formData.meters_per_piece,
  val => {
    if (echoing.value) return;
    formData.meters_per_roll = val;
  },
  { flush: 'sync' }
);

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
    meters_per_piece: [
      numericFormatRule('product.productFormDialogTab.validateMetersPerPieceFormat', 4),
    ],
    min_order_quantity: [
      numericFormatRule('product.productFormDialogTab.validateMinOrderQuantityFormat', 2),
    ],
    lead_time: [numericFormatRule('product.productFormDialogTab.validateLeadTimeFormat', 0)],
    price: [numericFormatRule('product.productFormDialogTab.validatePriceFormat', 2)],
    cost_price: [numericFormatRule('product.productFormDialogTab.validatePriceFormat', 2)],
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
        numericFormatRule(
          'product.productFormDialogTab.validateGramWeightFormat',
          2,
          true
        ),
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
  formData.meters_per_piece = '';
  formData.meters_per_roll = '';
  formData.width = '';
  formData.gram_weight = '';
  formData.product_type = '';
  formData.fabric_composition = '';
  formData.yarn_count = '';
  formData.density = '';
  formData.structure = '';
  formData.finish = '';
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
      // 回显区间内抑制带出：resetForm 与各列数值归一都写每匹米数，那属于回填而非用户改动
      echoing.value = true;
      try {
        resetForm();
        if (props.rowData) {
          Object.assign(formData, props.rowData);
          // 数字列后端出参为 Decimal 字符串（NULL 为 null），统一归一成输入框文本；
          // 空值归一为 ''，不写 0，避免历史未录入在产品编辑里被显示成 0 再落回库里
          formData.price = toInputText(props.rowData.price);
          formData.cost_price = toInputText(props.rowData.cost_price);
          formData.meters_per_piece = toInputText(props.rowData.meters_per_piece);
          formData.meters_per_roll = toInputText(props.rowData.meters_per_roll);
          formData.width = toInputText(props.rowData.width);
          formData.gram_weight = toInputText(props.rowData.gram_weight);
          formData.min_order_quantity = toInputText(props.rowData.min_order_quantity);
          formData.lead_time = toInputText(props.rowData.lead_time);
        }
      } finally {
        echoing.value = false;
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
// 米数两键与数值主档键把空值显式序列化为 null，走后端三态更新里的清空通道；键缺席的语义是
// 保持原值，不能把用户的清空做成省略键。
const buildSubmitPayload = (): Partial<Product> => ({
  product_code: formData.product_code,
  product_name: formData.product_name,
  category_id: formData.category_id,
  specification: formData.specification,
  unit: formData.unit,
  barcode: formData.barcode,
  price: toSubmitNumber(formData.price),
  cost_price: toSubmitNumber(formData.cost_price),
  meters_per_piece: toSubmitNumber(formData.meters_per_piece),
  meters_per_roll: toSubmitNumber(formData.meters_per_roll),
  // 幅宽与克重是报价单换算米数与公斤数的取数源，也是每公斤出米数的算式入参，
  // 界面清空必须落得回库里，故与米数同法经 toSubmitNumber 显式发 null 走三态清空通道
  width: toSubmitNumber(formData.width),
  gram_weight: toSubmitNumber(formData.gram_weight),
  // 主档文本列按采集值直发；null 即回显的历史空值，语义为不改动
  product_type: formData.product_type,
  fabric_composition: formData.fabric_composition,
  yarn_count: formData.yarn_count,
  density: formData.density,
  structure: formData.structure,
  finish: formData.finish,
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
