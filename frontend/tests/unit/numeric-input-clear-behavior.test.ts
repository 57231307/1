/**
 * NumericInput 清空态行为锁（vitest 挂载真实 el-input 实测键入 清空 提交的事件出参）。
 *
 * 钉住的行为（与各页把步进控件换成手写录入后的字段参数一一对应）：
 * 1) 三态可选模型（合同明细允差 number|undefined min 0 max 100 precision 2 optional）
 *    非空初值清空后 update:modelValue 出参为 undefined 不是 0 也不是空串
 * 2) 预留数量（number|undefined min 0.01 precision 2 optional）同上出 undefined
 * 3) 可空 null 模型行为画像 初值即 null 时清空事件被 el-input 判等吞掉 模型保持 null
 *    无事件出参 已键入数字后清空实测回下界而不是 null 与单源件头注释承诺存在偏差
 *    本批四处字段无一消费 null 模型 缺口作为组件文档对行为画像另票处置 这里按实测定型
 * 4) 必填 number 模型（初值 7 min 0.01 precision 2）清空回下界 0.01 而不是 0
 *    与旧步进控件必填模型清空回 min 的语义同源
 * 5) 正常键入路径回归防线 文本提交后按 precision 归一并钳进 max
 *
 * 调用方：vitest（tests/unit，CI vitest job）。
 * 入参：mount NumericInput 传入各字段真实 props。
 * 传给谁：断言 emitted 事件出参 无下游；存什么/存哪里：不落盘、不存储。
 */
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import { ElInput } from 'element-plus';
import NumericInput from '@/components/NumericInput.vue';

function mountNumeric(props: Record<string, unknown>) {
  return mount(NumericInput, {
    props: props as never,
    global: {
      components: { ElInput },
    },
  });
}

/** 键入文本并触发提交（change 走 el-input 原生 change 转发 模拟失焦前回车提交） */
async function typeAndCommit(
  wrapper: ReturnType<typeof mountNumeric>,
  text: string
): Promise<void> {
  const input = wrapper.find('input');
  await input.setValue(text);
  await input.trigger('change');
}

describe('NumericInput 清空与提交行为', () => {
  it('允差三态字段（optional min 0 max 100 precision 2）清空出参 undefined', async () => {
    const wrapper = mountNumeric({
      modelValue: 5.5,
      min: 0,
      max: 100,
      precision: 2,
      optional: true,
    });
    await typeAndCommit(wrapper, '');
    const updates = wrapper.emitted('update:modelValue');
    expect(updates, '清空应触发一次写回').toBeTruthy();
    expect(updates![updates!.length - 1][0], '未填态必须回 undefined').toBeUndefined();
    const changes = wrapper.emitted('change');
    expect(changes![changes!.length - 1][0]).toBeUndefined();
  });

  it('预留数量（optional min 0.01 precision 2）清空出参 undefined', async () => {
    const wrapper = mountNumeric({
      modelValue: 3,
      min: 0.01,
      precision: 2,
      optional: true,
    });
    await typeAndCommit(wrapper, '');
    const updates = wrapper.emitted('update:modelValue');
    expect(updates![updates!.length - 1][0], '未填态不得塌成 0').toBeUndefined();
  });

  it('可空 null 模型行为画像 键入后清空回下界 与组件头注释承诺存在缺口（另票处置）', async () => {
    // 实测两个缺口形态 一 初值直接为 null 时置空被 el-input 判等吞掉 onType 不触发
    // 事件层面无出参 模型保持 null 二 一旦真实键入过数字 清空走的是必填 number
    // 分支 回下界 min 而不是头注释承诺的 null 该承诺仅在提交瞬间模型仍为 null 时成立
    // 本批四处业务字段无一消费 null 模型（允差与预留为 optional 发货数量为 number）
    // 缺口不影响本批验收 作为单源件自身的文档对行为偏差另票处置 这里按实测画像钉住
    const wrapper = mountNumeric({
      modelValue: null,
      min: 0,
      precision: 2,
    });
    await typeAndCommit(wrapper, '12.5');
    const filled = wrapper.emitted('update:modelValue');
    expect(filled![filled!.length - 1][0], '键入路径本身可用').toBe(12.5);
    await typeAndCommit(wrapper, '');
    const updates = wrapper.emitted('update:modelValue');
    expect(updates![updates!.length - 1][0], '已键入后清空实测回下界 0（文档缺口画像）').toBe(0);
  });

  it('必填 number 模型（min 0.01 precision 2）清空回下界 0.01 而不是 0', async () => {
    const wrapper = mountNumeric({
      modelValue: 7,
      min: 0.01,
      precision: 2,
    });
    await typeAndCommit(wrapper, '');
    const updates = wrapper.emitted('update:modelValue');
    expect(updates![updates!.length - 1][0]).toBe(0.01);
  });

  it('正常键入按 precision 归一并钳进 max（允差填 250 钳到 100）', async () => {
    const wrapper = mountNumeric({
      modelValue: undefined,
      min: 0,
      max: 100,
      precision: 2,
      optional: true,
    });
    await typeAndCommit(wrapper, '250');
    const updates = wrapper.emitted('update:modelValue');
    expect(updates![updates!.length - 1][0]).toBe(100);
    await typeAndCommit(wrapper, '12.3456');
    const updates2 = wrapper.emitted('update:modelValue');
    expect(updates2![updates2!.length - 1][0], '按两位小数截断').toBe(12.35);
  });
});
