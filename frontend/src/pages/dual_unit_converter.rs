// 双计量单位转换器页面

use yew::prelude::*;
use crate::models::dual_unit_converter::{
    ConvertUnitResponse, ValidateDualUnitResponse,
};
use crate::services::dual_unit_converter_service::DualUnitConverterService;
use crate::services::crud_service::CrudService;

/// 转换方向枚举
#[derive(Debug, Clone, PartialEq)]
enum ConversionDirection {
    MetersToKg,
    KgToMeters,
}

/// 双计量单位转换器页面组件
#[function_component(DualUnitConverterPage)]
pub fn dual_unit_converter_page() -> Html {
    let direction = use_state(|| ConversionDirection::MetersToKg);
    let value = use_state(|| "100.000".to_string());
    let gram_weight = use_state(|| "180.00".to_string());
    let width_cm = use_state(|| "150.00".to_string());

    let validate_meters = use_state(|| "100.000".to_string());
    let validate_kg = use_state(|| "3.240".to_string());
    let validate_gram_weight = use_state(|| "180.00".to_string());
    let validate_width_cm = use_state(|| "150.00".to_string());
    let validate_tolerance = use_state(|| "0.5".to_string());

    let conversion_result = use_state(|| Option::<ConvertUnitResponse>::None);
    let validation_result = use_state(|| Option::<ValidateDualUnitResponse>::None);
    let error_message = use_state(|| Option::<String>::None);
    let is_loading = use_state(|| false);

    let active_tab = use_state(|| "convert".to_string());

    let on_direction_change = {
        let direction = direction.clone();
        Callback::from(move |new_direction| {
            direction.set(new_direction);
        })
    };

    let on_convert = {
        let direction = direction.clone();
        let value = value.clone();
        let gram_weight = gram_weight.clone();
        let width_cm = width_cm.clone();
        let conversion_result = conversion_result.clone();
        let error_message = error_message.clone();
        let is_loading = is_loading.clone();

        Callback::from(move |_| {
            let direction = direction.clone();
            let value = value.clone();
            let gram_weight = gram_weight.clone();
            let width_cm = width_cm.clone();
            let conversion_result = conversion_result.clone();
            let error_message = error_message.clone();
            let is_loading = is_loading.clone();

            is_loading.set(true);
            error_message.set(None);
            conversion_result.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let result = match *direction {
                    ConversionDirection::MetersToKg => {
                        DualUnitConverterService::meters_to_kg(&value, &gram_weight, &width_cm).await
                    }
                    ConversionDirection::KgToMeters => {
                        DualUnitConverterService::kg_to_meters(&value, &gram_weight, &width_cm).await
                    }
                };

                is_loading.set(false);

                match result {
                    Ok(resp) => {
                        conversion_result.set(Some(resp));
                    }
                    Err(e) => {
                        error_message.set(Some(e));
                    }
                }
            });
        })
    };

    let on_validate = {
        let validate_meters = validate_meters.clone();
        let validate_kg = validate_kg.clone();
        let validate_gram_weight = validate_gram_weight.clone();
        let validate_width_cm = validate_width_cm.clone();
        let validate_tolerance = validate_tolerance.clone();
        let validation_result = validation_result.clone();
        let error_message = error_message.clone();
        let is_loading = is_loading.clone();

        Callback::from(move |_| {
            let validate_meters = validate_meters.clone();
            let validate_kg = validate_kg.clone();
            let validate_gram_weight = validate_gram_weight.clone();
            let validate_width_cm = validate_width_cm.clone();
            let validate_tolerance = validate_tolerance.clone();
            let validation_result = validation_result.clone();
            let error_message = error_message.clone();
            let is_loading = is_loading.clone();

            is_loading.set(true);
            error_message.set(None);
            validation_result.set(None);

            wasm_bindgen_futures::spawn_local(async move {
                let tolerance_str = if validate_tolerance.is_empty() {
                    None
                } else {
                    Some(validate_tolerance.as_str())
                };

                let result = DualUnitConverterService::validate_dual_unit(
                    &validate_meters,
                    &validate_kg,
                    &validate_gram_weight,
                    &validate_width_cm,
                    tolerance_str,
                ).await;

                is_loading.set(false);

                match result {
                    Ok(resp) => {
                        validation_result.set(Some(resp));
                    }
                    Err(e) => {
                        error_message.set(Some(e));
                    }
                }
            });
        })
    };

    let on_tab_change = {
        let active_tab = active_tab.clone();
        Callback::from(move |tab: String| {
            active_tab.set(tab);
        })
    };

    let is_convert_tab = active_tab.as_str() == "convert";

    html! {
        <div class="dual-unit-converter-page">
            <div class="header">
                <h1>{"双计量单位转换器"}</h1>
            </div>

            <div class="tabs">
                <button
                    class={if is_convert_tab { "tab active" } else { "tab" }}
                    onclick={on_tab_change.clone().reform(|_| "convert".to_string())}
                >
                    {"单位换算"}
                </button>
                <button
                    class={if !is_convert_tab { "tab active" } else { "tab" }}
                    onclick={on_tab_change.clone().reform(|_| "validate".to_string())}
                >
                    {"双计量验证"}
                </button>
            </div>

            if is_convert_tab {
                <div class="tab-content">
                    <div class="form-section">
                        <h2>{"单位换算"}</h2>
                        <p class="description">{"在纺织品贸易中，米数和公斤数之间的转换取决于面料的克重和幅宽。"}</p>

                        <div class="direction-selector">
                            <label class="radio-label">
                                <input
                                    type="radio"
                                    name="direction"
                                    checked={*direction == ConversionDirection::MetersToKg}
                                    onchange={on_direction_change.reform(|_| ConversionDirection::MetersToKg)}
                                />
                                {"米数 → 公斤数"}
                            </label>
                            <label class="radio-label">
                                <input
                                    type="radio"
                                    name="direction"
                                    checked={*direction == ConversionDirection::KgToMeters}
                                    onchange={on_direction_change.reform(|_| ConversionDirection::KgToMeters)}
                                />
                                {"公斤数 → 米数"}
                            </label>
                        </div>

                        <div class="form-grid">
                            <div class="form-group">
                                <label for="value">
                                    {if *direction == ConversionDirection::MetersToKg { "米数 (m)" } else { "公斤数 (kg)" }}
                                </label>
                                <input
                                    id="value"
                                    type="text"
                                    value={(*value).clone()}
                                    oninput={let value = value.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        value.set(input.value());
                                    })}
                                    placeholder="请输入数值"
                                />
                            </div>

                            <div class="form-group">
                                <label for="gram-weight">{"克重 (g/m²)"}</label>
                                <input
                                    id="gram-weight"
                                    type="text"
                                    value={(*gram_weight).clone()}
                                    oninput={let gram_weight = gram_weight.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        gram_weight.set(input.value());
                                    })}
                                    placeholder="请输入克重"
                                />
                            </div>

                            <div class="form-group">
                                <label for="width-cm">{"幅宽 (cm)"}</label>
                                <input
                                    id="width-cm"
                                    type="text"
                                    value={(*width_cm).clone()}
                                    oninput={let width_cm = width_cm.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        width_cm.set(input.value());
                                    })}
                                    placeholder="请输入幅宽"
                                />
                            </div>
                        </div>

                        <button
                            class="btn-primary"
                            onclick={on_convert}
                            disabled={*is_loading}
                        >
                            {if *is_loading { "计算中..." } else { "开始换算" }}
                        </button>

                        if let Some(result) = conversion_result.as_ref() {
                            <div class="result-section">
                                <h3>{"换算结果"}</h3>
                                <div class="result-box">
                                    <div class="result-item">
                                        <span class="result-label">{"换算后数值："}</span>
                                        <span class="result-value">{&result.converted_value}</span>
                                        <span class="result-unit">{&result.to_unit}</span>
                                    </div>
                                    <div class="result-item">
                                        <span class="result-label">{"换算率："}</span>
                                        <span class="result-value">{&result.conversion_rate}</span>
                                    </div>
                                    <div class="result-formula">
                                        <span class="result-label">{"换算公式："}</span>
                                        <pre>{&result.formula}</pre>
                                    </div>
                                </div>
                            </div>
                        }
                    </div>
                </div>
            } else {
                <div class="tab-content">
                    <div class="form-section">
                        <h2>{"双计量单位一致性验证"}</h2>
                        <p class="description">{"验证米数和公斤数是否匹配，用于检验面料计量数据的准确性。"}</p>

                        <div class="form-grid">
                            <div class="form-group">
                                <label for="validate-meters">{"米数 (m)"}</label>
                                <input
                                    id="validate-meters"
                                    type="text"
                                    value={(*validate_meters).clone()}
                                    oninput={let validate_meters = validate_meters.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        validate_meters.set(input.value());
                                    })}
                                    placeholder="请输入米数"
                                />
                            </div>

                            <div class="form-group">
                                <label for="validate-kg">{"公斤数 (kg)"}</label>
                                <input
                                    id="validate-kg"
                                    type="text"
                                    value={(*validate_kg).clone()}
                                    oninput={let validate_kg = validate_kg.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        validate_kg.set(input.value());
                                    })}
                                    placeholder="请输入公斤数"
                                />
                            </div>

                            <div class="form-group">
                                <label for="validate-gram-weight">{"克重 (g/m²)"}</label>
                                <input
                                    id="validate-gram-weight"
                                    type="text"
                                    value={(*validate_gram_weight).clone()}
                                    oninput={let validate_gram_weight = validate_gram_weight.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        validate_gram_weight.set(input.value());
                                    })}
                                    placeholder="请输入克重"
                                />
                            </div>

                            <div class="form-group">
                                <label for="validate-width-cm">{"幅宽 (cm)"}</label>
                                <input
                                    id="validate-width-cm"
                                    type="text"
                                    value={(*validate_width_cm).clone()}
                                    oninput={let validate_width_cm = validate_width_cm.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        validate_width_cm.set(input.value());
                                    })}
                                    placeholder="请输入幅宽"
                                />
                            </div>

                            <div class="form-group">
                                <label for="validate-tolerance">{"允许误差率 (%)"}</label>
                                <input
                                    id="validate-tolerance"
                                    type="text"
                                    value={(*validate_tolerance).clone()}
                                    oninput={let validate_tolerance = validate_tolerance.clone(); Callback::from(move |e: InputEvent| {
                                        let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                        validate_tolerance.set(input.value());
                                    })}
                                    placeholder="默认 0.5"
                                />
                            </div>
                        </div>

                        <button
                            class="btn-primary"
                            onclick={on_validate}
                            disabled={*is_loading}
                        >
                            {if *is_loading { "验证中..." } else { "开始验证" }}
                        </button>

                        if let Some(result) = validation_result.as_ref() {
                            <div class="result-section">
                                <h3>{"验证结果"}</h3>
                                <div class="result-box">
                                    <div class="validation-status">
                                        if result.is_valid {
                                            <span class="status-pass">{"验证通过"}</span>
                                        } else {
                                            <span class="status-fail">{"验证失败"}</span>
                                        }
                                    </div>
                                    <div class="result-item">
                                        <span class="result-label">{"计算出的公斤数："}</span>
                                        <span class="result-value">{&result.calculated_kg}</span>
                                        <span class="result-unit">{"kg"}</span>
                                    </div>
                                    <div class="result-item">
                                        <span class="result-label">{"差异值："}</span>
                                        <span class="result-value">{&result.difference}</span>
                                    </div>
                                    <div class="result-item">
                                        <span class="result-label">{"允许的差异值："}</span>
                                        <span class="result-value">{&result.allowed_difference}</span>
                                    </div>
                                    <div class="result-item">
                                        <span class="result-label">{"误差率："}</span>
                                        <span class="result-value">{&result.error_rate}</span>
                                    </div>
                                </div>
                            </div>
                        }
                    </div>
                </div>
            }

            if let Some(error) = error_message.as_ref() {
                <div class="error-message">
                    <strong>{"错误："}</strong> {error}
                </div>
            }
        </div>
    }
}