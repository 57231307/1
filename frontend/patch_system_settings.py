import re

with open("src/pages/system_settings.rs", "r") as f:
    content = f.read()

# Add states for toggles
content = content.replace("let active_tab = use_state(|| \"company\".to_string());", "let active_tab = use_state(|| \"company\".to_string());\n    let allow_negative_stock = use_state(|| true);\n    let enable_credit_risk = use_state(|| true);")

# Replace dict buttons
content = content.replace("""<button onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))} class="bg-white border border-gray-300 text-gray-700 px-3 py-1.5 rounded-md text-sm hover:bg-gray-50">
                    {"+ 新增字典"}
                </button>""", "")

content = content.replace("""<a href="javascript:void(0);" onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))} class="text-indigo-600 hover:text-indigo-900 mr-3">{"编辑"}</a>
                                <a href="javascript:void(0);" onclick={Callback::from(|_| gloo_dialogs::alert("功能开发中..."))} class="text-red-600 hover:text-red-900">{"停用"}</a>""", "<span>{\"- \"}</span>")

# Replace toggles
# Allow negative stock toggle
toggle1 = """                    <button onclick={{
                        let allow_negative_stock = allow_negative_stock.clone();
                        Callback::from(move |_| allow_negative_stock.set(!*allow_negative_stock))
                    }} class={format!("{} relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500", if *allow_negative_stock { "bg-indigo-600" } else { "bg-gray-200" })}>
                        <span class={format!("{} pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200", if *allow_negative_stock { "translate-x-5" } else { "translate-x-0" })}></span>
                    </button>"""
content = re.sub(r'<button onclick=\{Callback::from\(\|_\| gloo_dialogs::alert\("风控功能开发中..."\)\)\}.*?</span>\n\s*</button>', toggle1, content, count=1, flags=re.DOTALL)

# Enable credit risk toggle
toggle2 = """                    <button onclick={{
                        let enable_credit_risk = enable_credit_risk.clone();
                        Callback::from(move |_| enable_credit_risk.set(!*enable_credit_risk))
                    }} class={format!("{} relative inline-flex flex-shrink-0 h-6 w-11 border-2 border-transparent rounded-full cursor-pointer transition-colors ease-in-out duration-200 focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-indigo-500", if *enable_credit_risk { "bg-indigo-600" } else { "bg-gray-200" })}>
                        <span class={format!("{} pointer-events-none inline-block h-5 w-5 rounded-full bg-white shadow transform ring-0 transition ease-in-out duration-200", if *enable_credit_risk { "translate-x-5" } else { "translate-x-0" })}></span>
                    </button>"""
content = re.sub(r'<button onclick=\{Callback::from\(\|_\| gloo_dialogs::alert\("风控功能开发中..."\)\)\}.*?</span>\n\s*</button>', toggle2, content, count=1, flags=re.DOTALL)

with open("src/pages/system_settings.rs", "w") as f:
    f.write(content)

