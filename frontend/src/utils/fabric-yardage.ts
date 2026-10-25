/**
 * 面料每公斤出米数派生工具（前端只读展示用的唯一算式点）。
 *
 * 与后端 backend/src/utils/dual_unit_converter.rs::DualUnitConverter::meters_per_kg 严格同源：
 * 米/公斤 = 1000 ÷ (克重 g/m² × 幅宽 m)，幅宽 m = 幅宽 cm ÷ 100，代入化简即
 * 100000 ÷ (克重 × 幅宽cm)；取整口径为向下取整保留一位小数（floor，非四舍五入）。
 * 该文件是前端算式的单点，产品表单只在编辑未落库的幅宽或克重时用它即时带出只读出米数；
 * 落库后的读值由后端在同一算式下随出参下发 meters_per_kg，两处不各写一套公式。
 * 同源关系由 tests/unit 的静态锁断言本文件的常量与后端算式一致，改任一侧即红。
 */

// 与后端算式同源的常量：克重按 g/m²、幅宽入参按 cm，公斤→克与米→厘米各除一次
const GRAM_PER_KG = 1000;
const CM_PER_M = 100;
// 向下取整保留一位小数用到的倍率（放大 10 倍取 floor 再缩回）
const FLOOR_SCALE = 10;

/**
 * 由克重与幅宽算每公斤出米数（向下取整保留一位小数）。
 * 入参：gramWeight 克重(g/m²)、widthCm 幅宽(cm)，均取自产品表单当前值。
 * 任一为空、非有限数或 ≤0 时返回 null（未录入不可算，不算成 0 冒充「每公斤出 0 米」）。
 * 返回：一位小数的米数（number），供只读展示；不落库、不发回后端。
 */
export function computeMetersPerKg(
  gramWeight: number | null | undefined,
  widthCm: number | null | undefined
): number | null {
  if (gramWeight == null || widthCm == null) return null;
  if (!Number.isFinite(gramWeight) || !Number.isFinite(widthCm)) return null;
  if (gramWeight <= 0 || widthCm <= 0) return null;
  const widthM = widthCm / CM_PER_M;
  const meters = GRAM_PER_KG / (gramWeight * widthM);
  return Math.floor(meters * FLOOR_SCALE) / FLOOR_SCALE;
}
