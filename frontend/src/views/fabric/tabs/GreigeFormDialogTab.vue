<!--
  GreigeFormDialogTab.vue - 坯布编辑对话框
  来源：原 fabric/index.vue 中 坯布编辑对话框
  拆分日期：2026-06-15 B3-4
-->
<template>
  <el-dialog
    :model-value="modelValue"
    :title="
      formData.id
        ? t('fabric.greigeFormDialog.titleEdit')
        : t('fabric.greigeFormDialog.titleCreate')
    "
    width="600px"
    :aria-label="
      formData.id
        ? t('fabric.greigeFormDialog.titleEdit')
        : t('fabric.greigeFormDialog.titleCreate')
    "
    @update:model-value="(val: boolean) => emit('update:modelValue', val)"
  >
    <el-form
      ref="formRef"
      :model="formData"
      :rules="rules"
      label-width="100px"
      :aria-label="t('fabric.greigeFormDialog.formAriaLabel')"
    >
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelCode')" prop="fabric_no">
            <el-input v-model="formData.fabric_no" :disabled="!!formData.id" />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelName')" prop="fabric_name">
            <el-input v-model="formData.fabric_name" />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('fabric.greigeFormDialog.labelFabricType')" prop="fabric_type">
        <el-input
          v-model="formData.fabric_type"
          :placeholder="t('fabric.greigeFormDialog.placeholderFabricType')"
        />
      </el-form-item>
      <el-form-item :label="t('fabric.greigeFormDialog.labelSupplier')" prop="supplier_id">
        <el-select v-model="formData.supplier_id" style="width: 100%">
          <el-option v-for="s in suppliers" :key="s.id" :label="s.supplier_name" :value="s.id" />
        </el-select>
      </el-form-item>
      <el-row :gutter="20">
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelWidth')" prop="width">
            <!-- 后端 UpdateGreigeFabricRequest 无 width/gram_weight 键：编辑态不可改 -->
            <el-input-number
              v-model="formData.width"
              :min="0"
              :disabled="!!formData.id"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
        <el-col :span="12">
          <el-form-item :label="t('fabric.greigeFormDialog.labelWeight')" prop="gram_weight">
            <el-input-number
              v-model="formData.gram_weight"
              :min="0"
              :disabled="!!formData.id"
              style="width: 100%"
            />
          </el-form-item>
        </el-col>
      </el-row>
      <el-form-item :label="t('fabric.greigeFormDialog.labelComposition')" prop="composition">
        <el-input
          v-model="formData.composition"
          :disabled="!!formData.id"
          :placeholder="t('fabric.greigeFormDialog.placeholderComposition')"
        />
      </el-form-item>
    </el-form>
    <template #footer>
      <el-button @click="emit('update:modelValue', false)">{{
        t('fabric.common.cancel')
      }}</el-button>
      <el-button type="primary" :loading="submitLoading" @click="handleSubmit">{{
        t('fabric.common.confirm')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, reactive, computed, watch } from 'vue';
import { useI18n } from 'vue-i18n';
import { ElMessage } from 'element-plus';
import type { FormInstance, FormRules } from 'element-plus';
import {
  createGreigeFabric,
  updateGreigeFabric,
  GREIGE_STATUS,
  type GreigeFabric,
  type GreigeStatusValue,
  type CreateGreigeFabricPayload,
  type UpdateGreigeFabricPayload,
} from '@/api/greige-fabric';
import type { Supplier } from '@/api/supplier';
import { logger } from '@/utils/logger';
import { decimalWireToNumber } from '@/utils/money';

const { t } = useI18n({ useScope: 'global' });

interface Props {
  modelValue: boolean;
  currentRow: GreigeFabric | null;
  suppliers: Supplier[];
}

interface Emits {
  (e: 'update:modelValue', val: boolean): void;
  (e: 'submitted'): void;
}

const props = defineProps<Props>();
const emit = defineEmits<Emits>();

const formRef = ref<FormInstance>();
const submitLoading = ref(false);

const formData = reactive({
  id: 0,
  fabric_no: '',
  fabric_name: '',
  fabric_type: '',
  supplier_id: undefined as number | undefined,
  width: 0,
  // 后端坯布 Model 无 weight 列：克重真实落库键为 gram_weight（g/m²，greige_fabric.rs）
  gram_weight: 0,
  composition: '',
  status: GREIGE_STATUS.IN_STOCK as GreigeStatusValue,
});

const rules = computed<FormRules>(() => ({
  fabric_type: [
    { required: true, message: t('fabric.greigeFormDialog.fabricTypeRequired'), trigger: 'blur' },
    {
      validator: (_rule, value, callback) => {
        if (typeof value === 'string' && value.trim() === '') {
          callback(new Error(t('fabric.greigeFormDialog.fabricTypeRequired')));
        } else {
          callback();
        }
      },
      trigger: 'blur',
    },
  ],
}));

const resetForm = () => {
  formData.id = 0;
  formData.fabric_no = '';
  formData.fabric_name = '';
  formData.fabric_type = '';
  formData.supplier_id = undefined;
  formData.width = 0;
  formData.gram_weight = 0;
  formData.composition = '';
  formData.status = GREIGE_STATUS.IN_STOCK;
};

watch(
  () => props.modelValue,
  val => {
    if (val) {
      if (props.currentRow) {
        const row = props.currentRow;
        formData.id = row.id;
        formData.fabric_no = row.fabric_no ?? '';
        formData.fabric_name = row.fabric_name ?? '';
        formData.fabric_type = row.fabric_type ?? '';
        formData.supplier_id = row.supplier_id ?? undefined;
        // 后端 Decimal 出参为十进制字符串，回填 el-input-number 前用 utils/money 归一
        formData.width = row.width != null ? decimalWireToNumber(row.width) : 0;
        formData.gram_weight = row.gram_weight != null ? decimalWireToNumber(row.gram_weight) : 0;
        formData.composition = row.composition ?? '';
        // 后端状态列只会是 GREIGE_STATUS 中文 token；词表外取值属脏数据，不静默改写
        formData.status = (row.status as GreigeStatusValue) ?? GREIGE_STATUS.IN_STOCK;
      } else {
        resetForm();
      }
    }
  }
);

const handleSubmit = async () => {
  const valid = await formRef.value?.validate().catch(() => false);
  if (!valid) return;
  submitLoading.value = true;
  try {
    if (formData.id) {
      // 后端 UpdateGreigeFabricRequest 不含 width/composition（创建后不可改），
      // 编辑态只提交该 DTO 真实接收的键；对应输入框已在模板中禁用
      const payload: UpdateGreigeFabricPayload = {
        fabric_name: formData.fabric_name || undefined,
        fabric_type: formData.fabric_type,
        supplier_id: formData.supplier_id,
        status: formData.status,
      };
      await updateGreigeFabric(formData.id, payload);
    } else {
      const payload: CreateGreigeFabricPayload = {
        // 白坯四维口径：编号留空整键省略，由后端自动生成
        fabric_no: formData.fabric_no || undefined,
        fabric_name: formData.fabric_name || undefined,
        fabric_type: formData.fabric_type,
        supplier_id: formData.supplier_id,
        width: formData.width || undefined,
        gram_weight: formData.gram_weight || undefined,
        composition: formData.composition || undefined,
        status: formData.status,
      };
      await createGreigeFabric(payload);
    }
    ElMessage.success(t('fabric.common.success'));
    emit('update:modelValue', false);
    emit('submitted');
  } catch (error) {
    const err = error as Error;
    ElMessage.error(err.message || t('fabric.common.failed'));
    logger.error(t('fabric.greigeFormDialog.saveFailed'), err.message);
  } finally {
    submitLoading.value = false;
  }
};
</script>
