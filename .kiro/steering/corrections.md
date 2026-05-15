---
inclusion: always
---

# Human Corrections

Log of mistakes corrected by the user. Read before acting to avoid repeating them. Keep under 200 words.

Format: `- **Topic**: correction`

---

- **Skills in hooks**: Skills are activated automatically by the IDE based on their description. Don't tell the agent to "read the skill" in hook prompts — just describe the task and trust the skill will be loaded.
- **Hooks design**: Hooks should not be overly prescriptive with specific file paths and tasks. Keep prompts ambiguous enough for the agent to discover what needs doing. Use sub-agents with personas (planner, coder, reviewer, tester) for complex workflows.
- **WhatsApp OCR design**: OCR is a tool the LLM invokes (Rig tool-use pattern), not a separate pipeline branch. Don't hardcode image→OCR routing in the message router.
- **Self-hosted only**: This project uses only self-hosted models. Never suggest external API providers (OpenAI, Anthropic, etc.) as options for inference.
- **Deployment**: Everything runs on Kubernetes, not Docker Compose. Don't reference docker-compose for deployment or configuration.
- **Sidebar navigation**: Chatbot WhatsApp belongs under Configuración (Sistema section), not as its own item in Herramientas. It's a configuration sub-page.
- **Container images**: Use official pre-built container images (e.g. intel/xpumanager from DockerHub) instead of building custom images or extracting RPMs manually. Don't overcomplicate.
- **OVMS endpoint**: The OVMS endpoint is `/v3`, not `/v1`. The readiness probe path (`/v1/config`) is unrelated to the chat completions API path. Don't change it.
- **WhatsApp self-messaging**: The user wants to message themselves (same number as the bot) for testing. Don't filter out self-messages in the baileys-service.
