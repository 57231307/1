/**
 * useSupplierProductDialog - 供应商商品新建/编辑对话框 composable
 * 注意：proc 传给子组件时传 reactive 代理本身，不可对象字面量快照。
 */
import { reactive, ref } from 'vue';
import { msg } from '@/utils/message';
import {
  createSupplierProduct,
  updateSupplierProduct,
  type SupplierProduct,
  type CreateSupplierProductPayload,
} from '@/api/supplier-product';

export interface SupplierProductFormState {
  id?: number;
  supplier_id: number | undefined;
  product_code: string;
  product_name: string;
  product_description: string;
  unit: string;
  is_enabled: boolean;
  remarks: string;
}

const defaultForm = (): SupplierProductFormState => ({
  supplier_id: undefined,
  product_code: '',
  product_name: '',
  product_description: '',
  unit: '',
  is_enabled: true,
  remarks: '',
});

export function useSupplierProductDialog(onSuccess: () => void) {
  const dialogVisible = ref(false);
  const isEdit = ref(false);
  const saving = ref(false);
  const form = reactive<SupplierProductFormState>(defaultForm());

  const openCreate = (supplierId?: number) => {
    Object.assign(form, defaultForm());
    form.id = undefined;
    // 从列表页当前供应商带入，避免重复选择
    form.supplier_id = supplierId;
    isEdit.value = false;
    dialogVisible.value = true;
  };

  const openEdit = (row: SupplierProduct) => {
    form.id = row.id;
    form.supplier_id = row.supplier_id;
    form.product_code = row.product_code;
    form.product_name = row.product_name;
    form.product_description = row.product_description ?? '';
    form.unit = row.unit;
    form.is_enabled = row.is_enabled;
    form.remarks = row.remarks ?? '';
    isEdit.value = true;
    dialogVisible.value = true;
  };

  const submit = async () => {
    if (!form.supplier_id || !form.product_code || !form.product_name || !form.unit) {
      msg.warning('supplierProductRequired');
      return;
    }
    saving.value = true;
    try {
      const payload: CreateSupplierProductPayload = {
        supplier_id: form.supplier_id,
        product_code: form.product_code,
        product_name: form.product_name,
        product_description: form.product_description || null,
        unit: form.unit,
        is_enabled: form.is_enabled,
        remarks: form.remarks || null,
      };
      if (isEdit.value && form.id) {
        await updateSupplierProduct(form.id, payload);
        msg.saveOk();
      } else {
        await createSupplierProduct(payload);
        msg.createOk();
      }
      dialogVisible.value = false;
      onSuccess();
    } catch (error: unknown) {
      msg.saveFail();
      console.error('[supplier-product] submit error:', error);
    } finally {
      saving.value = false;
    }
  };

  return {
    dialogVisible,
    isEdit,
    saving,
    form,
    openCreate,
    openEdit,
    submit,
  };
}
