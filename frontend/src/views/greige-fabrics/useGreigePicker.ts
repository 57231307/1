/**
 * 坯布页实体选择器的远程候选组合式。
 *
 * 功能：把坯布建档与出入库表单里「供应商 / 仓库 / 产品 / 采购入库单」四个实体字段从手填
 * 数字主键改成向服务端按关键字远程搜索，候选、加载中标志与请求序号在这里单点持有，
 * 已选项的名称回显由钉住项保住，编辑态回显按主键单条取回。
 * 调用方：components/GreigeFormDialog.vue 的建档与编辑表单，components/GreigeStockDialog.vue
 * 的入库对话框。两处只在控件上消费本组合式返回的状态与方法，不各自再装一套请求。
 * 入参：load 由调用方给出（关键字到候选项的一次取数，内部调对应 api 函数），
 *      loadOne 由调用方给出（按主键取单条，用于编辑回显名称）。
 * 传给谁：load 与 loadOne 落到 api/supplier.ts、api/warehouse.ts、api/product.ts、
 *      api/purchase-receipt.ts 的既有列表与详情函数，键名一律取后端查询结构体，前端不自造。
 * 存什么：当前关键字取回的候选、已选主键对应的钉住项、加载中标志、请求序号。
 * 存哪里：全部是本组合式闭包内的响应式状态，不写入全局 store，也不落浏览器存储。
 */
import { computed, ref } from 'vue';

import { i18n } from '@/i18n';
import { logAuxLoadFailure, logger } from '@/utils/logger';
import { msg } from '@/utils/message';

/** 控件消费的候选项：主键加可读名称，名称只用于展示，提交值仍是主键 */
export interface PickerOption {
  id: number;
  label: string;
}

/**
 * 候选单页条数。后端各列表端点对 page_size 做 clamp(1,100)，传大于 100 的值会被静默削到
 * 100，让人拿到被削过的结果还以为取全了，这是本仓已知的列表分页家族缺陷。
 * 因此固定取 100 以内的值，靠关键字与归属条件收窄候选，不靠加大单页把候选池假装变大。
 */
export const PICKER_PAGE_SIZE = 50;

/**
 * 按主键单条取回失败时的兜底展示项文案。
 * 只回显后端真实存在的主键编号，不伪造名称：名称取不到时用户至少要看到「这一项是谁」，
 * 而不是看到空白控件误以为字段没填（清空并提交才会真的清掉，见 syncPinned）。
 */
const idFallbackLabel = (id: number): string =>
  i18n.global.t('greigeFabrics.picker.idFallback', { id });

export function useGreigePicker(
  load: (keyword: string) => Promise<PickerOption[]>,
  loadOne: (id: number) => Promise<PickerOption | null>
) {
  /** 当前关键字向服务端取回的候选，换关键字即整批丢弃，不与上一个关键字的结果混在一起 */
  const candidates = ref<PickerOption[]>([]);
  /** 已选项的钉住集合：远程搜索换关键字后候选会被整批替换，靠它保住已选项的名称回显 */
  const pinned = ref<PickerOption[]>([]);
  const loading = ref(false);

  /**
   * 请求序号。后发请求为代表，旧关键字的响应晚到一律丢弃，既不得覆盖新关键字取到的候选，
   * 也不得用旧关键字的失败清空新关键字已取到的数据。
   */
  let searchSeq = 0;

  /** 控件可见候选：当前关键字的结果在前，未落在其中的钉住项补在后面 */
  const options = computed<PickerOption[]>(() => {
    const missing = pinned.value.filter(item =>
      candidates.value.every(current => current.id !== item.id)
    );
    return candidates.value.concat(missing);
  });

  /**
   * 按关键字向服务端取候选，不做本地过滤。
   * @param keyword 用户已输入的文本，原样交给调用方给出的 load；空串由请求层与后端双侧剔除，
   *        等价于不带筛选，因此这里不把空串换成其它值，也不据此走本地分支。
   */
  const search = async (keyword: string): Promise<void> => {
    const seq = ++searchSeq;
    candidates.value = [];
    loading.value = true;
    try {
      const items = await load(keyword);
      if (seq !== searchSeq) return;
      candidates.value = items;
    } catch (error: unknown) {
      if (seq !== searchSeq) return;
      candidates.value = [];
      logAuxLoadFailure('[greige] 坯布实体候选远程搜索失败', error);
      msg.error('loadFailed');
    } finally {
      if (seq === searchSeq) loading.value = false;
    }
  };

  /**
   * 把钉住集合换成恰好等于当前选中项。控件的值一定来自 options，按主键取不到即状态异常，
   * 记警告并跳过该项，不做假回显、不静默改写。
   * @param id 控件 change 事件给出的已选主键；清空时控件按自身空值配置交回 null 或空串，
   *        非数字一律按未选中处理，钉住集合随之清空
   */
  const syncPinned = (id: number | null | undefined): void => {
    if (typeof id !== 'number') {
      pinned.value = [];
      return;
    }
    const found = options.value.find(item => item.id === id);
    if (found === undefined) {
      logger.warn('[greige] 选中项不在当前候选并集中，名称回显无法钉住', { id });
      return;
    }
    pinned.value = [found];
  };

  /**
   * 编辑态回显：按已存主键单条取回名称。
   * 取回失败只影响名称展示，绝不清空已选主键（主键仍在表单值里，原样提交即保持原值）；
   * 取不到名称时钉一个只含主键编号的兜底项，避免用户看到空白控件误判成字段没填。
   * @param id 后端行里存的实体主键，null 与 undefined 表示未关联，直接清空钉住集合
   */
  const restore = async (id: number | null | undefined): Promise<void> => {
    if (typeof id !== 'number') {
      pinned.value = [];
      return;
    }
    if (pinned.value.some(item => item.id === id) || candidates.value.some(item => item.id === id)) {
      return;
    }
    try {
      const found = await loadOne(id);
      // 名称取到即用真实名称；后端按主键查无此行时同样落兜底项，不假装成功
      pinned.value = [found ?? { id, label: idFallbackLabel(id) }];
    } catch (error: unknown) {
      logAuxLoadFailure('[greige] 坯布关联实体名称回显失败', error);
      pinned.value = [{ id, label: idFallbackLabel(id) }];
    }
  };

  /** 丢弃当前关键字的候选与钉住项（对话框关闭或换行编辑时调用，避免跨行串候选） */
  const reset = (): void => {
    searchSeq += 1;
    candidates.value = [];
    pinned.value = [];
    loading.value = false;
  };

  return { options, loading, search, syncPinned, restore, reset };
}
