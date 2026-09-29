/**
 * 染料类型（ai_process_optimizations.dye_type 等）取值清单。
 *
 * 后端在入口处按白名单校验（handlers/ai_extend_handler.rs：
 * reactive/活性、disperse/分散、acid/酸性、vat/还原、direct/直接、cationic/阳离子、
 * sulfur/硫化），越界直接 422 并报允许值；库里保存的是提交的那个写法。
 *
 * 界面统一提交英文码：此前 value 取译文（活性染料 / Reactive Dye），
 * 白名单里既没有"活性染料"也没有英文名，任何选择都会被 422 拒绝。
 */
export const DYE_TYPE = {
  reactive: 'reactive',
  disperse: 'disperse',
  acid: 'acid',
  vat: 'vat',
  direct: 'direct',
  cationic: 'cationic',
  sulfur: 'sulfur',
} as const;

export type DyeTypeValue = (typeof DYE_TYPE)[keyof typeof DYE_TYPE];

export const DYE_TYPE_VALUES: DyeTypeValue[] = [
  DYE_TYPE.reactive,
  DYE_TYPE.disperse,
  DYE_TYPE.acid,
  DYE_TYPE.vat,
  DYE_TYPE.direct,
  DYE_TYPE.cationic,
  DYE_TYPE.sulfur,
];

export const DYE_TYPE_LABEL_KEY: Record<DyeTypeValue, string> = {
  [DYE_TYPE.reactive]: 'aiExtend.process.dyeReactive',
  [DYE_TYPE.disperse]: 'aiExtend.process.dyeDisperse',
  [DYE_TYPE.acid]: 'aiExtend.process.dyeAcid',
  [DYE_TYPE.vat]: 'aiExtend.process.dyeVat',
  [DYE_TYPE.direct]: 'aiExtend.process.dyeDirect',
  [DYE_TYPE.cationic]: 'aiExtend.process.dyeCationic',
  [DYE_TYPE.sulfur]: 'aiExtend.process.dyeSulfur',
};

/** 判断已存库的染料值是否在本词表内（越界值界面按原样展示，不猜测其含义） */
export const isDyeType = (value: string): value is DyeTypeValue =>
  DYE_TYPE_VALUES.includes(value as DyeTypeValue);
