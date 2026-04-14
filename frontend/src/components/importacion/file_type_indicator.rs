use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct FileTypeIndicatorProps {
    pub filename: String,
}

fn is_image_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".pdf")
}

#[function_component]
pub fn FileTypeIndicator(props: &FileTypeIndicatorProps) -> Html {
    let (icon, label) = if is_image_file(&props.filename) {
        ("🖼️", "Imagen")
    } else {
        ("📊", "Hoja de cálculo")
    };

    html! {
        <span class="inline-flex items-center gap-1 rounded-full bg-gray-100 px-3 py-1 text-sm text-gray-700">
            <span>{icon}</span>
            <span>{label}</span>
        </span>
    }
}
