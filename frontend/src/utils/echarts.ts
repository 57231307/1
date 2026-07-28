/**
 * V15 P1-20-4 echarts 按需引入（减少 bundle 体积）
 *
 * 仅注册项目实际使用的图表类型和组件，避免全量引入 echarts。
 * 使用方式：import { echarts } from '@/utils/echarts'
 */
import * as echarts from 'echarts/core';

// 图表类型（按需引入）
import { BarChart, LineChart, PieChart, ScatterChart } from 'echarts/charts';

// 组件（按需引入）
import {
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  DataZoomComponent,
  MarkLineComponent,
  MarkPointComponent,
  ToolboxComponent,
  GraphicComponent,
  CalendarComponent,
} from 'echarts/components';

// 渲染器
import { CanvasRenderer } from 'echarts/renderers';

// 注册按需组件
echarts.use([
  // 图表
  BarChart,
  LineChart,
  PieChart,
  ScatterChart,
  // 组件
  TitleComponent,
  TooltipComponent,
  GridComponent,
  LegendComponent,
  DataZoomComponent,
  MarkLineComponent,
  MarkPointComponent,
  ToolboxComponent,
  GraphicComponent,
  CalendarComponent,
  // 渲染器
  CanvasRenderer,
]);

export { echarts };
export default echarts;
export type { ECharts, EChartsOption } from 'echarts';
export type { BarSeriesOption, LineSeriesOption, PieSeriesOption } from 'echarts/charts';
