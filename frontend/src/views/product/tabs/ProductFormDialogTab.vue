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
            <el-input-number
              v-model="formData.meters_per_piece"
              :min="0"
              :precision="4"
              :controls="false"
              :value-on-clear="null"
              style="width: 100%"
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
            <el-input-number
              v-model="formData.meters_per_roll"
              :min="0"
              :precision="4"
              :controls="false"
              style="width: 100%"
              readonly
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelPrice')" prop="price">
            <el-input-number v-model="formData.price" :min="0" :precision="2" style="width: 100%" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('product.productFormDialogTab.labelCostPrice')" prop="cost_price">
            <el-input-number
              v-model="formData.cost_price"
              :min="0"
              :precision="2"
              style="width: 100%"
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
import type { FormInstance, FormRules } from 'element-plus';
import type { Product, ProductCategory } from '@/api/product';
import { createProduct, updateProduct } from '@/api/product';
import { QUOTATION_UNIT_LABEL_KEY, QUOTATION_UNIT_VALUES } from '@/constants/quotation-unit';
import { logger } from '@/utils/logger';

const { t } = useI18n({ useScope: 'global' });

// 交易单位下拉：value 用库里稳定的中文单位 token，label 走 i18n；报价行单位随此值带出
const unitOptions = computed(() =>
  QUOTATION_UNIT_VALUES.map(value => ({
    value,
    label: t(QUOTATION_UNIT_LABEL_KEY[value]),
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
  price: 0,
  cost_price: 0,
  // 清空经 value-on-clear 归一为 null，与未触碰的 undefined 同属空值，提交前统一显式序列化为 null
  meters_per_piece: undefined as number | null | undefined,
  meters_per_roll: undefined as number | null | undefined,
  description: '',
  is_active: true,
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

const formRules: FormRules = {
  product_code: [
    {
      required: true,
      message: t('product.productFormDialogTab.validateProductCodeRequired'),
      trigger: 'blur',
    },
    { max: 50, message: t('product.productFormDialogTab.validateProductCodeMax'), trigger: 'blur' },
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
};

const resetForm = () => {
  formData.id = undefined;
  formData.product_code = '';
  formData.product_name = '';
  formData.category_id = undefined;
  formData.specification = '';
  formData.unit = '';
  formData.barcode = '';
  formData.price = 0;
  formData.cost_price = 0;
  formData.meters_per_piece = undefined;
  formData.meters_per_roll = undefined;
  formData.description = '';
  formData.is_active = true;
  formRef.value?.clearValidate();
};

watch(
  () => props.modelValue,
  val => {
    visible.value = val;
    if (val) {
      // 回显区间内抑制带出：resetForm 与两列归一都写每匹米数，那属于回填而非用户改动
      echoing.value = true;
      try {
        resetForm();
        if (props.rowData) {
          Object.assign(formData, props.rowData);
          // 后端 Decimal 序列化为字符串，el-input-number 需数值：非空才 Number() 归一，真实空值保持 undefined
          formData.meters_per_piece =
            props.rowData.meters_per_piece != null
              ? Number(props.rowData.meters_per_piece)
              : undefined;
          formData.meters_per_roll =
            props.rowData.meters_per_roll != null
              ? Number(props.rowData.meters_per_roll)
              : undefined;
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
// 时间戳等由服务端掌管，客户端发了也会被反序列化丢弃，留着只会掩盖契约漂移）。
// 米数两键把空值显式序列化为 null，走后端三态更新里的清空通道；键缺席的语义是
// 保持原值，不能把用户的清空做成省略键。
const buildSubmitPayload = (): Partial<Product> => ({
  product_code: formData.product_code,
  product_name: formData.product_name,
  category_id: formData.category_id,
  specification: formData.specification,
  unit: formData.unit,
  barcode: formData.barcode,
  price: formData.price,
  cost_price: formData.cost_price,
  meters_per_piece: formData.meters_per_piece ?? null,
  meters_per_roll: formData.meters_per_roll ?? null,
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
