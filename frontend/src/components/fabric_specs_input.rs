use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct Props {
    #[prop_or_default]
    pub on_weight_calculated: Callback<f64>,
}

#[function_component(FabricSpecsInput)]
pub fn fabric_specs_input(props: &Props) -> Html {
    let meters = use_state(|| 0.0);
    let width_cm = use_state(|| 0.0);
    let weight_gsm = use_state(|| 0.0);
    let shrinkage_pct = use_state(|| 0.0);
    let allowance_pct = use_state(|| 0.0);

    let calculated_kg = {
        let m = *meters;
        let w = *width_cm;
        let g = *weight_gsm;
        let s = *shrinkage_pct;
        let a = *allowance_pct;
        
        // Base calculation: (meters * width(m) * gsm) / 1000 = kg
        // Apply shrinkage (e.g. 5% means we need 5% more fabric)
        // Apply allowance (空差)
        let effective_m = m * (1.0 + s / 100.0) * (1.0 + a / 100.0);
        let kg = (effective_m * (w / 100.0) * g) / 1000.0;
        kg
    };

    {
        let on_calc = props.on_weight_calculated.clone();
        let kg = calculated_kg;
        use_effect_with(kg, move |&val| {
            on_calc.emit(val);
            || ()
        });
    }

    let on_m_change = {
        let state = meters.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f64>() { state.set(val); }
            }
        })
    };

    let on_w_change = {
        let state = width_cm.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f64>() { state.set(val); }
            }
        })
    };

    let on_g_change = {
        let state = weight_gsm.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f64>() { state.set(val); }
            }
        })
    };

    let on_s_change = {
        let state = shrinkage_pct.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f64>() { state.set(val); }
            }
        })
    };

    let on_a_change = {
        let state = allowance_pct.clone();
        Callback::from(move |e: Event| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                if let Ok(val) = input.value().parse::<f64>() { state.set(val); }
            }
        })
    };

    html! {
        <div class="grid grid-cols-1 md:grid-cols-5 gap-4 p-4 bg-slate-50 rounded-md border border-slate-200">
            <div>
                <label class="block text-xs font-medium text-slate-700 mb-1">{"米数 (m)"}</label>
                <input type="number" onchange={on_m_change} class="w-full" placeholder="0.00" />
            </div>
            <div>
                <label class="block text-xs font-medium text-slate-700 mb-1">{"门幅 (cm)"}</label>
                <input type="number" onchange={on_w_change} class="w-full" placeholder="例如: 150" />
            </div>
            <div>
                <label class="block text-xs font-medium text-slate-700 mb-1">{"克重 (g/m²)"}</label>
                <input type="number" onchange={on_g_change} class="w-full" placeholder="例如: 220" />
            </div>
            <div>
                <label class="block text-xs font-medium text-slate-700 mb-1">{"缩率 (%)"}</label>
                <input type="number" onchange={on_s_change} class="w-full" placeholder="0.0" />
            </div>
            <div>
                <label class="block text-xs font-medium text-slate-700 mb-1">{"空差 (%)"}</label>
                <input type="number" onchange={on_a_change} class="w-full" placeholder="0.0" />
            </div>
            <div class="md:col-span-5 flex justify-end items-center mt-2 border-t border-slate-200 pt-2">
                <span class="text-sm text-slate-500 mr-2">{"预估理论重量:"}</span>
                <span class="text-lg font-bold text-indigo-600 font-mono">{format!("{:.2} kg", calculated_kg)}</span>
            </div>
        </div>
    }
}
