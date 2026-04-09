# Dual Unit Converter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the Dual Unit Converter module (frontend UI and backend skeleton) for the textile ERP to manage global unit constants and product-specific conversion rates (m to kg) based on width and weight.

**Architecture:** 
- Backend: Rust + Axum + Sea-ORM. Add new route and controller placeholders for unit constants and product conversion rules.
- Frontend: Rust + Yew. Update the `DualUnitConverter` page to display the global rules and a compact table for product-specific conversion rates.

**Tech Stack:** Rust, Axum, Yew, Sea-ORM

---

### Task 1: Backend Routes and Controller Skeletons

**Files:**
- Create: `/workspace/backend/src/controllers/base/unit_converter.rs`
- Modify: `/workspace/backend/src/routes/mod.rs`
- Modify: `/workspace/backend/src/controllers/mod.rs` (if necessary)
- Modify: `/workspace/backend/src/controllers/base/mod.rs` (if necessary)

- [ ] **Step 1: Create the controller file**
  Create `/workspace/backend/src/controllers/base/unit_converter.rs` with placeholder handlers.
  ```rust
  use axum::{Json, response::IntoResponse};
  use serde::{Deserialize, Serialize};

  #[derive(Serialize)]
  pub struct GlobalUnitConstant {
      pub id: i32,
      pub from_unit: String,
      pub to_unit: String,
      pub ratio: f64,
  }

  #[derive(Serialize)]
  pub struct ProductConversion {
      pub product_id: i32,
      pub product_code: String,
      pub product_name: String,
      pub width_cm: f64,
      pub weight_gsm: f64,
      pub meters_per_kg: f64,
  }

  pub async fn get_global_constants() -> impl IntoResponse {
      let constants = vec![
          GlobalUnitConstant { id: 1, from_unit: "码(yd)".to_string(), to_unit: "米(m)".to_string(), ratio: 0.9144 },
          GlobalUnitConstant { id: 2, from_unit: "磅(lb)".to_string(), to_unit: "公斤(kg)".to_string(), ratio: 0.453592 },
      ];
      Json(constants)
  }

  pub async fn get_product_conversions() -> impl IntoResponse {
      let products = vec![
          ProductConversion {
              product_id: 1,
              product_code: "FAB-001".to_string(),
              product_name: "全棉纯色汗布".to_string(),
              width_cm: 180.0,
              weight_gsm: 200.0,
              meters_per_kg: 2.7778, // 1000 / (1.8 * 200)
          },
          ProductConversion {
              product_id: 2,
              product_code: "FAB-002".to_string(),
              product_name: "涤纶网眼布".to_string(),
              width_cm: 160.0,
              weight_gsm: 130.0,
              meters_per_kg: 4.8077, // 1000 / (1.6 * 130)
          },
      ];
      Json(products)
  }
  ```

- [ ] **Step 2: Expose the module**
  Create or update `/workspace/backend/src/controllers/base/mod.rs`:
  ```rust
  pub mod unit_converter;
  ```

- [ ] **Step 3: Register the routes**
  Update `/workspace/backend/src/routes/mod.rs`. Add the routes to the `api_routes`.
  ```rust
  use crate::controllers::base::unit_converter;

  // Inside create_router function, add:
  let unit_routes = Router::new()
      .route("/constants", get(unit_converter::get_global_constants))
      .route("/products", get(unit_converter::get_product_conversions));

  // Attach to api_routes (e.g., .nest("/base/units", unit_routes))
  ```

- [ ] **Step 4: Check compilation and commit**
  ```bash
  cd /workspace/backend && cargo check
  git add src/controllers/base/ src/routes/mod.rs
  git commit -m "feat(backend): add dual unit converter api placeholders"
  ```

### Task 2: Frontend Data Models & Service

**Files:**
- Create: `/workspace/frontend/src/models/unit_converter.rs`
- Modify: `/workspace/frontend/src/models/mod.rs`
- Create: `/workspace/frontend/src/services/unit_converter.rs`
- Modify: `/workspace/frontend/src/services/mod.rs`

- [ ] **Step 1: Create Models**
  Create `/workspace/frontend/src/models/unit_converter.rs`:
  ```rust
  use serde::{Deserialize, Serialize};

  #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
  pub struct GlobalUnitConstant {
      pub id: i32,
      pub from_unit: String,
      pub to_unit: String,
      pub ratio: f64,
  }

  #[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
  pub struct ProductConversion {
      pub product_id: i32,
      pub product_code: String,
      pub product_name: String,
      pub width_cm: f64,
      pub weight_gsm: f64,
      pub meters_per_kg: f64,
  }
  ```
  Add `pub mod unit_converter;` to `/workspace/frontend/src/models/mod.rs`.

- [ ] **Step 2: Create Service**
  Create `/workspace/frontend/src/services/unit_converter.rs`:
  ```rust
  use crate::models::unit_converter::{GlobalUnitConstant, ProductConversion};
  use crate::services::api::ApiService;
  use anyhow::Result;

  pub struct UnitConverterService;

  impl UnitConverterService {
      pub async fn get_global_constants() -> Result<Vec<GlobalUnitConstant>> {
          ApiService::get("/api/v1/erp/base/units/constants").await
      }

      pub async fn get_product_conversions() -> Result<Vec<ProductConversion>> {
          ApiService::get("/api/v1/erp/base/units/products").await
      }
  }
  ```
  Add `pub mod unit_converter;` to `/workspace/frontend/src/services/mod.rs`.

- [ ] **Step 3: Commit Models & Service**
  ```bash
  cd /workspace/frontend && cargo check --target wasm32-unknown-unknown
  git add src/models/ src/services/
  git commit -m "feat(frontend): add unit converter models and api service"
  ```

### Task 3: Frontend UI Component (Compact Mode)

**Files:**
- Modify: `/workspace/frontend/src/pages/dual_unit_converter.rs`

- [ ] **Step 1: Build the UI Component**
  Replace the contents of `/workspace/frontend/src/pages/dual_unit_converter.rs` with a compact table layout using the models and services.
  ```rust
  use yew::prelude::*;
  use crate::components::main_layout::MainLayout;
  use crate::models::unit_converter::{GlobalUnitConstant, ProductConversion};
  use crate::services::unit_converter::UnitConverterService;
  use wasm_bindgen_futures::spawn_local;

  #[function_component(DualUnitConverterPage)]
  pub fn dual_unit_converter_page() -> Html {
      let constants = use_state(Vec::new);
      let products = use_state(Vec::new);
      let loading = use_state(|| true);

      {
          let constants = constants.clone();
          let products = products.clone();
          let loading = loading.clone();
          use_effect_with((), move |_| {
              spawn_local(async move {
                  if let Ok(c) = UnitConverterService::get_global_constants().await {
                      constants.set(c);
                  }
                  if let Ok(p) = UnitConverterService::get_product_conversions().await {
                      products.set(p);
                  }
                  loading.set(false);
              });
              || ()
          });
      }

      html! {
          <MainLayout current_page={"双单位换算"}>
              <div class="p-4">
                  <h2 class="text-xl font-bold mb-4">{"双单位换算规则中心"}</h2>
                  
                  if *loading {
                      <div>{"数据加载中..."}</div>
                  } else {
                      <div class="mb-8">
                          <h3 class="text-lg font-semibold mb-2">{"全局固定公式 (物理换算)"}</h3>
                          <div class="table-responsive">
                              <table class="data-table w-full">
                                  <thead>
                                      <tr>
                                          <th>{"换算前单位"}</th>
                                          <th>{"换算后单位 (主库存单位)"}</th>
                                          <th class="text-right">{"固定换算系数"}</th>
                                      </tr>
                                  </thead>
                                  <tbody>
                                      {for constants.iter().map(|c| html! {
                                          <tr>
                                              <td>{&c.from_unit}</td>
                                              <td>{&c.to_unit}</td>
                                              <td class="numeric-cell">{format!("{:.4}", c.ratio)}</td>
                                          </tr>
                                      })}
                                  </tbody>
                              </table>
                          </div>
                      </div>

                      <div>
                          <h3 class="text-lg font-semibold mb-2">{"产品级绑定公式 (米 ↔ 公斤)"}</h3>
                          <div class="table-responsive">
                              <table class="data-table w-full">
                                  <thead>
                                      <tr>
                                          <th>{"产品编号"}</th>
                                          <th>{"产品名称"}</th>
                                          <th class="text-right">{"门幅 (cm)"}</th>
                                          <th class="text-right">{"克重 (g/m²)"}</th>
                                          <th class="text-right">{"米/公斤 系数 (1公斤=X米)"}</th>
                                          <th class="text-center">{"操作"}</th>
                                      </tr>
                                  </thead>
                                  <tbody>
                                      {for products.iter().map(|p| html! {
                                          <tr>
                                              <td>{&p.product_code}</td>
                                              <td>{&p.product_name}</td>
                                              <td class="numeric-cell">{format!("{:.1}", p.width_cm)}</td>
                                              <td class="numeric-cell">{format!("{:.1}", p.weight_gsm)}</td>
                                              <td class="numeric-cell font-bold text-blue-600">{format!("{:.4}", p.meters_per_kg)}</td>
                                              <td class="text-center">
                                                  <button class="btn-secondary text-xs px-2 py-1">{"微调"}</button>
                                              </td>
                                          </tr>
                                      })}
                                  </tbody>
                              </table>
                          </div>
                      </div>
                  }
              </div>
          </MainLayout>
      }
  }
  ```

- [ ] **Step 2: Commit Frontend UI**
  ```bash
  cd /workspace/frontend && cargo check --target wasm32-unknown-unknown
  git add src/pages/dual_unit_converter.rs
  git commit -m "feat(frontend): implement compact UI for dual unit converter page"
  ```
````