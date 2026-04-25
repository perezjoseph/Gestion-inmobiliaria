use yew::prelude::*;

#[derive(Properties, PartialEq, Eq)]
pub struct FileTypeIndicatorProps {
    pub filename: String,
}

fn is_image_file(name: &str) -> bool {
    let ext = name.rsplit('.').next().unwrap_or("");
    ext.eq_ignore_ascii_case("jpg")
        || ext.eq_ignore_ascii_case("jpeg")
        || ext.eq_ignore_ascii_case("png")
        || ext.eq_ignore_ascii_case("pdf")
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
