#!/usr/bin/env python3
"""
Example MCP client for waveform-mcp server.
This demonstrates the proper MCP protocol flow.
"""

import json
import subprocess
import sys
import time
from typing import Dict, Any, Optional

class MCPClient:
    def __init__(self, server_path: str):
        self.server = subprocess.Popen(
            [server_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,  # Line buffered
        )
        self.request_id = 1
    
    def send_request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Dict[str, Any]:
        """Send a JSON-RPC request and return the response."""
        request = {
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": method,
            "params": params or {}
        }
        self.request_id += 1
        
        request_json = json.dumps(request)
        print(f"Sending: {request_json}", file=sys.stderr)
        self.server.stdin.write(request_json + "\n")
        self.server.stdin.flush()
        
        # Read response
        while True:
            line = self.server.stdout.readline()
            if not line:
                raise RuntimeError("Server closed connection")
            
            line = line.strip()
            if line:
                print(f"Received: {line}", file=sys.stderr)
                response = json.loads(line)
                if "id" in response and response["id"] == request["id"]:
                    return response
    
    def send_notification(self, method: str, params: Optional[Dict[str, Any]] = None):
        """Send a JSON-RPC notification."""
        notification = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params or {}
        }
        notification_json = json.dumps(notification)
        print(f"Sending notification: {notification_json}", file=sys.stderr)
        self.server.stdin.write(notification_json + "\n")
        self.server.stdin.flush()
    
    def initialize(self) -> Dict[str, Any]:
        """Initialize the MCP connection."""
        response = self.send_request("initialize", {
            "implementation": {
                "name": "example-client",
                "version": "1.0.0"
            },
            "capabilities": {}
        })
        
        # Send initialized notification
        self.send_notification("initialized", {})
        
        return response
    
    def list_tools(self) -> Dict[str, Any]:
        """List available tools."""
        return self.send_request("tools/list", {})
    
    def call_tool(self, name: str, arguments: Dict[str, Any]) -> Dict[str, Any]:
        """Call a tool."""
        return self.send_request("tools/call", {
            "name": name,
            "arguments": arguments
        })
    
    def shutdown(self):
        """Shutdown the server."""
        try:
            self.send_notification("exit", {})
        except:
            pass
        finally:
            self.server.terminate()
            self.server.wait()

def main():
    # Build the server if needed
    server_path = "./target/debug/waveform-mcp"
    
    print("Starting MCP client...", file=sys.stderr)
    client = MCPClient(server_path)
    
    try:
        # Step 1: Initialize
        print("\n1. Initializing...", file=sys.stderr)
        init_response = client.initialize()
        print(f"Initialization response: {json.dumps(init_response, indent=2)}")
        
        # Step 2: List tools
        print("\n2. Listing tools...", file=sys.stderr)
        tools_response = client.list_tools()
        print(f"Tools response: {json.dumps(tools_response, indent=2)}")
        
        # Step 3: Try to open a waveform (if we had a file)
        # This would fail since we don't have a real file
        print("\n3. Trying to call open_waveform with dummy file...", file=sys.stderr)
        try:
            tool_response = client.call_tool("open_waveform", {
                "file_path": "/tmp/test.vcd",
                "alias": "test_waveform"
            })
            print(f"Tool response: {json.dumps(tool_response, indent=2)}")
        except Exception as e:
            print(f"Tool call failed (expected): {e}", file=sys.stderr)
        
        print("\nClient test completed successfully!", file=sys.stderr)
        
    finally:
        print("\nShutting down...", file=sys.stderr)
        client.shutdown()

if __name__ == "__main__":
    main()