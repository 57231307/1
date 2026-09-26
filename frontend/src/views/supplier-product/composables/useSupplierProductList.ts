/**
 * useSupplierProductList - 供应商商品目录列表 composable
 * 按供应商过滤 + 关键字搜索 + 分页；顺带加载供应商下拉供筛选与表单复用。
 */
import { reactive, ref, onMounted } from 'vue';
import { msg } from '@/utils/message';
import {
  getSupplierProductList,
  updateSupplierProduct,
  type SupplierProduct,
  type SupplierProductQueryParams,
  type UpdateSupplierProductPayload,
} from '@/api/supplier-product';
import { getSupplierList } from '@/api/supplier';
import type { Supplier } from '@/api/supplier';

export function useSupplierProductList() {
  const loading = ref(false);
  const list = ref<SupplierProduct[]>([]);
  const total = ref(0);
  const suppliers = ref<Supplier[]>([]);

  const queryParams = reactive<SupplierProductQueryParams>({
    supplier_id: undefined,
    keyword: '',
    page: 1,
    page_size: 20,
  });

  const loadData = async () => {
    if (!queryParams.supplier_id) {
      list.value = [];
      total.value = 0;
      return;
    }
    loading.value = true;
    try {
      const res = await getSupplierProductList(queryParams);
      list.value = res.data.items;
      total.value = res.data.total;
    } catch (error: unknown) {
      msg.loadFail();
      console.error('[supplier-product] loadData error:', error);
    } finally {
      loading.value = false;
    }
  };

  const loadSuppliers = async () => {
    try {
      const res = await getSupplierList({ page_size: 200 });
      suppliers.value = res.data.items;
    } catch (error: unknown) {
      console.error('[supplier-product] loadSuppliers error:', error);
    }
  };

  const handleQuery = () => {
    queryParams.page = 1;
    loadData();
  };

  const handleReset = () => {
    queryParams.keyword = '';
    queryParams.page = 1;
    loadData();
  };

  const handlePageChange = (page: number) => {
    queryParams.page = page;
    loadData();
  };

  const handleSizeChange = (size: number) => {
    queryParams.page_size = size;
    queryParams.page = 1;
    loadData();
  };

  // 软停用：表被 FK 引用无硬删，置 is_enabled=false（整替换语义，回填其余必填字段）
  const toggleEnabled = async (row: SupplierProduct) => {
    const payload: UpdateSupplierProductPayload = {
      supplier_id: row.supplier_id,
      product_code: row.product_code,
      product_name: row.product_name,
      product_description: row.product_description,
      unit: row.unit,
      is_enabled: !row.is_enabled,
      remarks: row.remarks,
    };
    try {
      await updateSupplierProduct(row.id, payload);
      msg.updateOk();
      loadData();
    } catch (error: unknown) {
      msg.updateFail();
      console.error('[supplier-product] toggleEnabled error:', error);
    }
  };

  onMounted(() => {
    loadSuppliers();
  });

  return {
    loading,
    list,
    total,
    suppliers,
    queryParams,
    loadData,
    handleQuery,
    handleReset,
    handlePageChange,
    handleSizeChange,
    toggleEnabled,
  };
}
