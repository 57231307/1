import os
import re

css_content = """/* 
 * 面料二批管理系统 - 全局样式核心规范
 * 主色调：#165DFF 
 * 辅助色：#F5F7FA, #E5E6EB, #E8F3FF
 */
:root {
  /* Brand Colors */
  --primary: #165DFF;
  --primary-hover: #0F4CD0;
  
  /* Backgrounds & Borders */
  --bg-main: #F5F7FA;
  --bg-hover: #E8F3FF;
  --border-color: #E5E6EB;
  --card-bg: #FFFFFF;
  
  /* Text Colors */
  --text-main: #1D2129;
  --text-sub: #4E5969;
  --text-weak: #86909C;
  
  /* Functional Colors */
  --success: #00B42A;
  --danger: #F53F3F;
  --warning: #FF7D00;
  --info: #165DFF;
  --disabled: #C9CDD4;
  
  /* Fabric Specific */
  --color-knit: #4CAF50;
  --color-woven: #2196F3;
  
  /* Shadcn Tailwind Mapping (for existing components) */
  --background: 210 20% 98%;
  --foreground: 222 84% 4.9%;
  --card: 0 0% 100%;
  --card-foreground: 222 84% 4.9%;
  --popover: 0 0% 100%;
  --popover-foreground: 222 84% 4.9%;
  --primary-hsl: 222 100% 54%;
  --primary-foreground: 210 40% 98%;
  --secondary: 210 40% 96.1%;
  --secondary-foreground: 222.2 47.4% 11.2%;
  --muted: 210 40% 96.1%;
  --muted-foreground: 215.4 16.3% 46.9%;
  --accent: 214 100% 96%;
  --accent-foreground: 222 100% 54%;
  --destructive: 0 90% 60%;
  --destructive-foreground: 210 40% 98%;
  --border: 210 14% 89%;
  --input: 210 14% 89%;
  --ring: 222 100% 54%;
  
  /* Radii & Spacing */
  --radius: 4px;
  --radius-lg: 8px;
  --spacing-pc: 20px;
  --spacing-mobile: 15px;
}

@media (max-width: 768px) {
  :root {
    --radius: 6px; /* 移动端按钮圆角放大至6px */
  }
}

body {
    background-color: var(--bg-main);
    color: var(--text-main);
    font-family: "Inter", "Alibaba PuHuiTi 2.0", ui-sans-serif, system-ui, -apple-system, sans-serif;
    line-height: 1.6;
    margin: 0;
    padding: 0;
}

h1, h2, h3, h4, h5, h6 {
    color: var(--text-main);
    line-height: 1.5;
    font-weight: bold;
}

h1 { font-size: 18px; }
h2, .card-title { font-size: 16px; }
label, th { font-size: 14px; }
p, td, .btn { font-size: 14px; font-weight: normal; }
.text-xs, .help-text { font-size: 12px; color: var(--text-weak); }

/* Buttons */
.btn, .btn-primary, button {
    border-radius: var(--radius);
    line-height: 1.4;
    transition: all 0.2s ease-in-out;
    padding: 8px 16px;
    cursor: pointer;
    font-size: 14px;
    border: 1px solid transparent;
}

.btn-primary {
    background-color: var(--primary) !important;
    color: #FFF !important;
    box-shadow: 0 1px 2px rgba(0,0,0,0.05);
}

.btn-primary:hover {
    background-color: var(--primary-hover) !important;
    box-shadow: inset 0 2px 4px rgba(0,0,0,0.1);
}

.btn-primary:active {
    box-shadow: inset 0 3px 5px rgba(0,0,0,0.2);
}

.btn-secondary {
    background-color: #FFF !important;
    color: var(--primary) !important;
    border: 1px solid var(--primary) !important;
}

.btn-secondary:hover {
    background-color: var(--bg-hover) !important;
}

.btn-danger {
    background-color: var(--danger) !important;
    color: #FFF !important;
}

.btn-danger:hover {
    background-color: #E03535 !important;
}

.btn-text {
    background-color: transparent !important;
    color: var(--primary) !important;
    border: none !important;
    box-shadow: none !important;
}

.btn-text:hover {
    background-color: var(--bg-hover) !important;
}

.btn:disabled, button:disabled {
    background-color: var(--disabled) !important;
    color: var(--text-weak) !important;
    cursor: not-allowed;
    border: none !important;
    box-shadow: none !important;
}

/* Forms */
input, select, textarea {
    background-color: #FFF;
    border: 1px solid var(--border-color);
    border-radius: var(--radius);
    color: var(--text-main);
    padding: 8px 12px;
    font-size: 14px;
    transition: border-color 0.2s;
    width: 100%;
    box-sizing: border-box;
}

input::placeholder, textarea::placeholder {
    color: var(--text-weak);
}

input:focus, select:focus, textarea:focus {
    border-color: var(--primary);
    box-shadow: none;
    outline: none;
}

/* Cards */
.card, .metric-card {
    background-color: var(--card-bg);
    border-radius: var(--radius);
    box-shadow: 0 2px 8px rgba(0, 0, 0, 0.04);
    border: none;
    padding: 16px;
    margin-bottom: 16px;
}

@media (max-width: 768px) {
    .card, .metric-card {
        padding: 12px;
        margin-bottom: 12px;
    }
}

.card:hover, .metric-card:hover {
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
    transform: translateY(-2px);
}

/* Tables */
.table-responsive {
    overflow-x: auto;
    border-radius: var(--radius);
    background-color: var(--card-bg);
}

.data-table {
    width: 100%;
    border-collapse: collapse;
    border: none;
}

.data-table th {
    background-color: var(--bg-main) !important;
    color: var(--text-main) !important;
    font-weight: bold;
    border-bottom: 1px solid var(--border-color) !important;
    padding: 12px 16px;
    text-align: left;
}

.data-table td {
    background-color: #FFF;
    color: var(--text-main);
    border-bottom: 1px solid var(--border-color) !important;
    padding: 12px 16px;
}

.data-table tr:nth-child(even) td {
    background-color: var(--bg-main); /* Zebra Striping */
}

.data-table tr:hover td {
    background-color: var(--bg-hover) !important;
}

/* Status Badges */
.badge, .status-badge {
    display: inline-block;
    padding: 2px 8px;
    border-radius: 12px;
    font-size: 12px;
    font-weight: 500;
}

.status-success, .badge-success { background-color: #E8FFEA; color: var(--success); }
.status-danger, .badge-danger, .status-error { background-color: #FFECE8; color: var(--danger); }
.status-warning, .badge-warning { background-color: #FFF3E8; color: var(--warning); }
.status-info, .badge-info { background-color: var(--bg-hover); color: var(--info); }
.status-draft { background-color: #F2F3F5; color: var(--text-sub); }

/* Fabric Badges */
.badge-knit { background-color: #E8F5E9; color: var(--color-knit); border: 1px solid var(--color-knit); }
.badge-woven { background-color: #E3F2FD; color: var(--color-woven); border: 1px solid var(--color-woven); }

/* Sidebar */
.sidebar {
    background-color: var(--card-bg);
    border-right: 1px solid var(--border-color);
}

.nav-item {
    color: var(--text-sub);
    transition: all 0.2s;
    border-radius: var(--radius);
}

.nav-item:hover {
    background-color: var(--bg-hover);
    color: var(--primary);
}

.nav-item.active {
    background-color: var(--bg-hover);
    color: var(--primary);
    font-weight: bold;
}

/* Loading & Animation */
.loading-spinner {
    border: 2px solid var(--border-color);
    border-top-color: var(--primary);
    border-radius: 50%;
    animation: spin 1s linear infinite;
}

@keyframes spin {
    to { transform: rotate(360deg); }
}

/* Print Isolation */
@media print {
  body { background: white !important; color: black !important; }
  .sidebar, header, button, .modal-content, .btn, .no-print { display: none !important; }
  .main-content { margin: 0 !important; padding: 0 !important; width: 100% !important; box-shadow: none !important; }
  .data-table th, .data-table td { border: 1px solid #000 !important; color: black !important; }
  .status-badge { border: 1px solid #000 !important; background: transparent !important; color: black !important; }
}

/* Mobile Touches */
@media (max-width: 768px) {
  .btn, button, input, select {
    min-height: 44px; /* Touch target size */
  }
  .data-table td, .data-table th {
    padding: 16px 12px;
  }
  /* Bottom Navigation */
  .mobile-bottom-nav {
      position: fixed;
      bottom: 0;
      left: 0;
      width: 100%;
      height: 56px;
      background: #FFF;
      border-top: 1px solid var(--border-color);
      display: flex;
      justify-content: space-around;
      align-items: center;
      z-index: 100;
  }
  .mobile-nav-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      font-size: 10px;
      color: var(--text-sub);
  }
  .mobile-nav-item.active {
      color: var(--primary);
  }
}
"""

with open('frontend/styles/main.css', 'w', encoding='utf-8') as f:
    f.write(css_content)

print("CSS updated successfully.")
