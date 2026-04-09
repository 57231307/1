import re

with open("frontend/src/pages/product_list.rs", "r") as f:
    content = f.read()

# 1. Replace mobile mock logic
mobile_old = r"""                                let is_knit = i % 2 == 0; // Mock data for presentation
                                let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                let badge_text = if is_knit { "针织" } else { "梭织" };"""

mobile_new = r"""                                let is_knit = product.product_type.contains("针织");
                                let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                let badge_text = if product.product_type.is_empty() { "未分类" } else { &product.product_type };
                                let stock_qty = product.stock_qty.unwrap_or(0.0);
                                let stock_class = if stock_qty < 100.0 { "text-[#F53F3F] font-bold" } else { "text-[#1D2129] font-medium" };
                                
                                let specs = format!("{} / {} / {}", 
                                    product.fabric_composition.as_deref().unwrap_or("-"),
                                    product.gram_weight.map(|w| format!("{}g", w)).unwrap_or("-".to_string()),
                                    product.width.map(|w| format!("{}cm", w)).unwrap_or("-".to_string())
                                );"""

content = content.replace(mobile_old, mobile_new)

mobile_old_stock = r"""                                            <div class="flex justify-between items-end mt-2">
                                                <div class="text-[12px] text-[#4E5969]">{"库存: "}<span class="text-[#1D2129] font-medium">{"1,250 kg"}</span></div>
                                                <div class="text-[14px] font-bold text-[#F53F3F]">{"¥"}{product.price.as_deref().unwrap_or("0.00")}</div>
                                            </div>"""

mobile_new_stock = r"""                                            <div class="flex justify-between items-end mt-2">
                                                <div class="text-[12px] text-[#4E5969]">
                                                    <div class="truncate w-32">{specs}</div>
                                                    <div>{"库存: "}<span class={stock_class}>{format!("{:.1} {}", stock_qty, product.unit)}</span></div>
                                                </div>
                                                <div class="text-[14px] font-bold text-[#F53F3F]">{"¥"}{product.standard_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "0.00".to_string())}</div>
                                            </div>"""

content = content.replace(mobile_old_stock, mobile_new_stock)


# 2. Replace desktop mock logic
desktop_old = r"""                                                let is_knit = index % 2 == 0; // Mock logic
                                                let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                                let badge_text = if is_knit { "针织" } else { "梭织" };
                                                let stock_qty = if index == 1 { 80 } else { 1250 }; // Mock low stock
                                                let stock_class = if stock_qty < 100 { "text-[#F53F3F] font-bold" } else { "text-[#1D2129]" };"""

desktop_new = r"""                                                let is_knit = product.product_type.contains("针织");
                                                let badge_class = if is_knit { "badge-knit" } else { "badge-woven" };
                                                let badge_text = if product.product_type.is_empty() { "未分类" } else { &product.product_type };
                                                let stock_qty = product.stock_qty.unwrap_or(0.0);
                                                let stock_class = if stock_qty < 100.0 { "text-[#F53F3F] font-bold" } else { "text-[#1D2129]" };
                                                
                                                let specs = format!("{} / {} / {}", 
                                                    product.fabric_composition.as_deref().unwrap_or("-"),
                                                    product.gram_weight.map(|w| format!("{}g", w)).unwrap_or("-".to_string()),
                                                    product.width.map(|w| format!("{}cm", w)).unwrap_or("-".to_string())
                                                );"""

content = content.replace(desktop_old, desktop_new)

desktop_old_stock = r"""                                                        <td><span class={format!("px-2 py-0.5 rounded text-[12px] {}", badge_class)}>{badge_text}</span></td>
                                                        <td class="text-[#4E5969]">{"100%棉 / 180g / 170cm"}</td>
                                                        <td class="text-right text-[#1D2129]">
                                                            <div>{"¥"}{product.price.as_deref().unwrap_or("0.00")}</div>
                                                            <div class="text-[#86909C] text-[12px]">{"¥"}{product.price.as_deref().unwrap_or("0.00")}</div>
                                                        </td>
                                                        <td class={format!("text-right {}", stock_class)}>{format!("{} {}", stock_qty, product.unit)}</td>"""

desktop_new_stock = r"""                                                        <td><span class={format!("px-2 py-0.5 rounded text-[12px] {}", badge_class)}>{badge_text}</span></td>
                                                        <td class="text-[#4E5969] text-[12px]">{specs}</td>
                                                        <td class="text-right text-[#1D2129]">
                                                            <div>{"¥"}{product.standard_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "0.00".to_string())}</div>
                                                            <div class="text-[#86909C] text-[12px]">{"¥"}{product.cost_price.map(|p| format!("{:.2}", p)).unwrap_or_else(|| "0.00".to_string())}</div>
                                                        </td>
                                                        <td class={format!("text-right {}", stock_class)}>{format!("{:.1} {}", stock_qty, product.unit)}</td>"""

content = content.replace(desktop_old_stock, desktop_new_stock)

with open("frontend/src/pages/product_list.rs", "w") as f:
    f.write(content)

print("Product list fixed.")
