import re

with open("/home/root0/桌面/121/1/frontend/src/pages/greige_fabric.rs", "r") as f:
    content = f.read()

# 1. Add StockOut to Msg enum
if "StockOut(i32)" not in content:
    content = content.replace("    OperationSuccess(String),\n}", "    OperationSuccess(String),\n    StockOut(i32),\n}")

# 2. Add StockOut to match block in update()
stock_out_logic = """
            Msg::StockOut(id) => {
                let link = ctx.link().clone();
                spawn_local(async move {
                    match GreigeFabricService::stock_out(id).await {
                        Ok(_) => {
                            link.send_message(Msg::OperationSuccess("出库成功".to_string()));
                            link.send_message(Msg::LoadFabrics);
                        }
                        Err(e) => link.send_message(Msg::LoadError(e)),
                    }
                });
                false
            }"""
match = re.search(r'(\s*Msg::OperationSuccess\(msg\) => \{.*?\n\s*false\n\s*\})', content, re.DOTALL)
if match:
    content = content[:match.end()] + stock_out_logic + content[match.end():]

# 3. Replace the button
content = content.replace(
    """<button class="btn-small btn-warning" onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))}>
                                                {"出库"}
                                            </button>""",
    """<button class="btn-small btn-warning" onclick={ctx.link().callback(move |_| Msg::StockOut(fabric_id))}>
                                                {"出库"}
                                            </button>"""
)

with open("/home/root0/桌面/121/1/frontend/src/pages/greige_fabric.rs", "w") as f:
    f.write(content)
