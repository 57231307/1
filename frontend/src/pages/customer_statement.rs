use crate::components::main_layout::MainLayout;
use crate::components::tracked_print_button::TrackedPrintButton;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
struct StatementItem {
    date: String,
    ref_no: String,
    doc_type: String,
    debit: f64,  // 应收 (借)
    credit: f64, // 已收 (贷)
    balance: f64,
}

#[function_component(CustomerStatementPage)]
pub fn customer_statement_page() -> Html {
    let items = use_state(|| vec![
        StatementItem { date: "2026-04-01".to_string(), ref_no: "-".to_string(), doc_type: "期初结余".to_string(), debit: 0.0, credit: 0.0, balance: 15000.0 },
        StatementItem { date: "2026-04-05".to_string(), ref_no: "SO-20260405-01".to_string(), doc_type: "销售发货".to_string(), debit: 32000.0, credit: 0.0, balance: 47000.0 },
        StatementItem { date: "2026-04-06".to_string(), ref_no: "RCPT-20260406".to_string(), doc_type: "银行收款".to_string(), debit: 0.0, credit: 20000.0, balance: 27000.0 },
        StatementItem { date: "2026-04-08".to_string(), ref_no: "SR-20260408".to_string(), doc_type: "销售退货".to_string(), debit: 0.0, credit: 1500.0, balance: 25500.0 },
    ]);

    html! {
        <MainLayout current_page="客户对账单">
            <div class="card p-4 md:p-8 bg-white max-w-5xl mx-auto print:shadow-none print:border-none">
                // Report Header
                <div class="text-center mb-8 border-b pb-6 border-slate-200">
                    <h1 class="text-2xl font-bold text-slate-900 tracking-wider">{"客户对账单 (STATEMENT OF ACCOUNT)"}</h1>
                    <div class="mt-4 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm text-slate-600 text-left">
                        <div><span class="font-semibold">{"客户名称:"}</span> {" 广州协诚制衣厂"}</div>
                        <div><span class="font-semibold">{"客户编码:"}</span> {" C-GZ-001"}</div>
                        <div><span class="font-semibold">{"对账区间:"}</span> {" 2026-04-01 至 2026-04-30"}</div>
                        <div><span class="font-semibold">{"打印日期:"}</span> {" 2026-04-09"}</div>
                    </div>
                </div>

                // Print controls (Hidden in print mode)
                <div class="flex justify-end mb-4 print:hidden gap-2">
                    <select class="text-sm border-slate-300 rounded-md">
                        <option>{"本月"}</option>
                        <option>{"上月"}</option>
                        <option>{"本季度"}</option>
                    </select>
                    <TrackedPrintButton document_type="CustomerStatement" document_id="C-GZ-001" class="bg-indigo-50 text-indigo-700 border-indigo-200" />
                </div>

                // Statement Table
                <div class="table-responsive overflow-x-auto w-full pb-4 shadow-sm sm:rounded-lg print:shadow-none">
                    <table class="data-table w-full text-left text-sm text-slate-700">
                        <thead class="bg-slate-100 text-slate-800">
                            <tr>
                                <th class="px-4 py-3 font-semibold border-b">{"日期"}</th>
                                <th class="px-4 py-3 font-semibold border-b">{"单据编号"}</th>
                                <th class="px-4 py-3 font-semibold border-b">{"业务类型"}</th>
                                <th class="px-4 py-3 font-semibold text-right border-b">{"应收金额 (借)"}</th>
                                <th class="px-4 py-3 font-semibold text-right border-b">{"已收/退货 (贷)"}</th>
                                <th class="px-4 py-3 font-semibold text-right border-b">{"结余金额"}</th>
                            </tr>
                        </thead>
                        <tbody class="divide-y divide-slate-200">
                            {for items.iter().map(|item| html! {
                                <tr class="hover:bg-slate-50">
                                    <td class="px-4 py-3">{&item.date}</td>
                                    <td class="px-4 py-3 font-mono text-xs">{&item.ref_no}</td>
                                    <td class="px-4 py-3">{&item.doc_type}</td>
                                    <td class="px-4 py-3 text-right numeric-cell font-mono">{if item.debit > 0.0 { format!("¥{:.2}", item.debit) } else { "".to_string() }}</td>
                                    <td class="px-4 py-3 text-right numeric-cell font-mono text-green-600">{if item.credit > 0.0 { format!("¥{:.2}", item.credit) } else { "".to_string() }}</td>
                                    <td class="px-4 py-3 text-right numeric-cell font-mono font-semibold text-indigo-600">{format!("¥{:.2}", item.balance)}</td>
                                </tr>
                            })}
                        </tbody>
                        <tfoot class="bg-slate-50 font-semibold text-slate-900 border-t-2 border-slate-300">
                            <tr>
                                <td colspan="3" class="px-4 py-3 text-right">{"本期合计:"}</td>
                                <td class="px-4 py-3 text-right font-mono">{"¥32,000.00"}</td>
                                <td class="px-4 py-3 text-right font-mono text-green-600">{"¥21,500.00"}</td>
                                <td class="px-4 py-3 text-right font-mono text-indigo-600">{"¥25,500.00"}</td>
                            </tr>
                        </tfoot>
                    </table>
                </div>
                
                // Signatures
                <div class="mt-16 grid grid-cols-2 md:grid-cols-3 gap-8 text-sm text-slate-700">
                    <div>{"制表人: ____________________"}</div>
                    <div>{"财务审核: ____________________"}</div>
                    <div>{"客户确认(盖章): ____________________"}</div>
                </div>
            </div>
        </MainLayout>
    }
}
