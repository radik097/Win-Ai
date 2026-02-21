import subprocess
import json
import sys
import base64

def send_request(proc, method, params, req_id):
    request = {
        "jsonrpc": "2.0",
        "id": req_id,
        "method": method,
        "params": params
    }
    print(f"Sending: {json.dumps(request)}", file=sys.stderr)
    proc.stdin.write(json.dumps(request) + "\n")
    proc.stdin.flush()

def read_response(proc):
    line = proc.stdout.readline()
    if not line:
        # Check stderr if stdout is empty
        err = proc.stderr.read()
        if err:
            print(f"DEBUG Server Stderr: {err}", file=sys.stderr)
        return None
    print(f"Received: {line.strip()}", file=sys.stderr)
    try:
        return json.loads(line)
    except json.JSONDecodeError:
        return line

def main():
    server_path = r"d:\Rust\Win-Ai\win_mcp\target\release\win_mcp.exe"
    proc = subprocess.Popen(
        [server_path],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=0 # Use unbuffered
    )

    try:
        # 1. Initialize
        send_request(proc, "initialize", {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {"name": "Antigravity", "version": "1.0.0"}
        }, 0)
        
        # Read until we get id 0
        while True:
            resp = read_response(proc)
            if resp and isinstance(resp, dict) and resp.get("id") == 0:
                break
        
        # 2. Call get_screen_metadata
        send_request(proc, "tools/call", {
            "name": "get_screen_metadata",
            "arguments": {"max_depth": 2}
        }, 1)
        
        # Read until we get id 1
        while True:
            resp = read_response(proc)
            if resp and isinstance(resp, dict) and resp.get("id") == 1:
                break
        
        print(json.dumps(resp))

    finally:
        proc.terminate()

if __name__ == "__main__":
    main()
