use serde_json::Value;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

use crate::services::api::BASE_URL;

// ── Props ──────────────────────────────────────────────────────────────

#[derive(Properties, PartialEq)]
pub struct DocumentEditorProps {
    #[prop_or_default]
    pub contenido: Option<Value>,
    #[prop_or(false)]
    pub readonly: bool,
    pub entity_type: AttrValue,
    pub entity_id: AttrValue,
    #[prop_or_default]
    pub tipo_documento: Option<AttrValue>,
    #[prop_or_default]
    pub documento_id: Option<AttrValue>,
    pub on_save: Callback<Value>,
}

// ── Block model ────────────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq)]
struct EditorBlock {
    block_type: String,
    text: String,
    level: Option<u8>,
    ordered: Option<bool>,
    items: Vec<String>,
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
    confidence: Option<f64>,
    source: Option<String>,
}

fn parse_blocks(value: &Value) -> Vec<EditorBlock> {
    let arr = match value {
        Value::Array(a) => a.clone(),
        Value::Object(obj) => obj
            .get("blocks")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        _ => Vec::new(),
    };
    arr.iter().map(parse_single_block).collect()
}

fn parse_single_block(v: &Value) -> EditorBlock {
    let block_type = v
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("paragraph")
        .to_string();
    let text = v
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let level = v.get("level").and_then(Value::as_u64).map(|l| l as u8);
    let ordered = v.get("ordered").and_then(Value::as_bool);
    let items = v
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let headers = v
        .get("headers")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default();
    let rows = v
        .get("rows")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_array)
                .map(|row| {
                    row.iter()
                        .filter_map(Value::as_str)
                        .map(String::from)
                        .collect()
                })
                .collect()
        })
        .unwrap_or_default();
    let confidence = v.get("confidence").and_then(Value::as_f64);
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .map(String::from);

    EditorBlock {
        block_type,
        text,
        level,
        ordered,
        items,
        headers,
        rows,
        confidence,
        source,
    }
}

fn is_ocr_low(block: &EditorBlock) -> bool {
    block.source.as_deref() == Some("ocr") && block.confidence.is_some_and(|c| c < 0.80)
}

fn has_placeholder(text: &str) -> bool {
    text.contains("{{") && text.contains("}}")
}

/// Highlight `{{placeholder}}` spans with the `.gi-editor-placeholder` class.
fn highlight_placeholders(text: &str) -> Html {
    let mut parts: Vec<Html> = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("{{") {
        if start > 0 {
            parts.push(html! { <>{&remaining[..start]}</> });
        }
        let after_open = &remaining[start + 2..];
        if let Some(end) = after_open.find("}}") {
            let placeholder = &after_open[..end];
            parts.push(html! {
                <span class="gi-editor-placeholder">{format!("{{{{{placeholder}}}}}")}</span>
            });
            remaining = &after_open[end + 2..];
        } else {
            parts.push(html! { <>{&remaining[start..]}</> });
            remaining = "";
        }
    }
    if !remaining.is_empty() {
        parts.push(html! { <>{remaining}</> });
    }
    html! { <>{ for parts }</> }
}

// ── Toolbar sub-component ──────────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct EditorToolbarProps {
    readonly: bool,
    on_format: Callback<String>,
    on_save: Callback<()>,
    on_export_pdf: Callback<()>,
    exporting: bool,
}

#[component]
fn EditorToolbar(props: &EditorToolbarProps) -> Html {
    let fmt = |cmd: &'static str| {
        let on_format = props.on_format.clone();
        let cmd = cmd.to_string();
        Callback::from(move |_: MouseEvent| on_format.emit(cmd.clone()))
    };

    let on_save_click = {
        let on_save = props.on_save.clone();
        Callback::from(move |_: MouseEvent| on_save.emit(()))
    };

    let on_export_click = {
        let on_export = props.on_export_pdf.clone();
        Callback::from(move |_: MouseEvent| on_export.emit(()))
    };

    html! {
        <div class="gi-editor-toolbar" role="toolbar" aria-label="Barra de herramientas del editor">
            if !props.readonly {
                <button class="gi-editor-toolbar-btn" onclick={fmt("formatBlock_h1")} aria-label="Encabezado 1" title="Encabezado 1">{"H1"}</button>
                <button class="gi-editor-toolbar-btn" onclick={fmt("formatBlock_h2")} aria-label="Encabezado 2" title="Encabezado 2">{"H2"}</button>
                <div class="gi-editor-toolbar-sep" />
                <button class="gi-editor-toolbar-btn" onclick={fmt("bold")} aria-label="Negrita" title="Negrita">{"B"}</button>
                <button class="gi-editor-toolbar-btn" onclick={fmt("italic")} aria-label="Cursiva" title="Cursiva">{"I"}</button>
                <button class="gi-editor-toolbar-btn" onclick={fmt("underline")} aria-label="Subrayado" title="Subrayado">{"U"}</button>
                <div class="gi-editor-toolbar-sep" />
                <button class="gi-editor-toolbar-btn" onclick={fmt("insertOrderedList")} aria-label="Lista ordenada" title="Lista ordenada">{"OL"}</button>
                <button class="gi-editor-toolbar-btn" onclick={fmt("insertUnorderedList")} aria-label="Lista sin orden" title="Lista sin orden">{"UL"}</button>
                <div class="gi-editor-toolbar-sep" />
                <button class="gi-editor-toolbar-btn" onclick={fmt("insertTable")} aria-label="Insertar tabla" title="Insertar tabla">{"Tabla"}</button>
                <button class="gi-editor-toolbar-btn" onclick={fmt("insertHorizontalRule")} aria-label="Salto de página" title="Salto de página">{"—"}</button>
                <div class="gi-editor-toolbar-sep" />
                <button class="gi-btn gi-btn-primary" style="font-size: var(--text-sm); padding: var(--space-1) var(--space-3);" onclick={on_save_click}>
                    {"Guardar"}
                </button>
            }
            <button
                class="gi-btn gi-btn-ghost"
                style="font-size: var(--text-sm); padding: var(--space-1) var(--space-3);"
                onclick={on_export_click}
                disabled={props.exporting}
            >
                {if props.exporting { "Exportando..." } else { "Exportar PDF" }}
            </button>
        </div>
    }
}

// ── Content renderer (read-only blocks) ────────────────────────────────

#[derive(Properties, PartialEq)]
struct EditorBlocksViewProps {
    blocks: Vec<EditorBlock>,
}

#[component]
fn EditorBlocksView(props: &EditorBlocksViewProps) -> Html {
    html! {
        <>
            { for props.blocks.iter().map(render_block) }
        </>
    }
}

fn render_block(block: &EditorBlock) -> Html {
    let ocr_class = if is_ocr_low(block) {
        "gi-editor-ocr-low".to_string()
    } else {
        String::new()
    };
    let confidence_title = block
        .confidence
        .map(|c| format!("Confianza OCR: {c:.0}%", c = c * 100.0));

    match block.block_type.as_str() {
        "heading" => render_heading(block, ocr_class, confidence_title),
        "list" => render_list(block, ocr_class, confidence_title),
        "table" => render_table(block, ocr_class, confidence_title),
        "page_break" => html! { <hr class="gi-editor-page-break" /> },
        // "paragraph" and any unknown type render as paragraph
        _ => render_paragraph(block, ocr_class, confidence_title),
    }
}

fn render_heading(block: &EditorBlock, ocr_class: String, title: Option<String>) -> Html {
    let content = if has_placeholder(&block.text) {
        highlight_placeholders(&block.text)
    } else {
        html! { <>{&block.text}</> }
    };
    let title_attr = title.unwrap_or_default();
    match block.level.unwrap_or(1) {
        2 => html! { <h2 class={ocr_class} title={title_attr}>{content}</h2> },
        _ => html! { <h1 class={ocr_class} title={title_attr}>{content}</h1> },
    }
}

fn render_paragraph(block: &EditorBlock, ocr_class: String, title: Option<String>) -> Html {
    let content = if has_placeholder(&block.text) {
        highlight_placeholders(&block.text)
    } else {
        html! { <>{&block.text}</> }
    };
    let title_attr = title.unwrap_or_default();
    html! { <p class={ocr_class} title={title_attr}>{content}</p> }
}

fn render_list(block: &EditorBlock, ocr_class: String, title: Option<String>) -> Html {
    let title_attr = title.unwrap_or_default();
    let items = block.items.iter().map(|item| {
        let content = if has_placeholder(item) {
            highlight_placeholders(item)
        } else {
            html! { <>{item.clone()}</> }
        };
        html! { <li>{content}</li> }
    });
    if block.ordered == Some(true) {
        html! { <ol class={ocr_class} title={title_attr}>{ for items }</ol> }
    } else {
        html! { <ul class={ocr_class} title={title_attr}>{ for items }</ul> }
    }
}

fn render_table(block: &EditorBlock, ocr_class: String, title: Option<String>) -> Html {
    let title_attr = title.unwrap_or_default();
    html! {
        <table class={ocr_class} title={title_attr}>
            if !block.headers.is_empty() {
                <thead>
                    <tr>
                        { for block.headers.iter().map(|h| html! { <th>{h}</th> }) }
                    </tr>
                </thead>
            }
            <tbody>
                { for block.rows.iter().map(|row| html! {
                    <tr>
                        { for row.iter().map(|cell| html! { <td>{cell}</td> }) }
                    </tr>
                }) }
            </tbody>
        </table>
    }
}

// ── Serialize contenteditable back to JSON ─────────────────────────────

fn serialize_editor_content() -> Value {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return Value::Array(Vec::new());
    };
    let Some(el) = doc.query_selector(".gi-editor-content").ok().flatten() else {
        return Value::Array(Vec::new());
    };
    let inner = el.inner_html();
    html_to_blocks(&inner)
}

fn html_to_blocks(html_str: &str) -> Value {
    let mut blocks = Vec::new();
    for line in html_str.split('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(block) = parse_html_line(trimmed) {
            blocks.push(block);
        }
    }
    if blocks.is_empty() {
        let clean = strip_tags(html_str);
        if !clean.trim().is_empty() {
            blocks.push(serde_json::json!({
                "type": "paragraph",
                "text": clean.trim()
            }));
        }
    }
    Value::Array(blocks)
}

fn parse_html_line(line: &str) -> Option<Value> {
    let lower = line.to_lowercase();
    if lower.starts_with("<h1") {
        Some(serde_json::json!({
            "type": "heading",
            "level": 1,
            "text": strip_tags(line)
        }))
    } else if lower.starts_with("<h2") {
        Some(serde_json::json!({
            "type": "heading",
            "level": 2,
            "text": strip_tags(line)
        }))
    } else if lower.starts_with("<hr") {
        Some(serde_json::json!({ "type": "page_break" }))
    } else if lower.starts_with("<ol") || lower.starts_with("<ul") {
        let ordered = lower.starts_with("<ol");
        let items = extract_list_items(line);
        Some(serde_json::json!({
            "type": "list",
            "ordered": ordered,
            "items": items
        }))
    } else if lower.starts_with("<table") {
        let (headers, rows) = extract_table(line);
        Some(serde_json::json!({
            "type": "table",
            "headers": headers,
            "rows": rows
        }))
    } else {
        let text = strip_tags(line);
        if text.trim().is_empty() {
            None
        } else {
            Some(serde_json::json!({
                "type": "paragraph",
                "text": text.trim()
            }))
        }
    }
}

fn strip_tags(html_str: &str) -> String {
    let mut result = String::with_capacity(html_str.len());
    let mut in_tag = false;
    for ch in html_str.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => result.push(ch),
            _ => {}
        }
    }
    result
}

fn extract_list_items(html_str: &str) -> Vec<String> {
    let lower = html_str.to_lowercase();
    let mut items = Vec::new();
    let mut search_from = 0;
    while let Some(start) = lower[search_from..].find("<li") {
        let abs_start = search_from + start;
        if let Some(tag_end) = lower[abs_start..].find('>') {
            let content_start = abs_start + tag_end + 1;
            let content_end = lower[content_start..]
                .find("</li")
                .map_or(lower.len(), |i| content_start + i);
            items.push(
                strip_tags(&html_str[content_start..content_end])
                    .trim()
                    .to_string(),
            );
            search_from = content_end;
        } else {
            break;
        }
    }
    items
}

fn extract_table(html_str: &str) -> (Vec<String>, Vec<Vec<String>>) {
    let lower = html_str.to_lowercase();
    let mut headers = Vec::new();
    let mut rows = Vec::new();

    // Extract <th> elements
    let mut search_from = 0;
    while let Some(start) = lower[search_from..].find("<th") {
        let abs_start = search_from + start;
        if let Some(tag_end) = lower[abs_start..].find('>') {
            let content_start = abs_start + tag_end + 1;
            let content_end = lower[content_start..]
                .find("</th")
                .map_or(lower.len(), |i| content_start + i);
            headers.push(
                strip_tags(&html_str[content_start..content_end])
                    .trim()
                    .to_string(),
            );
            search_from = content_end;
        } else {
            break;
        }
    }

    // Extract <tr> rows with <td> cells
    let tbody_start = lower.find("<tbody").unwrap_or(0);
    let mut tr_from = tbody_start;
    while let Some(tr_start) = lower[tr_from..].find("<tr") {
        let abs_tr = tr_from + tr_start;
        let tr_end = lower[abs_tr..]
            .find("</tr")
            .map_or(lower.len(), |i| abs_tr + i);
        let tr_html = &html_str[abs_tr..tr_end];
        let tr_lower = &lower[abs_tr..tr_end];

        if tr_lower.contains("<td") {
            let mut cells = Vec::new();
            let mut cell_from = 0;
            while let Some(td_start) = tr_lower[cell_from..].find("<td") {
                let td_abs = cell_from + td_start;
                if let Some(tag_end) = tr_lower[td_abs..].find('>') {
                    let c_start = td_abs + tag_end + 1;
                    let c_end = tr_lower[c_start..]
                        .find("</td")
                        .map_or(tr_lower.len(), |i| c_start + i);
                    cells.push(strip_tags(&tr_html[c_start..c_end]).trim().to_string());
                    cell_from = c_end;
                } else {
                    break;
                }
            }
            if !cells.is_empty() {
                rows.push(cells);
            }
        }
        tr_from = tr_end + 1;
    }

    (headers, rows)
}

// ── exec_command helper ────────────────────────────────────────────────

fn exec_format_command(cmd: &str) {
    let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Ok(html_doc) = doc.dyn_into::<web_sys::HtmlDocument>() else {
        return;
    };
    if let Some(tag_suffix) = cmd.strip_prefix("formatBlock_") {
        let tag = match tag_suffix {
            "h1" => "h1",
            "h2" => "h2",
            _ => "p",
        };
        let _ = html_doc.exec_command_with_show_ui_and_value("formatBlock", false, tag);
    } else if cmd == "insertTable" {
        let table_html = "<table><thead><tr><th>Columna 1</th><th>Columna 2</th></tr></thead><tbody><tr><td>&nbsp;</td><td>&nbsp;</td></tr></tbody></table>";
        let _ = html_doc.exec_command_with_show_ui_and_value("insertHTML", false, table_html);
    } else {
        let _ = html_doc.exec_command_with_show_ui_and_value(cmd, false, "");
    }
}

// ── PDF export helper ──────────────────────────────────────────────────

fn export_pdf(documento_id: AttrValue, exporting: UseStateHandle<bool>) {
    spawn_local(async move {
        exporting.set(true);
        let url = format!("{BASE_URL}/documentos/{documento_id}/exportar-pdf");
        // Open the PDF URL directly — the browser will handle the download
        if let Some(win) = web_sys::window() {
            let _ = win.open_with_url(&url);
        }
        exporting.set(false);
    });
}

// ── Main DocumentEditor component ──────────────────────────────────────

#[component]
pub fn DocumentEditor(props: &DocumentEditorProps) -> Html {
    let blocks = use_memo(props.contenido.clone(), |contenido| {
        contenido.as_ref().map(parse_blocks).unwrap_or_default()
    });
    let exporting = use_state(|| false);

    let on_format = Callback::from(|cmd: String| {
        exec_format_command(&cmd);
    });

    let on_save_click = {
        let on_save = props.on_save.clone();
        Callback::from(move |()| {
            let content = serialize_editor_content();
            on_save.emit(content);
        })
    };

    let on_export_pdf = {
        let documento_id = props.documento_id.clone();
        let exporting = exporting.clone();
        Callback::from(move |()| {
            if let Some(ref doc_id) = documento_id {
                export_pdf(doc_id.clone(), exporting.clone());
            }
        })
    };

    html! {
        <div class="gi-editor">
            <EditorToolbar
                readonly={props.readonly}
                on_format={on_format}
                on_save={on_save_click}
                on_export_pdf={on_export_pdf}
                exporting={*exporting}
            />
            <EditorContentArea
                blocks={(*blocks).clone()}
                readonly={props.readonly}
            />
        </div>
    }
}

// ── Content area sub-component ─────────────────────────────────────────

#[derive(Properties, PartialEq)]
struct EditorContentAreaProps {
    blocks: Vec<EditorBlock>,
    readonly: bool,
}

#[component]
fn EditorContentArea(props: &EditorContentAreaProps) -> Html {
    let node_ref = use_node_ref();
    let initial_html = blocks_to_html(&props.blocks);
    let readonly = props.readonly;

    {
        let node_ref = node_ref.clone();
        use_effect_with((initial_html, readonly), move |(html, ro)| {
            if !*ro {
                if let Some(el) = node_ref.cast::<web_sys::HtmlElement>() {
                    el.set_inner_html(html);
                }
            }
        });
    }

    if readonly {
        html! {
            <div
                class="gi-editor-content"
                role="textbox"
                aria-multiline="true"
                aria-readonly="true"
                aria-label="Contenido del documento (solo lectura)"
            >
                <EditorBlocksView blocks={props.blocks.clone()} />
            </div>
        }
    } else {
        html! {
            <div
                ref={node_ref}
                class="gi-editor-content"
                contenteditable="true"
                role="textbox"
                aria-multiline="true"
                aria-label="Contenido del documento"
            />
        }
    }
}

fn blocks_to_html(blocks: &[EditorBlock]) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    for block in blocks {
        match block.block_type.as_str() {
            "heading" => {
                let tag = if block.level == Some(2) { "h2" } else { "h1" };
                let cls = if is_ocr_low(block) {
                    " class=\"gi-editor-ocr-low\""
                } else {
                    ""
                };
                let _ = write!(out, "<{tag}{cls}>{}</{tag}>", block.text);
            }
            "paragraph" => {
                let cls = if is_ocr_low(block) {
                    " class=\"gi-editor-ocr-low\""
                } else {
                    ""
                };
                let _ = write!(out, "<p{cls}>{}</p>", block.text);
            }
            "list" => {
                let tag = if block.ordered == Some(true) {
                    "ol"
                } else {
                    "ul"
                };
                let _ = write!(out, "<{tag}>");
                for item in &block.items {
                    let _ = write!(out, "<li>{item}</li>");
                }
                let _ = write!(out, "</{tag}>");
            }
            "table" => {
                out.push_str("<table><thead><tr>");
                for h in &block.headers {
                    let _ = write!(out, "<th>{h}</th>");
                }
                out.push_str("</tr></thead><tbody>");
                for row in &block.rows {
                    out.push_str("<tr>");
                    for cell in row {
                        let _ = write!(out, "<td>{cell}</td>");
                    }
                    out.push_str("</tr>");
                }
                out.push_str("</tbody></table>");
            }
            "page_break" => {
                out.push_str("<hr class=\"gi-editor-page-break\">");
            }
            _ => {
                let _ = write!(out, "<p>{}</p>", block.text);
            }
        }
    }
    out
}
