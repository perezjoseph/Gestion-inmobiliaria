#!/usr/bin/env node
/**
 * Kiro CLI Custom Agent Configuration Validator
 *
 * Validates a kiro-cli agent JSON file against the documented schema.
 * Bundled with the harness-engineering skill for use during development.
 *
 * Usage: node scripts/validate-kiro-agent.js <path-to-agent.json> [--format json|text]
 *
 * Exit codes:
 *   0 — Valid configuration
 *   1 — Validation errors found
 *   2 — Usage error (missing argument, file not found, bad flags)
 */

const fs = require("fs");
const path = require("path");

// --- Constants ---

const VALID_HOOK_TRIGGERS = [
    "agentSpawn",
    "userPromptSubmit",
    "preToolUse",
    "postToolUse",
    "stop",
];

// Built-in tool config names from kiro.dev/docs/cli/reference/built-in-tools.
// Use the config name here (not the internal alias like fs_write/execute_bash,
// which belong in hook matchers). Aliases are accepted too for resilience.
const BUILTIN_TOOLS = [
    "read", "write", "shell", "aws", "grep", "glob",
    "subagent", "web_search", "web_fetch", "code", "introspect",
    "tool_search", "delegate", "knowledge", "thinking", "todo",
    "goal", "session", "report",
    // Internal aliases (accepted, though config name is preferred)
    "fs_read", "fs_write", "execute_bash", "execute_cmd", "use_aws", "use_subagent",
];

const FILE_URI_PATTERN = /^file:\/\//;
const SKILL_URI_PATTERN = /^skill:\/\//;

// --- Help ---

function printHelp() {
    console.log(`Usage: node scripts/validate-kiro-agent.js <path-to-agent.json> [OPTIONS]

Validate a kiro-cli custom agent JSON configuration against the documented schema.

Options:
  --format FORMAT    Output format: text (default) or json
  --help             Show this help message

Exit codes:
  0  Valid configuration
  1  Validation errors found
  2  Usage error (missing argument, file not found)

Examples:
  node scripts/validate-kiro-agent.js .kiro/agents/autofix.json
  node scripts/validate-kiro-agent.js .kiro/agents/autofix.json --format json`);
}

// --- Validation ---

function validate(configPath) {
    const errors = [];
    const configDir = path.dirname(path.resolve(configPath));

    let raw;
    try {
        raw = fs.readFileSync(configPath, "utf-8");
    } catch (e) {
        return [{ field: "(file)", expected: "readable JSON file", actual: e.message }];
    }

    let config;
    try {
        config = JSON.parse(raw);
    } catch (e) {
        return [{ field: "(file)", expected: "valid JSON", actual: e.message }];
    }

    if (typeof config !== "object" || config === null || Array.isArray(config)) {
        return [{ field: "(root)", expected: "JSON object", actual: typeof config }];
    }

    // name
    if (config.name !== undefined) {
        if (typeof config.name !== "string" || config.name.trim() === "") {
            errors.push({ field: "name", expected: "non-empty string", actual: JSON.stringify(config.name) });
        }
    }

    // description
    if (config.description !== undefined && typeof config.description !== "string") {
        errors.push({ field: "description", expected: "string", actual: typeof config.description });
    }

    // prompt
    if (config.prompt !== undefined) {
        if (typeof config.prompt !== "string") {
            errors.push({ field: "prompt", expected: "string or file:// URI", actual: typeof config.prompt });
        } else if (FILE_URI_PATTERN.test(config.prompt)) {
            const promptPath = config.prompt.replace(FILE_URI_PATTERN, "");
            const resolved = path.resolve(configDir, promptPath);
            if (!fs.existsSync(resolved)) {
                errors.push({ field: "prompt", expected: `file exists at ${resolved}`, actual: "file not found" });
            }
        }
    }

    // model
    if (config.model !== undefined) {
        if (typeof config.model !== "string" || config.model.trim() === "") {
            errors.push({ field: "model", expected: "non-empty string", actual: JSON.stringify(config.model) });
        }
    }

    // tools
    if (config.tools !== undefined) {
        if (!Array.isArray(config.tools)) {
            errors.push({ field: "tools", expected: "array", actual: typeof config.tools });
        } else {
            config.tools.forEach((tool, i) => {
                if (typeof tool !== "string") {
                    errors.push({ field: `tools[${i}]`, expected: "string", actual: typeof tool });
                } else if (tool !== "*" && tool !== "@builtin" && !tool.startsWith("@") && !BUILTIN_TOOLS.includes(tool)) {
                    errors.push({ field: `tools[${i}]`, expected: "valid tool reference (*, @builtin, @server, @server/tool, or built-in name)", actual: tool });
                }
            });
        }
    }

    // allowedTools
    if (config.allowedTools !== undefined) {
        if (!Array.isArray(config.allowedTools)) {
            errors.push({ field: "allowedTools", expected: "array", actual: typeof config.allowedTools });
        } else {
            config.allowedTools.forEach((tool, i) => {
                if (typeof tool !== "string" || tool.trim() === "") {
                    errors.push({ field: `allowedTools[${i}]`, expected: "non-empty string pattern", actual: JSON.stringify(tool) });
                }
            });
        }
    }

    // resources
    if (config.resources !== undefined) {
        if (!Array.isArray(config.resources)) {
            errors.push({ field: "resources", expected: "array", actual: typeof config.resources });
        } else {
            config.resources.forEach((res, i) => {
                if (typeof res === "string") {
                    if (!FILE_URI_PATTERN.test(res) && !SKILL_URI_PATTERN.test(res)) {
                        errors.push({ field: `resources[${i}]`, expected: "file:// or skill:// URI", actual: res });
                    }
                } else if (typeof res === "object" && res !== null) {
                    if (res.type !== "knowledgeBase") {
                        errors.push({ field: `resources[${i}].type`, expected: "knowledgeBase", actual: res.type });
                    }
                    if (!res.source || typeof res.source !== "string") {
                        errors.push({ field: `resources[${i}].source`, expected: "non-empty string", actual: JSON.stringify(res.source) });
                    }
                    if (!res.name || typeof res.name !== "string") {
                        errors.push({ field: `resources[${i}].name`, expected: "non-empty string", actual: JSON.stringify(res.name) });
                    }
                } else {
                    errors.push({ field: `resources[${i}]`, expected: "string URI or knowledgeBase object", actual: typeof res });
                }
            });
        }
    }

    // hooks
    if (config.hooks !== undefined) {
        if (typeof config.hooks !== "object" || config.hooks === null || Array.isArray(config.hooks)) {
            errors.push({ field: "hooks", expected: "object", actual: typeof config.hooks });
        } else {
            Object.keys(config.hooks).forEach((trigger) => {
                if (!VALID_HOOK_TRIGGERS.includes(trigger)) {
                    errors.push({
                        field: `hooks.${trigger}`,
                        expected: `one of: ${VALID_HOOK_TRIGGERS.join(", ")}`,
                        actual: trigger,
                    });
                }
                const hookList = config.hooks[trigger];
                if (!Array.isArray(hookList)) {
                    errors.push({ field: `hooks.${trigger}`, expected: "array of hook objects", actual: typeof hookList });
                } else {
                    hookList.forEach((hook, i) => {
                        if (typeof hook !== "object" || hook === null) {
                            errors.push({ field: `hooks.${trigger}[${i}]`, expected: "object with command field", actual: typeof hook });
                        } else if (!hook.command || typeof hook.command !== "string") {
                            errors.push({ field: `hooks.${trigger}[${i}].command`, expected: "non-empty string", actual: JSON.stringify(hook.command) });
                        }
                    });
                }
            });
        }
    }

    // mcpServers
    if (config.mcpServers !== undefined) {
        if (typeof config.mcpServers !== "object" || config.mcpServers === null || Array.isArray(config.mcpServers)) {
            errors.push({ field: "mcpServers", expected: "object", actual: typeof config.mcpServers });
        } else {
            Object.keys(config.mcpServers).forEach((server) => {
                const srv = config.mcpServers[server];
                if (typeof srv !== "object" || srv === null) {
                    errors.push({ field: `mcpServers.${server}`, expected: "object", actual: typeof srv });
                } else if (srv.type === "http" || srv.type === "sse" || typeof srv.url === "string") {
                    if (!srv.url || typeof srv.url !== "string") {
                        errors.push({ field: `mcpServers.${server}.url`, expected: "non-empty string (HTTP/SSE MCP server)", actual: JSON.stringify(srv.url) });
                    }
                } else if (!srv.command || typeof srv.command !== "string") {
                    errors.push({ field: `mcpServers.${server}.command`, expected: "non-empty string (stdio MCP server) or a 'url' for an HTTP server", actual: JSON.stringify(srv.command) });
                }
            });
        }
    }

    // toolAliases
    if (config.toolAliases !== undefined) {
        if (typeof config.toolAliases !== "object" || config.toolAliases === null || Array.isArray(config.toolAliases)) {
            errors.push({ field: "toolAliases", expected: "object (key-value string map)", actual: typeof config.toolAliases });
        } else {
            Object.entries(config.toolAliases).forEach(([key, val]) => {
                if (typeof val !== "string") {
                    errors.push({ field: `toolAliases["${key}"]`, expected: "string", actual: typeof val });
                }
            });
        }
    }

    // toolsSettings
    if (config.toolsSettings !== undefined) {
        if (typeof config.toolsSettings !== "object" || config.toolsSettings === null || Array.isArray(config.toolsSettings)) {
            errors.push({ field: "toolsSettings", expected: "object", actual: typeof config.toolsSettings });
        }
    }

    // includeMcpJson
    if (config.includeMcpJson !== undefined && typeof config.includeMcpJson !== "boolean") {
        errors.push({ field: "includeMcpJson", expected: "boolean", actual: typeof config.includeMcpJson });
    }

    // keyboardShortcut
    if (config.keyboardShortcut !== undefined) {
        if (typeof config.keyboardShortcut !== "string") {
            errors.push({ field: "keyboardShortcut", expected: "string (e.g. ctrl+a)", actual: typeof config.keyboardShortcut });
        } else if (!/^(ctrl|shift)\+[a-z0-9]$/i.test(config.keyboardShortcut)) {
            errors.push({ field: "keyboardShortcut", expected: "format: modifier+key (e.g. ctrl+a, shift+1)", actual: config.keyboardShortcut });
        }
    }

    // welcomeMessage
    if (config.welcomeMessage !== undefined && typeof config.welcomeMessage !== "string") {
        errors.push({ field: "welcomeMessage", expected: "string", actual: typeof config.welcomeMessage });
    }

    return errors;
}

// --- Main ---

const args = process.argv.slice(2);

if (args.includes("--help") || args.includes("-h")) {
    printHelp();
    process.exit(0);
}

const format = args.includes("--format")
    ? args[args.indexOf("--format") + 1] || "text"
    : "text";

const targetPath = args.find((a) => !a.startsWith("--") && a !== format);

if (!targetPath) {
    console.error("Error: No agent config file specified.\n");
    printHelp();
    process.exit(2);
}

if (!fs.existsSync(targetPath)) {
    console.error(`Error: File not found: ${targetPath}`);
    process.exit(2);
}

const errors = validate(targetPath);

if (format === "json") {
    const result = {
        file: targetPath,
        valid: errors.length === 0,
        errors: errors,
    };
    console.log(JSON.stringify(result, null, 2));
    process.exit(errors.length === 0 ? 0 : 1);
}

// Text format (default)
if (errors.length === 0) {
    console.log(`\u2713 Valid: ${targetPath}`);
    process.exit(0);
} else {
    console.error(`\u2717 Invalid: ${targetPath}\n`);
    errors.forEach((err) => {
        console.error(`  ${err.field}: expected ${err.expected}, got ${err.actual}`);
    });
    console.error(`\n${errors.length} error(s) found.`);
    process.exit(1);
}
