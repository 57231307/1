/**
 * useSupplierProductColor - 某供应商商品的色号维护 composable
 * 外层对话框列出该商品色号（搜索/分页），内层对话框新建/编辑单条色号。
 * 表被 FK 引用无硬删，停用走 update is_enabled=false。
 */
import { reactive, ref } from 'vue';
import { msg } from '@/utils/message';
import {
  getSupplierProductColorList,
  createSupplierProductColor,
  updateSupplierProductColor,
  type SupplierProduct,
  type SupplierProductColor,
  type SupplierProductColorQueryParams,
  type CreateSupplierProductColorPayload,
} from '@/api/supplier-product';

export interface SupplierProductColorFormState {
  id?: number;
  supplier_product_id: number;
  color_no: string;
  color_name: string;
  pantone_code: string;
  /** 字符串承载 Decimal，禁止对字符串直接 .toFixed */
  extra_cost: string;
  is_enabled: boolean;
}

const defaultColorForm = (supplierProductId: number): SupplierProductColorFormState => ({
  supplier_product_id: supplierProductId,
  color_no: '',
  color_name: '',
  pantone_code: '',
  extra_cost: '0',
  is_enabled: true,
});

export function useSupplierProductColor(onSuccess: () => void) {
  // 外层：色号列表对话框
  const dialogVisible = ref(false);
  const currentProduct = ref<SupplierProduct | null>(null);
  const loading = ref(false);
  const list = ref<SupplierProductColor[]>([]);
  const total = ref(0);
  const queryParams = reactive<SupplierProductColorQueryParams>({
    supplier_product_id: undefined,
    keyword: '',
    page: 1,
    page_size: 20,
  });

  // 内层：单条色号表单对话框
  const formVisible = ref(false);
  const isEdit = ref(false);
  const saving = ref(false);
  const form = reactive<SupplierProductColorFormState>(defaultColorForm(0));

  const loadColors = async () => {
    if (!queryParams.supplier_product_id) return;
    loading.value = true;
    try {
      const res = await getSupplierProductColorList(queryParams);
      list.value = res.data.items;
      total.value = res.data.total;
    } catch (error: unknown) {
      msg.loadFail();
      console.error('[supplier-product-color] loadColors error:', error);
    } finally {
      loading.value = false;
    }
  };

  const open = (product: SupplierProduct) => {
    currentProduct.value = product;
    queryParams.supplier_product_id = product.id;
    queryParams.keyword = '';
    queryParams.page = 1;
    list.value = [];
    total.value = 0;
    dialogVisible.value = true;
    loadColors();
  };

  const handleQuery = () => {
    queryParams.page = 1;
    loadColors();
  };

  const handlePageChange = (page: number) => {
    queryParams.page = page;
    loadColors();
  };

  const handleSizeChange = (size: number) => {
    queryParams.page_size = size;
    queryParams.page = 1;
    loadColors();
  };

  const openCreate = () => {
    if (!currentProduct.value) return;
    Object.assign(form, defaultColorForm(currentProduct.value.id));
    form.id = undefined;
    isEdit.value = false;
    formVisible.value = true;
  };

  const openEdit = (row: SupplierProductColor) => {
    form.id = row.id;
    form.supplier_product_id = row.supplier_product_id;
    form.color_no = row.color_no;
    form.color_name = row.color_name;
    form.pantone_code = row.pantone_code ?? '';
    form.extra_cost = row.extra_cost;
    form.is_enabled = row.is_enabled;
    isEdit.value = true;
    formVisible.value = true;
  };

  const submit = async () => {
    if (!form.supplier_product_id || !form.color_no || !form.color_name) {
      msg.warning('supplierProductColorRequired');
      return;
    }
    saving.value = true;
    try {
      const payload: CreateSupplierProductColorPayload = {
        supplier_product_id: form.supplier_product_id,
        color_no: form.color_no,
        color_name: form.color_name,
        pantone_code: form.pantone_code || null,
        // 空串归一为 '0'；始终发字符串承载 Decimal
        extra_cost: form.extra_cost === '' ? '0' : form.extra_cost,
        is_enabled: form.is_enabled,
      };
      if (isEdit.value && form.id) {
        await updateSupplierProductColor(form.id, payload);
        msg.saveOk();
      } else {
        await createSupplierProductColor(payload);
        msg.createOk();
      }
      formVisible.value = false;
      await loadColors();
      onSuccess();
    } catch (error: unknown) {
      msg.saveFail();
      console.error('[supplier-product-color] submit error:', error);
    } finally {
      saving.value = false;
    }
  };

  const toggleEnabled = async (row: SupplierProductColor) => {
    const payload: CreateSupplierProductColorPayload = {
      supplier_product_id: row.supplier_product_id,
      color_no: row.color_no,
      color_name: row.color_name,
      pantone_code: row.pantone_code,
      extra_cost: row.extra_cost,
      is_enabled: !row.is_enabled,
    };
    try {
      await updateSupplierProductColor(row.id, payload);
      msg.updateOk();
      await loadColors();
    } catch (error: unknown) {
      msg.updateFail();
      console.error('[supplier-product-color] toggleEnabled error:', error);
    }
  };

  return {
    dialogVisible,
    currentProduct,
    loading,
    list,
    total,
    queryParams,
    open,
    loadColors,
    handleQuery,
    handlePageChange,
    handleSizeChange,
    // 内层表单
    formVisible,
    isEdit,
    saving,
    form,
    openCreate,
    openEdit,
    submit,
    toggleEnabled,
  };
}
