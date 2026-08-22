use yew::Html;
use api_types::AttributeValue;

#[derive(Debug, Properties, PartialEq)]
pub struct Props {
    pub element: crate::status::Element,
    pub id: String,
}

#[function_component(ElementDisplay)]
pub fn element_display(props: &Props) -> Html {
    html!{
        <div class={if props.element.online {"element element-online"} else { "element element-offline" }}>
            <h2><b class={if props.element.online { "status-online status" } else { "status-offline status" }}>{"⬤"}</b>{"   "}{&props.id}</h2>
            <div class="attributes">{
                props.element.attributes.iter()
                    .map(|(a, b)| (a.clone(), b.clone()))
                    .map(|(id, val)| html!(<AttributeDisplay id={id} value={val}/>))
                    .collect::<Html>()
            }</div>
        </div>
    }
}
#[derive(Debug, Properties, PartialEq)]
pub struct AttributeDisplayProps {
    id: String,
    value: AttributeValue,
}
#[function_component(AttributeDisplay)]
fn display_attribute(props: &AttributeDisplayProps) -> Html {
    let rendered_value = render_attr_value(&props.value);
    html!{
        <div class="attr">{&props.id}{": "}{rendered_value}</div>
    }
}
fn render_attr_value(value: &AttributeValue) -> String {
    match value {
        AttributeValue::Marker => String::new(),
        AttributeValue::Custom(inner) => inner.to_string(),
        AttributeValue::Timestamp(dt) => dt.format("%d.%m.%Y %H:%M:%S%.3f").to_string(),
        AttributeValue::Percentage(val) => format!("{:.2}%", val * 100.0),
        AttributeValue::History(history) => {
            use std::fmt::Write;
            let mut res = String::new();
            for (timestamp, value) in history {
                writeln!(res, "{}: {},",
                         timestamp.format("%d.%m.%Y %H:%M:%S%.3f"),
                         render_attr_value(value)
                ).expect("unable to build history string");
            }
            res
        }
        _ => todo!("unmatched attribute value!")
    }
}