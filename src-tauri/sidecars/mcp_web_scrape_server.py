"""MCP Web Scrape Server — BeautifulSoup + requests.

Tool: web_scrape
  - url: target URL
  - fields: CSS selector → field name mapping (optional, returns full text if empty)
  - list: CSS selector for repeated items (optional)
  - list_fields: field mapping for each list item (used when list is set)
  - timeout: request timeout in seconds (default 30)
"""

import json
import sys
import os

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from mcp_protocol import McpServer, McpTool, log_stderr
from repl_skin import ReplSkin

skin = ReplSkin("web_scrape", version="1.0.0")


class WebScrapeTool(McpTool):
    def __init__(self):
        super().__init__(
            name="web_scrape",
            description="Scrape structured data from a web page",
            input_schema={
                "type": "object",
                "properties": {
                    "url": {"type": "string", "description": "Target URL to scrape"},
                    "fields": {"type": "object", "description": "CSS selector → field name mapping"},
                    "list": {"type": "string", "description": "CSS selector for repeated items"},
                    "list_fields": {"type": "object", "description": "Field mapping for each list item"},
                    "timeout": {"type": "integer", "default": 30},
                    "headers": {"type": "object", "description": "Custom HTTP headers"},
                },
                "required": ["url"],
            },
        )

    def execute(self, arguments: dict) -> dict:
        import requests
        from bs4 import BeautifulSoup

        url = arguments["url"]
        fields = arguments.get("fields", {})
        list_sel = arguments.get("list")
        list_fields = arguments.get("list_fields", {})
        timeout = arguments.get("timeout", 30)
        headers = arguments.get("headers", {})
        if not headers:
            headers = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36"}

        resp = requests.get(url, headers=headers, timeout=timeout)
        resp.raise_for_status()
        soup = BeautifulSoup(resp.text, "html.parser")

        result = {"url": url, "status": resp.status_code}

        # Extract fields
        if fields:
            extracted = {}
            for css, name in fields.items():
                el = soup.select_one(css)
                if el:
                    # Try getting text, then href, then src
                    extracted[name] = el.get_text(strip=True) or el.get("href") or el.get("src") or ""
                else:
                    extracted[name] = None
            result["fields"] = extracted

        # Extract list items
        if list_sel and list_fields:
            items = []
            for item in soup.select(list_sel):
                entry = {}
                for css, name in list_fields.items():
                    el = item.select_one(css)
                    if el:
                        entry[name] = el.get_text(strip=True) or el.get("href") or el.get("src") or ""
                    else:
                        entry[name] = None
                items.append(entry)
            result["list"] = items
            result["count"] = len(items)

        # If no fields or list specified, return full page text
        if not fields and not list_sel:
            result["text"] = soup.get_text(separator="\n", strip=True)[:10000]
            result["title"] = soup.title.string if soup.title else ""

        return result


def main():
    server = McpServer("wf-web-scrape", "1.0.0")
    server.add_tool(WebScrapeTool())
    server.run()


if __name__ == "__main__":
    main()
