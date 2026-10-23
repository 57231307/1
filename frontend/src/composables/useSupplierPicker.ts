/**
 * 供应商主数据可搜索选择器的数据源组合式。
 * 功能：把供应商 360 与供应商评估两处的供应商入口从手填数字主键改成按名称或编码关键字
 * 向服务端远程搜索，候选、加载中标志与请求序号在这里单点持有，已选项的名称回显由钉住项保住。
 * 调用方：src/views/supplier/enhanced/index.vue 的单选入口，src/views/supplier-evaluation/index.vue
 * 的对比多选入口。两处只在控件上消费本组合式返回的状态与方法，不各自再装一套请求。
 * 入参：无。搜索关键字由控件的 remote-method 传入，已选主键由控件的 change 事件传入。
 * 传给谁：getSupplierList，即 GET /purchase/suppliers。关键字键名 keyword 与分页键名 page、
 * page_size 全部来自后端 SupplierQueryParams（backend/src/services/supplier_service.rs），
 * 前端不另起第二套键名。已选主键由调用方页面继续用于供应商详情与评估各端点。
 * 存什么：当前关键字取回的候选、全部关键字累计取回过的钉住项、加载中标志、请求序号。
 * 存哪里：全部是本组合式闭包内的响应式状态，不写入全局 store，也不落浏览器存储。
 */
import { computed, ref } from 'vue';

import { i18n } from '@/i18n';
import { getSupplierList, type Supplier } from '@/api/supplier';
import { logAuxLoadFailure, logger } from '@/utils/logger';
import { msg } from '@/utils/message';

/**
 * 候选单页条数。后端 supplier_service.rs 对 page_size 做 clamp(1,100)，传大于 100 的值会被
 * 静默削到 100，让人拿到被削过的结果还以为取全了，这是本仓已知的列表分页家族缺陷。
 * 因此这里固定取 100 以内的值，靠关键字收窄候选而不是靠加大单页把候选池假装变大。
 */
const PICKER_PAGE_SIZE = 50;
const PICKER_PAGE = 1;

export function useSupplierPicker() {
  /** 当前关键字向服务端取回的候选，换关键字即整批丢弃，不与上一个关键字的结果混在一起 */
  const candidates = ref<Supplier[]>([]);
  /** 已选项的钉住集合：远程搜索换关键字后候选会被整批替换，靠它保住已选供应商的名称回显 */
  const pinned = ref<Supplier[]>([]);
  const loading = ref(false);

  /**
   * 请求序号。后发请求为代表，旧关键字的响应晚到一律丢弃，既不得覆盖新关键字取到的候选，
   * 也不得用旧关键字的失败清空新关键字已取到的数据。
   */
  let searchSeq = 0;

  /** 选项文案：名称与编码同列，编码是本页此前唯一可见的供应商标识，保留便于与旧习惯对照 */
  const optionLabel = (supplier: Supplier): string =>
    i18n.global.t('supplier.enhanced.picker.optionLabel', {
      name: supplier.supplier_name,
      code: supplier.supplier_code,
    });

  /** 控件可见候选：当前关键字的结果在前，未落在其中的钉住项补在后面 */
  const options = computed<Supplier[]>(() => {
    const missing = pinned.value.filter(item =>
      candidates.value.every(current => current.id !== item.id)
    );
    return candidates.value.concat(missing);
  });

  /**
   * 按关键字向服务端取候选，不做本地过滤。
   * @param keyword 用户已输入的文本，原样下发到后端 keyword 键；空串由请求层与后端双侧剔除，
   *        等价于不带筛选，因此这里不把空串换成其它值，也不据此走本地分支。
   */
  const search = async (keyword: string): Promise<void> => {
    const seq = ++searchSeq;
    candidates.value = [];
    loading.value = true;
    try {
      const res = await getSupplierList({
        page: PICKER_PAGE,
        page_size: PICKER_PAGE_SIZE,
        keyword: keyword.trim(),
      });
      if (seq !== searchSeq) return;
      candidates.value = res.data.items;
    } catch (error: unknown) {
      if (seq !== searchSeq) return;
      candidates.value = [];
      logAuxLoadFailure('[supplier] 供应商候选远程搜索失败', error);
      msg.error('loadFailed');
    } finally {
      if (seq === searchSeq) loading.value = false;
    }
  };

  /**
   * 把钉住集合换成恰好等于当前选中项。控件的值一定来自 options，按主键取不到即状态异常，
   * 记警告并跳过该项，不做假回显、不静默改写。
   * @param ids 控件 change 事件给出的已选主键，单选传单个数字，多选传数组；
   *        清空时控件按自身的空值配置交回 undefined 或空串，非数字一律按未选中处理
   */
  const syncPinned = (ids: number | number[] | null | undefined): void => {
    const pickedIds = (Array.isArray(ids) ? ids : [ids]).filter(
      (id): id is number => typeof id === 'number'
    );
    // 必须在替换前取并集候选：钉住项是上一次的候选，替换后就只剩当前关键字的结果了
    const pool = options.value;
    const next: Supplier[] = [];
    for (const id of pickedIds) {
      const found = pool.find(item => item.id === id);
      if (found === undefined) {
        logger.warn('[supplier] 选中项不在当前候选并集中，名称回显无法钉住', { id });
        continue;
      }
      next.push(found);
    }
    pinned.value = next;
  };

  return { options, loading, search, syncPinned, optionLabel };
}
