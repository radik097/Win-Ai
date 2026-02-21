mod vision;
mod executor;
mod inspector;
mod gui;

use mcp_rust_sdk::server::{Server, ServerHandler};
use mcp_rust_sdk::transport::stdio::StdioTransport;
use mcp_rust_sdk::types::{Implementation, ClientCapabilities, ServerCapabilities};
use mcp_rust_sdk::error::{Error, ErrorCode};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::Mutex;
use vision::DesktopCapture;
use executor::HardwareExecutor;
use inspector::UiInspector;
use gui::JarvisGui;
use async_trait::async_trait;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 1. Инициализация системных модулей
    let vision_res = DesktopCapture::new();
    let executor_res = HardwareExecutor::new();
    let inspector_res = UiInspector::new();

    let vision_status = if vision_res.is_ok() { "Active" } else { "Vision Init Failed" };
    let executor_status = if executor_res.is_ok() { "Active" } else { "Driver Missing (Vision-Only)" };
    let inspector_status = if inspector_res.is_ok() { "Active" } else { "UIA Init Failed" };

    let vision = Arc::new(Mutex::new(vision_res?));
    let executor = executor_res.ok().map(Arc::new);
    let inspector = Arc::new(inspector_res?);

    // 2. Создаем транспорт и обработчик
    let (transport, _) = StdioTransport::new();
    let handler = Arc::new(JarvisHandler {
        vision: vision.clone(),
        executor,
        inspector,
    });

    // 3. Запуск сервера в фоне
    let server = Server::new(Arc::new(transport), handler);
    tokio::spawn(async move {
        if let Err(e) = server.start().await {
            eprintln!("MCP Server error: {}", e);
        }
    });

    // 4. Запуск GUI в основном потоке
    let gui = JarvisGui::new(vision_status, executor_status, inspector_status)
        .map_err(|e| anyhow::anyhow!("GUI Init failed: {}", e))?;
    gui.run();

    Ok(())
}

struct JarvisHandler {
    vision: Arc<Mutex<DesktopCapture>>,
    executor: Option<Arc<HardwareExecutor>>,
    inspector: Arc<UiInspector>,
}

#[async_trait]
impl ServerHandler for JarvisHandler {
    async fn initialize(
        &self,
        _implementation: Implementation,
        _capabilities: ClientCapabilities,
    ) -> Result<ServerCapabilities, Error> {
        Ok(ServerCapabilities {
            custom: Some(vec![("tools".to_string(), json!({}))].into_iter().collect()),
        })
    }

    async fn shutdown(&self) -> Result<(), Error> {
        Ok(())
    }

    async fn handle_method(
        &self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Error> {
        match method {
            "tools/list" => {
                Ok(json!({
                    "tools": [
                        {
                            "name": "get_screen_metadata",
                            "description": "Получить список активных окон и их координаты (UI Tree)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "max_depth": { "type": "integer", "default": 3 }
                                }
                            }
                        },
                        {
                            "name": "capture_screen",
                            "description": "Сделать скриншот рабочего стола (возвращает base64)",
                            "inputSchema": {
                                "type": "object",
                                "properties": {}
                            }
                        },
                        {
                            "name": "execute_click",
                            "description": "Выполнить клик мышью по координатам",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "x": { "type": "integer" },
                                    "y": { "type": "integer" }
                                },
                                "required": ["x", "y"]
                            }
                        },
                        {
                            "name": "open_url",
                            "description": "Открыть URL в браузере по умолчанию",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "url": { "type": "string" }
                                },
                                "required": ["url"]
                            }
                        },
                        {
                            "name": "launch_app",
                            "description": "Запустить приложение по пути или имени",
                            "inputSchema": {
                                "type": "object",
                                "properties": {
                                    "path": { "type": "string" }
                                },
                                "required": ["path"]
                            }
                        }
                    ]
                }))
            }
            "tools/call" => {
                let params = params.ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing parameters"))?;
                let tool_name = params["name"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing tool name"))?;
                let args = params["arguments"].clone();

                match tool_name {
                    "get_screen_metadata" => {
                        let max_depth = args["max_depth"].as_u64().unwrap_or(3) as usize;
                        let tree = self.inspector.get_ui_tree(max_depth).map_err(|e| Error::protocol(ErrorCode::RequestFailed, e.to_string()))?;
                        Ok(json!({
                            "content": [{"type": "text", "text": serde_json::to_string(&tree).unwrap_or_default()}]
                        }))
                    }
                    "capture_screen" => {
                        let mut vision = self.vision.lock().await;
                        let data = vision.capture_frame().map_err(|e| Error::protocol(ErrorCode::RequestFailed, e.to_string()))?;
                        
                        use image::{ImageBuffer, Rgba, ImageFormat};
                        use std::io::Cursor;
                        use base64::Engine;

                        let (width, height) = vision.get_dimensions();
                        let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(width, height, data)
                            .ok_or_else(|| Error::protocol(ErrorCode::InternalError, "Failed to create image buffer"))?;
                        
                        let mut png_data = Vec::new();
                        let mut cursor = Cursor::new(&mut png_data);
                        img.write_to(&mut cursor, ImageFormat::Png).map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;

                        let b64 = base64::engine::general_purpose::STANDARD.encode(png_data);
                        Ok(json!({
                            "content": [{"type": "text", "text": format!("data:image/png;base64,{}", b64)}]
                        }))
                    }
                    "execute_click" => {
                        let executor = self.executor.as_ref().ok_or_else(|| {
                            Error::protocol(ErrorCode::MethodNotFound, "Hardware executor is not available (driver missing)")
                        })?;
                        
                        let x = args["x"].as_i64().unwrap_or(0) as i32;
                        let y = args["y"].as_i64().unwrap_or(0) as i32;
                        
                        executor.smooth_move(x, y, 10).map_err(|e| Error::protocol(ErrorCode::RequestFailed, e.to_string()))?;
                        executor.click(x, y).map_err(|e| Error::protocol(ErrorCode::RequestFailed, e.to_string()))?;
                        
                        Ok(json!({
                            "content": [{"type": "text", "text": format!("Clicked at {}, {}", x, y)}]
                        }))
                    }
                    "open_url" => {
                        let url = args["url"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing URL"))?;
                        std::process::Command::new("powershell")
                            .arg("-NoProfile")
                            .arg("-Command")
                            .arg(format!("Start-Process '{}'", url))
                            .spawn()
                            .map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        
                        Ok(json!({
                            "content": [{"type": "text", "text": format!("Opened URL: {}", url)}]
                        }))
                    }
                    "launch_app" => {
                        let path = args["path"].as_str().ok_or_else(|| Error::protocol(ErrorCode::InvalidParams, "Missing path"))?;
                        std::process::Command::new("powershell")
                            .arg("-NoProfile")
                            .arg("-Command")
                            .arg(format!("Start-Process '{}'", path))
                            .spawn()
                            .map_err(|e| Error::protocol(ErrorCode::InternalError, e.to_string()))?;
                        
                        Ok(json!({
                            "content": [{"type": "text", "text": format!("Launched application: {}", path)}]
                        }))
                    }
                    _ => Err(Error::protocol(ErrorCode::MethodNotFound, format!("Tool {} not found", tool_name))),
                }
            }
            _ => Err(Error::protocol(ErrorCode::MethodNotFound, format!("Method {} not found", method))),
        }
    }
}
