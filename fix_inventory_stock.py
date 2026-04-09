import re

with open("frontend/src/pages/inventory_stock.rs", "r") as f:
    content = f.read()

structs_old = r"""#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct StockResponse {
    pub id: i32,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub quantity: f64,
    pub locked_quantity: f64,
    pub status: String,
    // Emulated tree view children: Rolls
    #[serde(default)]
    pub rolls: Vec<RollResponse>,
}"""

structs_new = r"""#[derive(Clone, PartialEq, Deserialize, Serialize)]
pub struct StockResponse {
    pub id: i32,
    pub product_id: i32,
    pub warehouse_id: i32,
    pub batch_no: String,
    pub color_no: String,
    pub quantity_meters: f64,
    pub quantity_kg: f64,
    pub grade: String,
    // Emulated tree view children: Rolls
    #[serde(default)]
    pub rolls: Vec<RollResponse>,
}

#[derive(Clone, PartialEq, Deserialize)]
pub struct StockFabricListResponse {
    pub stock: Vec<StockResponse>,
    pub total: u64,
}"""

content = content.replace(structs_old, structs_new)

fetch_old = r"""                if let Ok(mut res) = ApiService::get::<Vec<StockResponse>>("/api/v1/erp/inventory-stocks").await {
                    // Inject mock rolls for demonstration of the tree view since the backend doesn't have the roll table yet
                    for (i, stock) in res.iter_mut().enumerate() {
                        if i % 2 == 0 {
                            stock.rolls = vec![
                                RollResponse { roll_no: format!("R{}-01", stock.id), batch_no: format!("B{}", stock.id), length: 120.5, defect_points: 0.0 },
                                RollResponse { roll_no: format!("R{}-02", stock.id), batch_no: format!("B{}", stock.id), length: 118.0, defect_points: 12.5 },
                            ];
                        }
                    }
                    stocks.set(res);
                }"""

fetch_new = r"""                if let Ok(res) = ApiService::get::<StockFabricListResponse>("/api/v1/erp/inventory/stock-fabric?page=1&page_size=100").await {
                    let mut stocks_list = res.stock;
                    
                    // Fetch actual rolls from backend piece API
                    if let Ok(rolls) = ApiService::get::<Vec<RollResponse>>("/api/v1/erp/inventory/pieces").await {
                        for stock in stocks_list.iter_mut() {
                            let mut stock_rolls = Vec::new();
                            for r in &rolls {
                                if r.batch_no == stock.batch_no {
                                    stock_rolls.push(r.clone());
                                }
                            }
                            stock.rolls = stock_rolls;
                        }
                    }
                    
                    stocks.set(stocks_list);
                }"""

content = content.replace(fetch_old, fetch_new)

# also need to update table columns
table_old = r"""                                    <th>{"总数量"}</th>
                                    <th>{"锁定数量"}</th>
                                    <th>{"状态"}</th>"""
table_new = r"""                                    <th>{"批次号"}</th>
                                    <th>{"色号/等级"}</th>
                                    <th>{"库存量(米/公斤)"}</th>"""
content = content.replace(table_old, table_new)

row_old = r"""                                                <td>{format!("{:.2} 卷", stock.quantity)}</td>
                                                <td class="text-[#FF7D00]">{format!("{:.2}", stock.locked_quantity)}</td>
                                                <td><span class="badge-knit">{&stock.status}</span></td>"""
row_new = r"""                                                <td>{&stock.batch_no}</td>
                                                <td>{format!("{} / {}", stock.color_no, stock.grade)}</td>
                                                <td>{format!("{:.1} 米 / {:.1} kg", stock.quantity_meters, stock.quantity_kg)}</td>"""
content = content.replace(row_old, row_new)


with open("frontend/src/pages/inventory_stock.rs", "w") as f:
    f.write(content)

print("Updated inventory_stock.rs")
