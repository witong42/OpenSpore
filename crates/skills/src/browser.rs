pub mod launcher;
pub mod session;
pub mod resolver;

use chromiumoxide::page::Page;
use chromiumoxide::layout::Point;
use futures_util::StreamExt;
use serde::Deserialize;
use tracing::{error, info};
use crate::Skill;
use std::sync::Arc;
use tokio::sync::Mutex;
use async_trait::async_trait;
use self::session::SessionManager;
use self::launcher::BrowserType;

#[derive(Clone, Deserialize, Debug)]
struct ElementMetadata {
    role: String,
    name: String,
}

pub struct BrowserSkill {
    session_manager: SessionManager,
    current_page: Arc<Mutex<Option<Page>>>,
    element_registry: Arc<Mutex<std::collections::HashMap<String, ElementMetadata>>>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum BrowserAction {
    Navigate { url: String },
    Click { selector: String },
    Type { selector: String, text: String },
    Fill { selector: String, text: String },
    Scroll { x: Option<i32>, y: Option<i32> },
    Hover { selector: String },
    Wait { selector: Option<String>, ms: Option<u64> },
    Snapshot,
    Screenshot,
    Url,
    Title,
    Evaluate { expr: String },
    Close,
    Reset,
}

impl BrowserSkill {
    pub fn new(preferred_browser: Option<BrowserType>) -> Self {
        Self {
            session_manager: SessionManager::new(preferred_browser),
            current_page: Arc::new(Mutex::new(None)),
            element_registry: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    async fn get_page(&self) -> anyhow::Result<Page> {
        let mut page_lock = self.current_page.lock().await;
        if let Some(page) = &*page_lock {
            if let Ok(info) = page.evaluate("1+1").await {
                if info.value().and_then(|v| v.as_u64()) == Some(2) {
                    return Ok(page.clone());
                }
            }
        }

        info!("Starting new browser session...");
        let (browser, mut handler) = self.session_manager.get_or_create_session().await?;

        tokio::spawn(async move {
            while let Some(event) = handler.next().await {
                if let Err(e) = event {
                    error!("Browser handler error: {}", e);
                    break;
                }
            }
        });

        let page = browser.new_page("about:blank").await?;
        *page_lock = Some(page.clone());
        Ok(page)
    }

    async fn eval_all_frames(&self, page: &Page, js: &str) -> Vec<(String, Option<serde_json::Value>)> {
        let mut results = Vec::new();
        // Always try main context first
        if let Ok(res) = page.evaluate(js).await {
            results.push(("main".to_string(), res.value().cloned()));
        }

        if let Ok(frames) = page.frames().await {
            for frame_id in frames {
                if let Ok(ctx_id) = page.frame_execution_context(frame_id.clone()).await {
                    if let Some(id) = ctx_id {
                        let eval = chromiumoxide::cdp::js_protocol::runtime::EvaluateParams::builder()
                            .expression(js)
                            .context_id(id)
                            .await_promise(true)
                            .return_by_value(true)
                            .build()
                            .unwrap();

                        if let Ok(res) = page.evaluate(eval).await {
                            results.push((format!("{:?}", frame_id), res.value().cloned()));
                        }
                    }
                }
            }
        }
        results
    }

    async fn resolve_selector_js(&self, selector: &str) -> String {
        let ref_id = if selector.starts_with("[ref=") && selector.ends_with("]") {
            selector.trim_start_matches("[ref=").trim_end_matches("]").to_string()
        } else if selector.starts_with("e") && selector[1..].chars().all(char::is_numeric) {
            selector.to_string()
        } else {
            String::new()
        };

        if !ref_id.is_empty() {
            let registry = self.element_registry.lock().await;
            if let Some(meta) = registry.get(&ref_id) {
                let role_json = serde_json::to_string(&meta.role).unwrap_or_else(|_| "\"\"".to_string());
                let name_json = serde_json::to_string(&meta.name).unwrap_or_else(|_| "\"\"".to_string());
                info!("Using semantic lookup for {}: role={}, name={}", selector, meta.role, meta.name);
                return format!("__findElementBySemantic({}, {}) || __findElement('{}')", role_json, name_json, selector);
            }
        }

        let sel_json = serde_json::to_string(selector).unwrap_or_else(|_| format!("\"{}\"", selector));
        format!("__findElement({})", sel_json)
    }

    async fn generate_snapshot(&self, page: &Page) -> String {
        // Use __scanPage() instead of __generateSnapshot
        let js = format!("(function() {{ \n{}\n return __scanPage(); }})()", resolver::find_element_js());
        let frames_res = self.eval_all_frames(page, &js).await;

        let mut all_snapshot_lines = Vec::new();
        let mut merged_registry = std::collections::HashMap::new();

        for (frame_id, val) in frames_res {
            if let Some(obj) = val.and_then(|v| v.as_object().cloned()) {
                if let Some(snap_str) = obj.get("snapshot").and_then(|s| s.as_str()) {
                    if !snap_str.is_empty() {
                        if frame_id != "main" {
                            all_snapshot_lines.push(format!("--- Frame: {} ---", frame_id));
                        }
                        all_snapshot_lines.push(snap_str.to_string());
                    }
                }
                if let Some(elements) = obj.get("elements").and_then(|e| e.as_object()) {
                    for (k, v) in elements {
                        if let Ok(meta) = serde_json::from_value::<ElementMetadata>(v.clone()) {
                            merged_registry.insert(k.clone(), meta);
                        }
                    }
                }
            }
        }

        // Update global registry
        {
            let mut registry = self.element_registry.lock().await;
            *registry = merged_registry;
        }

        all_snapshot_lines.join("\n")
    }
}

#[async_trait]
impl Skill for BrowserSkill {
    fn name(&self) -> &'static str { "browser" }

    fn description(&self) -> &'static str {
        "Interact with a web browser. Actions: navigate, click, type, fill, scroll, hover, wait, snapshot, screenshot, url, title, close, reset."
    }

    async fn execute(&self, args: &str) -> Result<String, String> {
        let action: BrowserAction = serde_json::from_str(args).map_err(|e| {
            let valid = "navigate, click, type, fill, scroll, hover, wait, snapshot, screenshot, url, title, close, reset";
            format!("Invalid browser action. Valid actions are: {}. Error: {}", valid, e)
        })?;

        match action {
            BrowserAction::Close | BrowserAction::Reset => {
                self.session_manager.remove_session_state();
                let mut page_lock = self.current_page.lock().await;
                *page_lock = None;
                {
                    let mut registry = self.element_registry.lock().await;
                    registry.clear();
                }
                Ok("Browser session reset.".to_string())
            }
            _ => {
                let page = self.get_page().await.map_err(|e| e.to_string())?;
                match action {
                    BrowserAction::Navigate { url } => {
                        page.goto(&url).await.map_err(|e| e.to_string())?;
                        tokio::time::sleep(std::time::Duration::from_millis(2000)).await;
                        // Auto-snapshot to populate registry
                        let _ = self.generate_snapshot(&page).await;
                        Ok(format!("Navigated to {}", url))
                    }
                    BrowserAction::Click { selector } => {
                        let find_js = self.resolve_selector_js(&selector).await;
                        let js = format!(
                            "(function() {{ \n{}\n const el = {}; if (el) {{ el.scrollIntoView({{behavior: 'instant', block: 'center', inline: 'center'}}); const r = el.getBoundingClientRect(); return {{ x: r.left + r.width/2.0, y: r.top + r.height/2.0, js_click: (window.self !== window.top || el.tagName === 'A' || el.tagName === 'BUTTON' || el.closest('a')), tag_name: el.tagName, preview: (el.innerText || el.textContent || '').substring(0, 50).trim() }}; }} return null; }})()",
                            resolver::find_element_js(),
                            find_js
                        );

                        // Implicit Wait Loop (5 seconds)
                        let start = std::time::Instant::now();
                        while start.elapsed() < std::time::Duration::from_secs(5) {
                            let frames_res = self.eval_all_frames(&page, &js).await;
                            for (_frame_id, val) in frames_res {
                                        if let Some(res) = val {
                                            if !res.is_null() {
                                                let x = res.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                                let y = res.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                                let needs_js_click = res.get("js_click").and_then(|v| v.as_bool()).unwrap_or(false);
                                                let tag_name = res.get("tag_name").and_then(|v| v.as_str()).unwrap_or("UNKNOWN").to_string();
                                                let preview = res.get("preview").and_then(|v| v.as_str()).unwrap_or("").to_string();

                                                info!("Found element <{}>: '{}' at ({}, {})", tag_name, preview, x, y);

                                                if needs_js_click {
                                                    info!("Subframe detected or robust click needed for <{}>. Injecting aligned JS click...", tag_name);
                                                    let click_js = format!(
                                                        "(function() {{
{}
 const el = {};
 if (el) {{
    // Prevent popup blocking for links
    const link = el.tagName === 'A' ? el : el.closest('a');
    if (link && link.getAttribute('target') === '_blank') {{
        link.setAttribute('target', '_self');
    }}

    // Dispatch full click event sequence
    const events = ['mousedown', 'mouseup', 'click'];
    events.forEach(type => {{
        const ev = new MouseEvent(type, {{
            bubbles: true,
            cancelable: true,
            view: window,
            buttons: 1
        }});
        el.dispatchEvent(ev);
    }});
    // Also try standard .click() as backup
    if (el.click) el.click();
    return true;
 }}
 return false;
}})()",
                                                        resolver::find_element_js(),
                                                        find_js
                                                    );
                                                    let click_res = self.eval_all_frames(&page, &click_js).await;
                                                    for (_, c_val) in click_res {
                                                        if c_val.and_then(|v| v.as_bool()).unwrap_or(false) {
                                                            match tag_name.as_str() {
                                                                "A" | "BUTTON" | "INPUT" => return Ok(format!("Clicked <{}> '{}' (JS-injected)", tag_name, preview)),
                                                                _ => return Ok(format!("Clicked element <{}> (JS-injected)", tag_name))
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    info!("Main frame target. Using CDP hardware click at ({}, {}) for <{}>", x, y, tag_name);
                                                    let point = Point::new(x, y);
                                                    page.move_mouse(point.clone()).await.map_err(|e| e.to_string())?;
                                                    page.click(point).await.map_err(|e| e.to_string())?;
                                                    return Ok(format!("Clicked <{}> '{}' (CDP hardware click)", tag_name, preview));
                                                }
                                            }
                                        }
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }

                        let snapshot = self.generate_snapshot(&page).await;
                        let truncated_snapshot: String = snapshot.chars().take(8000).collect();
                        let hint = if snapshot.len() > 8000 { "... (truncated)" } else { "" };
                        Err(format!("Element '{}' not found in any frame after 5s waiting.\n\nHere is a partial snapshot of the current page to help you correct the selector:\n\n{}{}", selector, truncated_snapshot, hint))
                    }
                    BrowserAction::Type { selector, text } => {
                        let find_js = self.resolve_selector_js(&selector).await;
                        let js = format!(
                            "(function() {{ \n{}\n const el = {}; if (el) {{ el.focus(); return true; }} return false; }})()",
                            resolver::find_element_js(),
                            find_js
                        );

                        let start = std::time::Instant::now();
                        while start.elapsed() < std::time::Duration::from_secs(5) {
                            let frames_res = self.eval_all_frames(&page, &js).await;
                            for (_, val) in frames_res {
                                if val.and_then(|v| v.as_bool()).unwrap_or(false) {
                                    let body = page.find_element("body").await.map_err(|e| e.to_string())?;
                                    let mut chunk = String::new();
                                    for c in text.chars() {
                                        if c == '\r' || c == '\n' {
                                            if !chunk.is_empty() {
                                                body.type_str(&chunk).await.map_err(|e| e.to_string())?;
                                                chunk.clear();
                                            }
                                            body.press_key("Enter").await.map_err(|e| e.to_string())?;
                                        } else {
                                            chunk.push(c);
                                        }
                                    }
                                    if !chunk.is_empty() {
                                        body.type_str(&chunk).await.map_err(|e| e.to_string())?;
                                    }
                                    return Ok(format!("Typed into '{}'", selector));
                                }
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        Err(format!("Element not found: {}", selector))
                    }
                    BrowserAction::Fill { selector, text } => {
                        let find_js = self.resolve_selector_js(&selector).await;
                        let text_json = serde_json::to_string(&text).unwrap_or_default();
                        let js = format!(
                            "(function() {{ \n{}\n const el = {}; if (el) {{ el.focus(); el.value = {}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); el.dispatchEvent(new Event('change', {{ bubbles: true }})); return true; }} return false; }})()",
                            resolver::find_element_js(),
                            find_js,
                            text_json
                        );

                        let start = std::time::Instant::now();
                        while start.elapsed() < std::time::Duration::from_secs(5) {
                            let frames_res = self.eval_all_frames(&page, &js).await;
                            for (_, val) in frames_res {
                                if val.and_then(|v| v.as_bool()).unwrap_or(false) {
                                    return Ok(format!("Filled '{}'", selector));
                                }
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }

                         // Fallback to simpler fill in Rust
                        if let Ok(el) = page.find_element(selector.clone()).await {
                            let _ = el.call_js_fn(format!("function() {{ this.value = {}; }}", text_json), true).await;
                           return Ok(format!("Filled '{}' (fallback)", selector));
                        }
                        Err(format!("Element not found: {}", selector))
                    }
                    BrowserAction::Scroll { x, y } => {
                        let dx = x.unwrap_or(0);
                        let dy = y.unwrap_or(300);
                        page.evaluate(format!("window.scrollBy({}, {})", dx, dy)).await.map_err(|e| e.to_string())?;
                        Ok(format!("Scrolled by {}, {}", dx, dy))
                    }
                    BrowserAction::Hover { selector } => {
                        let find_js = self.resolve_selector_js(&selector).await;
                        let js = format!(
                            "(function() {{ \n{}\n const el = {}; if (el) {{ const rect = el.getBoundingClientRect(); return {{ x: rect.left + rect.width/2.0, y: rect.top + rect.height/2.0 }}; }} return null; }})()",
                            resolver::find_element_js(),
                            find_js
                        );

                        let start = std::time::Instant::now();
                        while start.elapsed() < std::time::Duration::from_secs(5) {
                            let frames_res = self.eval_all_frames(&page, &js).await;
                            for (_, val) in frames_res {
                                if let Some(coords) = val {
                                    if !coords.is_null() {
                                        let x = coords.get("x").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        let y = coords.get("y").and_then(|v| v.as_f64()).unwrap_or(0.0);
                                        page.move_mouse(Point::new(x, y)).await.map_err(|e| e.to_string())?;
                                        return Ok(format!("Hovered '{}'", selector));
                                    }
                                }
                            }
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        }
                        Err(format!("Element not found: {}", selector))
                    }
                    BrowserAction::Wait { selector, ms } => {
                        let timeout = ms.unwrap_or(5000);
                        if let Some(s) = selector {
                            let find_js = self.resolve_selector_js(&s).await;
                            let js = format!("(function() {{ \n{}\n return !!({}); }})()", resolver::find_element_js(), find_js);
                            let mut found = false;
                            for _ in 0..(timeout/500) {
                                let frames_res = self.eval_all_frames(&page, &js).await;
                                for (_, val) in frames_res {
                                    if val.and_then(|v| v.as_bool()).unwrap_or(false) { found = true; break; }
                                }
                                if found { break; }
                                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            }
                            if found { Ok(format!("Element '{}' appeared.", s)) } else { Err(format!("Timeout waiting for '{}'", s)) }
                        } else {
                            tokio::time::sleep(std::time::Duration::from_millis(timeout)).await;
                            Ok(format!("Waited for {}ms", timeout))
                        }
                    }
                    BrowserAction::Snapshot => {
                        Ok(self.generate_snapshot(&page).await)
                    }
                    BrowserAction::Screenshot => {
                        let screenshot = page.screenshot(chromiumoxide::page::ScreenshotParams::builder().full_page(true).build()).await.map_err(|e| e.to_string())?;
                        let path = openspore_core::path_utils::get_app_root().join("workspace").join("screenshots");
                        std::fs::create_dir_all(&path).ok();
                        let file_name = format!("screenshot_{}.png", chrono::Local::now().format("%Y%m%d_%H%M%S"));
                        let full_path = path.join(&file_name);
                        std::fs::write(&full_path, screenshot).map_err(|e| e.to_string())?;
                        Ok(format!("Screenshot saved to: {}", full_path.display()))
                    }
                    BrowserAction::Url => Ok(page.url().await.map_err(|e| e.to_string())?.unwrap_or_default().to_string()),
                    BrowserAction::Title => {
                        let result = page.evaluate("document.title").await.map_err(|e| e.to_string())?;
                        Ok(result.value().and_then(|v| v.as_str()).unwrap_or("").to_string())
                    }
                    BrowserAction::Evaluate { expr } => {
                         let result = page.evaluate(expr).await.map_err(|e| e.to_string())?;
                         Ok(serde_json::to_string_pretty(&result.value()).unwrap_or_default())
                    }
                    BrowserAction::Close | BrowserAction::Reset => unreachable!(),
                }
            }
        }
    }
}
