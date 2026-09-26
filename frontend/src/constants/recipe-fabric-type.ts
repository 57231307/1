/**
 * 染色处方布类（dye_recipe.fabric_type / 处方优化请求的 fabric_type）取值清单。
 *
 * 该列没有编码词表，库里存的就是中文布类名（迁移里是 VARCHAR 自由文本，无 CHECK），
 * 后端配伍判断按 `eq_ignore_ascii_case` 比对服务内常量列表
 * （services/ai/recipe_opt.rs 的 compatibility 表：cotton/棉/棉布、涤纶、真丝/丝绸、羊毛）。
 *
 * 因此界面必须提交这些中文稳定值本身：此前 el-option 的 value 取译文，
 * 英文界面提交 "Cotton"/"Synthetic" 一律配不上（甚至触发 422 不配伍）。
 * 化纤/合成纤维不在后端配伍表内，故不作为选项（要支持需先扩表，已登记台账）。
 */
export const RECIPE_FABRIC_TYPE = {
  cotton: '棉',
  polyester: '涤纶',
  silk: '丝绸',
  wool: '羊毛',
} as const;

export type RecipeFabricTypeValue = (typeof RECIPE_FABRIC_TYPE)[keyof typeof RECIPE_FABRIC_TYPE];

export const RECIPE_FABRIC_TYPE_VALUES: RecipeFabricTypeValue[] = [
  RECIPE_FABRIC_TYPE.cotton,
  RECIPE_FABRIC_TYPE.polyester,
  RECIPE_FABRIC_TYPE.silk,
  RECIPE_FABRIC_TYPE.wool,
];

/** 布类 → i18n 文案键 */
export const RECIPE_FABRIC_TYPE_LABEL_KEY: Record<RecipeFabricTypeValue, string> = {
  [RECIPE_FABRIC_TYPE.cotton]: 'advancedModule.recipe.fabricCotton',
  [RECIPE_FABRIC_TYPE.polyester]: 'advancedModule.recipe.fabricPolyester',
  [RECIPE_FABRIC_TYPE.silk]: 'advancedModule.recipe.fabricSilk',
  [RECIPE_FABRIC_TYPE.wool]: 'advancedModule.recipe.fabricWool',
};
