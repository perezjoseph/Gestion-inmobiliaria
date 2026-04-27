use yew::prelude::*;

const DEFAULT_TASA_DOP_USD: f64 = 58.50;

#[derive(Properties, PartialEq)]
pub struct CurrencyDisplayProps {
    pub monto: f64,
    pub moneda: String,
    #[prop_or(false)]
    pub show_conversion: bool,
    #[prop_or(DEFAULT_TASA_DOP_USD)]
    pub tasa_cambio: f64,
}

#[component]
pub fn CurrencyDisplay(props: &CurrencyDisplayProps) -> Html {
    let formatted = format_dr_currency(props.monto, &props.moneda);

    let conversion = if props.show_conversion && props.tasa_cambio > 0.0 {
        let (converted_amount, converted_currency) = if props.moneda == "USD" {
            (props.monto * props.tasa_cambio, "DOP")
        } else {
            (props.monto / props.tasa_cambio, "USD")
        };
        let converted_formatted = format_dr_currency(converted_amount, converted_currency);
        Some(converted_formatted)
    } else {
        None
    };

    html! {
        <span class="inline-flex items-center gap-1">
            <span>{formatted}</span>
            if let Some(conv) = conversion {
                <span style="font-size: 0.85em; opacity: 0.7;" title="Conversión aproximada">
                    {format!("(≈ {conv})")}
                </span>
            }
        </span>
    }
}

fn format_dr_currency(amount: f64, currency: &str) -> String {
    let abs = amount.abs();
    let integer_part = abs as u64;
    let decimal_part = ((abs - integer_part as f64) * 100.0).round() as u64;

    let int_str = format_thousands(integer_part);
    let sign = if amount < 0.0 { "-" } else { "" };

    format!("{sign}{currency} {int_str},{decimal_part:02}")
}

fn format_thousands(n: u64) -> String {
    let s = n.to_string();
    if s.len() <= 3 {
        return s;
    }

    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push('.');
        }
        result.push(ch);
    }

    result
}
