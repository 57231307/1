# 组件文档

## 图表组件

### BaseChart

基础图表组件，所有图表组件的基类。

**Props:**
- `option`: ECharts 配置选项
- `height`: 图表高度，默认 `400px`
- `loading`: 是否显示加载状态
- `autoResize`: 是否自动响应容器大小变化
- `alt`: 图表描述（用于无障碍访问）

**Events:**
- `ready`: 图表实例就绪
- `click`: 图表点击事件

### BarChart

柱状图组件，基于 BaseChart 封装。

**Props:**
- `xAxisData`: X 轴数据
- `series`: 系列数据
- `title`: 图表标题
- `horizontal`: 是否水平显示
- `showLabel`: 是否显示标签
- `enableDataZoom`: 是否启用 dataZoom 缩放

### LineChart

折线图组件，基于 BaseChart 封装。

**Props:**
- `xAxisData`: X 轴数据
- `series`: 系列数据
- `title`: 图表标题
- `showArea`: 是否显示面积
- `smooth`: 是否平滑曲线
- `enableDataZoom`: 是否启用 dataZoom 缩放

### PieChart

饼图组件，基于 BaseChart 封装。

**Props:**
- `data`: 饼图数据
- `title`: 图表标题
- `radius`: 饼图半径

## 表格组件

### V2Table

通用表格组件，支持分页、排序、筛选。

**Props:**
- `data`: 表格数据
- `columns`: 列配置
- `loading`: 是否显示加载状态
- `pagination`: 分页配置

## 表单组件

### AdvancedFilter

高级筛选组件，支持多条件组合筛选。

**Props:**
- `fields`: 筛选字段配置
- `model`: 筛选模型

**Events:**
- `search`: 筛选查询
- `reset`: 重置筛选

## 布局组件

### MainLayout

主布局组件，包含侧边栏、头部、内容区。

**Features:**
- 响应式侧边栏
- 移动端抽屉模式
- 多级菜单支持
- 面包屑导航
