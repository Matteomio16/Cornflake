'use client';

import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';

interface McpInfo {
  exe_path: string;
  exe_exists: boolean;
  claude_code_command: string;
  desktop_config_path: string;
  desktop_registered: boolean;
}

export function ClaudeConnectSettings() {
  const [info, setInfo] = useState<McpInfo | null>(null);
  const refresh = () => invoke<McpInfo>('mcp_info').then(setInfo).catch(() => undefined);

  useEffect(() => {
    refresh();
  }, []);

  if (!info) return null;

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-1">Use your meetings in Claude</h3>
      <p className="text-sm text-gray-600 mb-4">
        Cornflake includes an MCP server that lets Claude Code and Claude Desktop list, search and read your meetings, and
        file notes into Claude Code project memory after you approve a preview. It runs on this computer and reads the
        Cornflake database directly.
      </p>
      {!info.exe_exists && (
        <p className="mb-3 text-sm text-red-800">The MCP server is missing from this installation ({info.exe_path}).</p>
      )}
      <div className="flex flex-wrap gap-2">
        <button
          disabled={!info.exe_exists}
          onClick={() =>
            invoke<string>('mcp_register_claude_code')
              .then((out) => toast.success(out || 'Added to Claude Code'))
              .catch((e) => toast.error(String(e)))
          }
          className="text-sm font-medium px-3 py-1.5 rounded bg-gray-900 text-white disabled:opacity-50"
        >
          Connect Claude Code
        </button>
        <button
          disabled={!info.exe_exists}
          onClick={() =>
            invoke<string>('mcp_register_claude_desktop')
              .then((backup) => {
                toast.success(backup ? `Added. Previous config backed up to ${backup}` : 'Added to Claude Desktop');
                refresh();
              })
              .catch((e) => toast.error(String(e)))
          }
          className="text-sm font-medium px-3 py-1.5 rounded border border-gray-300 disabled:opacity-50"
        >
          {info.desktop_registered ? 'Reconnect Claude Desktop' : 'Connect Claude Desktop'}
        </button>
      </div>
      <p className="mt-3 text-xs text-gray-500">
        Claude Code runs: <code className="font-mono">{info.claude_code_command}</code>. Claude Desktop: adds a
        &quot;cornflake&quot; entry to {info.desktop_config_path} and keeps a backup. Restart Claude Desktop afterwards.
      </p>
    </div>
  );
}
