"""
Convert IDE .md agents to identical Kiro CLI .json agents AND replicate
the full IDE environment (steering, skills, AGENTS.md) for evaluation fidelity.

Usage:
  python create_agent.py                        # Sync everything
  python create_agent.py --list                 # Show available agents
  python create_agent.py --agents-only          # Only regenerate .json agents
"""

import argparse
import json
import re
import shutil
import sys
from pathlib import Path

SOURCE_PROJECT = Path(__file__).parent.parent  # d:\realestate
EVAL_PROJECT = Path(__file__).parent  # d:\realestate\subagent-eval


def parse_md_agent(md_path: Path) -> dict | None:
    content = md_path.read_text(encoding="utf-8")
    match = re.match(r"^---\s*\n(.*?)\n---\s*\n(.*)$", content, re.DOTALL)
    if not match:
        return None

    frontmatter = match.group(1)
    body = match.group(2).strip()

    name_match = re.search(r'^name:\s*["\']?([^"\'\n]+)["\']?', frontmatter, re.MULTILINE)
    desc_match = re.search(r'^description:\s*"(.*?)"', frontmatter, re.MULTILINE | re.DOTALL)
    tools_match = re.search(r'^tools:\s*\[(.*?)\]', frontmatter, re.MULTILINE)

    if not name_match:
        return None

    name = name_match.group(1).strip()
    description = desc_match.group(1).strip() if desc_match else ""
    tools = []
    if tools_match:
        tools = [t.strip().strip('"').strip("'") for t in tools_match.group(1).split(",")]

    return {"name": name, "description": description, "tools": tools, "body": body}


def build_cli_config(agent: dict, prompts_dir: Path, model: str) -> dict:
    prompt_file = prompts_dir / f"{agent['name']}-system.md"
    prompt_file.parent.mkdir(parents=True, exist_ok=True)
    prompt_file.write_text(agent["body"], encoding="utf-8")

    prompt_ref = f"file://../shared/{agent['name']}-system.md"

    tool_map = {"read": "read", "write": "write", "shell": "shell", "web": "web", "@mcp": "@context7"}
    tools = [tool_map.get(t, t) for t in agent["tools"]]
    allowed_tools = tools[:]

    return {
        "name": agent["name"],
        "description": agent["description"],
        "prompt": prompt_ref,
        "model": model,
        "tools": tools,
        "allowedTools": allowed_tools,
        "resources": [
            "file://../../AGENTS.md",
            "file://../../.kiro/steering/**/*.md",
            "skill://../../.kiro/skills/**/SKILL.md",
        ],
    }


def sync_environment():
    copies = [
        ("AGENTS.md", "AGENTS.md"),
        (".kiro/steering", ".kiro/steering"),
        (".kiro/skills", ".kiro/skills"),
    ]
    print("Syncing IDE environment:\n")
    for src_rel, dst_rel in copies:
        src = SOURCE_PROJECT / src_rel
        dst = EVAL_PROJECT / dst_rel
        if not src.exists():
            print(f"  ⚠ {src_rel} not found, skipping")
            continue
        if dst.exists():
            if dst.is_dir():
                shutil.rmtree(dst)
            else:
                dst.unlink()
        if src.is_dir():
            shutil.copytree(src, dst)
            count = sum(1 for _ in dst.rglob("*") if _.is_file())
            print(f"  ✓ {src_rel}/ ({count} files)")
        else:
            dst.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dst)
            print(f"  ✓ {src_rel}")
    print()


def convert_agents(model: str, names: list[str] | None = None):
    source = SOURCE_PROJECT / ".kiro" / "agents"
    output = EVAL_PROJECT / ".kiro" / "agents"
    prompts_dir = EVAL_PROJECT / ".kiro" / "shared"
    output.mkdir(parents=True, exist_ok=True)
    prompts_dir.mkdir(parents=True, exist_ok=True)

    # Copy .json agents directly (like eval-judge.json)
    for json_file in sorted(source.glob("*.json")):
        shutil.copy2(json_file, output / json_file.name)
        print(f"  ✓ {json_file.name} (copied)")

    # Convert .md agents to .json
    agents = []
    for md_file in sorted(source.glob("*.md")):
        agent = parse_md_agent(md_file)
        if agent:
            if names and agent["name"] not in names:
                continue
            agents.append(agent)

    if not agents:
        print("No agents found.")
        sys.exit(1)

    print(f"Converting {len(agents)} agent(s) → {output}/\n")
    for agent in agents:
        config = build_cli_config(agent, prompts_dir, model)
        out_file = output / f"{agent['name']}.json"
        out_file.write_text(json.dumps(config, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
        print(f"  ✓ {agent['name']}.json")

    print(f"\n  Run default agent (orchestrator): kiro-cli chat --no-interactive --trust-all-tools \"prompt\"")
    print(f"  Run specialist directly: kiro-cli chat --no-interactive --agent {agents[0]['name']} --trust-all-tools \"prompt\"")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("names", nargs="*")
    parser.add_argument("--model", default="claude-opus-4-6")
    parser.add_argument("--list", action="store_true")
    parser.add_argument("--agents-only", action="store_true")
    args = parser.parse_args()

    source = SOURCE_PROJECT / ".kiro" / "agents"
    if not source.exists():
        print(f"Error: {source} not found")
        sys.exit(1)

    if args.list:
        for md_file in sorted(source.glob("*.md")):
            agent = parse_md_agent(md_file)
            if agent:
                print(f"  {agent['name']:25s} [{', '.join(agent['tools'])}]")
        return

    if not args.agents_only:
        sync_environment()
    convert_agents(args.model, args.names or None)


if __name__ == "__main__":
    main()
