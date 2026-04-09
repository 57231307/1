# Textile ERP Compact UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor the Bingxi ERP frontend to a compact UI tailored for the high-density data requirements of the textile industry, and implement a multi-level accordion sidebar with complete industry modules.

**Architecture:** Yew (Rust WASM) with pure CSS styling. State management for sidebar using Yew hooks (`use_state`).

**Tech Stack:** Rust, Yew, CSS3

---

### Task 1: Introduce Compact CSS Variables & Table Styles

**Files:**
- Modify: `/workspace/frontend/styles/main.css`

- [ ] **Step 1: Modify global CSS variables**
  Open `/workspace/frontend/styles/main.css` and update the `:root` variables to define the compact mode. Add the following inside `:root`:
  ```css
  --sidebar-width: 200px;
  --spacing-compact: 4px 8px;
  --font-size-base: 13px;
  --font-size-small: 12px;
  --table-row-height: 32px;
  ```

- [ ] **Step 2: Update typography and table styles**
  In the same file, update the body font-size to use the new variable. Locate the `body {` block and update `font-size`. Add `.numeric-cell` class for tabular numbers.
  ```css
  body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    background-color: var(--bg-color);
    color: var(--text-color);
    font-size: var(--font-size-base); /* Updated to use 13px */
    line-height: 1.5;
  }

  .numeric-cell {
    font-variant-numeric: tabular-nums;
    text-align: right;
  }
  ```

- [ ] **Step 3: Make data tables compact**
  Locate the `.data-table th, .data-table td` block and update the padding and height.
  ```css
  .data-table th, .data-table td {
    padding: var(--spacing-compact); /* Updated to 4px 8px */
    height: var(--table-row-height);  /* Updated to 32px */
    border-bottom: 1px solid var(--border-color);
    text-align: left;
    font-size: var(--font-size-base);
  }
  ```

- [ ] **Step 4: Commit CSS changes**
  ```bash
  git add frontend/styles/main.css
  git commit -m "feat(ui): introduce compact css variables and tabular table styles for textile ERP"
  ```

### Task 2: Add Multi-level Sidebar Styles

**Files:**
- Modify: `/workspace/frontend/styles/main.css`

- [ ] **Step 1: Add Accordion Menu Styles**
  Add styles to support expanding and collapsing sub-menus in the sidebar. Append this to the end of the navigation section in `main.css`.
  ```css
  /* Multi-level Sidebar Styles */
  .nav-group {
    margin-bottom: 4px;
  }
  
  .nav-group-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 8px 16px;
    color: var(--text-secondary);
    font-size: var(--font-size-base);
    font-weight: 600;
    cursor: pointer;
    transition: background-color 0.2s, color 0.2s;
    border-radius: 6px;
    margin: 0 8px;
  }
  
  .nav-group-header:hover {
    background-color: var(--bg-color);
    color: var(--primary-color);
  }
  
  .nav-group-icon {
    font-size: 10px;
    transition: transform 0.3s ease;
  }
  
  .nav-group-icon.open {
    transform: rotate(90deg);
  }
  
  .nav-sub-items {
    max-height: 0;
    overflow: hidden;
    transition: max-height 0.3s ease-out;
  }
  
  .nav-sub-items.open {
    max-height: 500px; /* Large enough to show all items */
  }
  
  .nav-sub-items .nav-item {
    padding-left: 40px; /* Indent sub-items */
    font-size: var(--font-size-small); /* Use smaller font for sub-items */
  }
  ```

- [ ] **Step 2: Commit Sidebar Styles**
  ```bash
  git add frontend/styles/main.css
  git commit -m "style(nav): add accordion multi-level sidebar styles"
  ```

### Task 3: Refactor Navigation Component to Accordion with Textile Modules

**Files:**
- Modify: `/workspace/frontend/src/components/navigation.rs`

- [ ] **Step 1: Define Menu Structure Data Model**
  Open `/workspace/frontend/src/components/navigation.rs`. Define a structured way to hold the menu data inside the `Navigation` component. Add necessary imports: `use yew::prelude::*;`, `use yew_router::prelude::*;`, `use crate::app::Route;`.
  Replace the existing `Navigation` component implementation. First, define the state hook for the open group.
  ```rust
  use yew::prelude::*;
  use yew_router::prelude::*;
  use crate::app::Route;

  #[derive(Properties, PartialEq)]
  pub struct Props {
      pub current_page: String,
  }

  struct MenuGroup {
      title: &'static str,
      icon: &'static str,
      items: Vec<MenuItem>,
  }

  struct MenuItem {
      name: &'static str,
      route: Route,
  }
  ```

- [ ] **Step 2: Build the Textile ERP Menu Tree**
  Inside the `navigation` function, define the menu structure.
  ```rust
  #[function_component(Navigation)]
  pub fn navigation(props: &Props) -> Html {
      let open_group = use_state(|| String::from(""));
      
      let toggle_group = {
          let open_group = open_group.clone();
          Callback::from(move |group_name: String| {
              if *open_group == group_name {
                  open_group.set(String::from("")); // Close if already open
              } else {
                  open_group.set(group_name); // Open new group
              }
          })
      };

      let menu_groups = vec![
          MenuGroup {
              title: "工作台", icon: "📊",
              items: vec![MenuItem { name: "仪表板", route: Route::Dashboard }],
          },
          MenuGroup {
              title: "基础数据", icon: "📁",
              items: vec![
                  MenuItem { name: "产品管理", route: Route::ProductList },
                  MenuItem { name: "产品分类", route: Route::ProductCategory },
                  MenuItem { name: "仓库管理", route: Route::WarehouseList },
                  MenuItem { name: "部门管理", route: Route::DepartmentList },
                  MenuItem { name: "角色管理", route: Route::RoleList },
                  MenuItem { name: "用户管理", route: Route::UserList },
                  MenuItem { name: "供应商", route: Route::Supplier },
                  MenuItem { name: "客户管理", route: Route::Customer },
                  MenuItem { name: "双单位换算", route: Route::DualUnitConverter },
              ],
          },
          MenuGroup {
              title: "销售与CRM", icon: "🤝",
              items: vec![
                  MenuItem { name: "销售订单", route: Route::SalesOrder },
                  MenuItem { name: "面料订单", route: Route::SalesOrder }, // Fallback to SalesOrder for now
                  MenuItem { name: "销售合同", route: Route::SalesContract },
                  MenuItem { name: "销售退货", route: Route::SalesReturn },
                  MenuItem { name: "销售价格", route: Route::SalesPrice },
                  MenuItem { name: "客户信用", route: Route::CustomerCredit },
                  MenuItem { name: "销售分析", route: Route::SalesAnalysis },
                  MenuItem { name: "CRM线索", route: Route::CrmLead },
                  MenuItem { name: "CRM商机", route: Route::CrmOpportunity },
              ],
          },
          MenuGroup {
              title: "生产管理", icon: "🏭",
              items: vec![
                  MenuItem { name: "坯布管理", route: Route::GreigeFabric },
                  MenuItem { name: "染化料配方", route: Route::DyeRecipe },
                  MenuItem { name: "染缸批次", route: Route::DyeBatch },
                  MenuItem { name: "面料批次", route: Route::Batch },
                  MenuItem { name: "质检管理", route: Route::QualityInspection },
              ],
          },
          MenuGroup {
              title: "库存管理", icon: "📦",
              items: vec![
                  MenuItem { name: "库存查询", route: Route::InventoryStock },
                  MenuItem { name: "库存盘点", route: Route::InventoryCount },
                  MenuItem { name: "库存调拨", route: Route::InventoryTransfer },
                  MenuItem { name: "库存调整", route: Route::InventoryAdjustment },
              ],
          },
          MenuGroup {
              title: "采购管理", icon: "🛒",
              items: vec![
                  MenuItem { name: "采购订单", route: Route::PurchaseOrder },
                  MenuItem { name: "采购合同", route: Route::PurchaseContract },
                  MenuItem { name: "采购收货", route: Route::PurchaseReceipt },
                  MenuItem { name: "采购退货", route: Route::PurchaseReturn },
                  MenuItem { name: "采购价格", route: Route::PurchasePrice },
                  MenuItem { name: "入厂检验", route: Route::PurchaseInspection },
                  MenuItem { name: "供应商评价", route: Route::SupplierEvaluation },
              ],
          },
          MenuGroup {
              title: "财务与成本", icon: "💰",
              items: vec![
                  MenuItem { name: "应收账款", route: Route::ArInvoice },
                  MenuItem { name: "应付账款", route: Route::ApInvoice },
                  MenuItem { name: "收款核销", route: Route::ApVerification }, // Fallback placeholder
                  MenuItem { name: "付款核销", route: Route::ApVerification },
                  MenuItem { name: "资金管理", route: Route::FundManagement },
                  MenuItem { name: "财务分析", route: Route::FinancialAnalysis },
                  MenuItem { name: "凭证管理", route: Route::Voucher },
                  MenuItem { name: "预算管理", route: Route::BudgetManagement },
                  MenuItem { name: "成本收集", route: Route::CostCollection },
                  MenuItem { name: "辅助核算", route: Route::AssistAccounting },
                  MenuItem { name: "业务追溯", route: Route::BusinessTrace },
                  MenuItem { name: "固定资产", route: Route::FixedAsset },
              ],
          },
      ];
  ```

- [ ] **Step 3: Render the Accordion HTML**
  Replace the return block of `navigation` with the new accordion logic.
  ```rust
      html! {
          <nav class="sidebar">
              <div class="sidebar-header">
                  <div class="logo">{"Bingxi ERP"}</div>
              </div>
              <div class="sidebar-menu">
                  {
                      for menu_groups.into_iter().map(|group| {
                          let group_title = group.title.to_string();
                          let is_open = *open_group == group_title;
                          let icon_class = if is_open { "nav-group-icon open" } else { "nav-group-icon" };
                          let sub_items_class = if is_open { "nav-sub-items open" } else { "nav-sub-items" };
                          
                          let on_header_click = {
                              let toggle_group = toggle_group.clone();
                              let group_title_clone = group_title.clone();
                              Callback::from(move |_| toggle_group.emit(group_title_clone.clone()))
                          };

                          html! {
                              <div class="nav-group">
                                  <div class="nav-group-header" onclick={on_header_click}>
                                      <span>
                                          <span class="icon">{group.icon}</span>
                                          {group.title}
                                      </span>
                                      <span class={icon_class}>{"▶"}</span>
                                  </div>
                                  <div class={sub_items_class}>
                                      {
                                          for group.items.into_iter().map(|item| {
                                              let is_active = props.current_page == item.name;
                                              let class = if is_active { "nav-item active" } else { "nav-item" };
                                              html! {
                                                  <Link<Route> to={item.route} classes={class}>
                                                      {item.name}
                                                  </Link<Route>>
                                              }
                                          })
                                      }
                                  </div>
                              </div>
                          }
                      })
                  }
              </div>
          </nav>
      }
  }
  ```

- [ ] **Step 4: Commit Navigation changes**
  ```bash
  cargo check --target wasm32-unknown-unknown
  git add frontend/src/components/navigation.rs
  git commit -m "feat(ui): implement multi-level accordion sidebar with full textile ERP modules"
  ```

### Task 4: Fix Sidebar Width in Layout

**Files:**
- Modify: `/workspace/frontend/styles/main.css`

- [ ] **Step 1: Adjust sidebar width**
  Ensure `.sidebar` and `.main-content` respect `--sidebar-width`. Open `main.css`.
  ```css
  .sidebar {
    width: var(--sidebar-width); /* Changed from fixed 250px */
    background-color: var(--surface-color);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    height: 100vh;
    position: fixed;
    left: 0;
    top: 0;
    z-index: 100;
  }

  .main-content {
    flex: 1;
    margin-left: var(--sidebar-width); /* Changed from 250px */
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }
  ```

- [ ] **Step 2: Commit Layout fixes**
  ```bash
  git add frontend/styles/main.css
  git commit -m "style(ui): apply compact sidebar width to main layout"
  ```