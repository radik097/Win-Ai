mod vision;
mod executor;
mod inspector;

use mcp_rust_sdk::server::{McpServer, McpService};
use mcp_rust_sdk::types::{CallToolResult, TextContent};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use vision::DesktopCapture;
use executor::HardwareExecutor;
use inspector::UiInspector;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Создаем сервер Jarvis
    let server = McpServer::new("Jarvis-Core")
        .version("0.1.0")
        .tool(
            "get_screen_metadata",
            "Получить список активных окон и их координаты (UI Tree)",
            json!({
                "type": "object",
                "properties": {
                    "max_depth": { "type": "integer", "default": 3 }
                }
            }),
        )
        .tool(
            "capture_screen",
            "Сделать скриншот рабочего стола (возвращает base64)",
            json!({
                "type": "object",
                "properties": {}
            }),
        )
        .tool(
            "execute_click",
            "Выполнить клик мышью по координатам",
            json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer" },
                    "y": { "type": "integer" }
                },
                "required": ["x", "y"]
            }),
        )
        .build();

    // 2. Инициализация системных модулей
    let vision = Arc::new(Mutex::new(DesktopCapture::new()?));
    let executor = Arc::new(HardwareExecutor::new()?);
    let inspector = Arc::new(UiInspector::new()?);

    // 3. Запуск обработчика команд через Stdio
    let service = Arc::new(JarvisHandler { vision, executor, inspector });
    mcp_rust_sdk::stdio::run_server(server, service).await?;

    Ok(())
}

struct JarvisHandler {
    vision: Arc<Mutex<DesktopCapture>>,
    executor: Arc<HardwareExecutor>,
    inspector: Arc<UiInspector>,
}

#[async_trait::async_trait]
impl McpService for JarvisHandler {
    async fn call_tool(&self, tool_name: &str, arguments: serde_json::Value) -> CallToolResult {
        match tool_name {
            "get_screen_metadata" => {
                let max_depth = arguments["max_depth"].as_u64().unwrap_or(3) as usize;
                match self.inspector.get_ui_tree(max_depth) {
                    Ok(tree) => {
                        let json_tree = serde_json::to_string(&tree).unwrap_or_default();
                        CallToolResult::new_success(vec![TextContent::text(json_tree)])
                    }
                    Err(e) => CallToolResult::new_error(format!("UI Inspection failed: {}", e)),
                }
            }
            "capture_screen" => {
                let mut vision = self.vision.lock().await;
                match vision.capture_frame() {
                    Ok(data) => {
                        use image::{ImageBuffer, Rgba, ImageFormat};
                        use std::io::Cursor;
                        use base64::Engine;

                        let (width, height) = vision.get_dimensions();
                        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, data)
                            .unwrap_or_default();
                        
                        let mut png_data = Vec::new();
                        let mut cursor = Cursor::new(&mut png_data);
                        img.write_to(&mut cursor, ImageFormat::Png).ok();

                        let b64 = base64::engine::general_purpose::STANDARD.encode(png_data);
                        CallToolResult::new_success(vec![TextContent::text(format!("data:image/png;base64,{}", b64))])
                    }
                    Err(e) => CallToolResult::new_error(format!("Capture failed: {}", e)),
                }
            }
            "execute_click" => {
                let x = arguments["x"].as_i64().unwrap_or(0) as i32;
                let y = arguments["y"].as_i64().unwrap_or(0) as i32;
                
                // Use smooth move for better realism
                if let Err(e) = self.executor.smooth_move(x, y, 10) {
                    return CallToolResult::new_error(format!("Move failed: {}", e));
                }
                if let Err(e) = self.executor.click(x, y) {
                    return CallToolResult::new_error(format!("Click failed: {}", e));
                }
                
                CallToolResult::new_success(vec![TextContent::text(format!("Clicked at {}, {}", x, y))])
            }
            _ => CallToolResult::new_error(format!("Tool {} not found", tool_name)),
        }
    }
}
