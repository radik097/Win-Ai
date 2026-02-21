# Jarvis AI Agent (MCP Server)

Jarvis is a powerful Windows-native AI agent designed to control a computer via a Virtual Display. It operates as a [Model Context Protocol (MCP)](https://modelcontextprotocol.io/) server, connecting directly to LLMs like Claude or GPT.

## 🚀 Features

- **Direct Screen Capture (Vision)**: Uses DXGI Desktop Duplication API for hardware-accelerated, high-frequency screen capture from the RTX 4060.
- **Hardware-Level Input (Executor)**: Utilizes the **Interception Driver** to simulate mouse and keyboard events at the driver level, bypassing traditional UI hooks.
- **UI Tree Inspection**: Deep analysis of Windows applications via **IUIAutomation**, providing structural metadata (IDs, names, bounding boxes) to the AI.
- **MCP Protocol**: Seamless integration with Open WebUI, Claude Desktop, and other MCP-compatible clients.

## 🛠️ Tech Stack

- **Language**: Rust
- **APIs**: windows-rs, DXGI, UI Automation, Interception Driver
- **Protocol**: MCP (JSON-RPC 2.0)
- **Runtime**: Tokio

## 📦 Installation & Setup

### 1. Requirements
- **Windows 10/11**
- **Rust Toolchain** (latest stable)
- **Visual Studio Build Tools** (with Windows SDK)
- **Interception Driver**:
  1. Download [Interception](https://github.com/oblitum/Interception/releases).
  2. Run `install-interception.exe` as Administrator.
  3. **Reboot** your computer.

### 2. Build
Open the **Developer PowerShell for VS** and run:
```powershell
cargo build --release
```

### 3. Usage
Add the following to your MCP client configuration (e.g., Claude Desktop):

```json
{
  "mcpServers": {
    "jarvis": {
      "command": "C:/path/to/Win-Ai/win_mcp/target/release/win_mcp.exe",
      "args": []
    }
  }
}
```

## 🛠️ MCP Tools

| Tool | Description |
| :--- | :--- |
| `get_screen_metadata` | Returns a JSON tree of all visible UI elements. |
| `capture_screen` | Captures a high-quality PNG of the current display. |
| `execute_click` | Performs a hardware-level mouse click at (x, y). |

## ⚠️ Important Notes
- **RTX 4060**: The vision module is optimized for NVIDIA GPU performance.
- **Interception**: Hardware-level control requires the driver to be present. If `interception.dll` is missing or the driver is not installed, the executor will fall back to stubs.
