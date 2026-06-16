# MCP Server Anti-Patterns & Best Practices

Guide for building production-quality MCP servers with Rig + rmcp. Synthesized from community production experience and protocol design principles.

## Core Mental Model

An MCP server is a **User Interface for AI agents**, not a thin API wrapper. The LLM has no persistent memory of your API, no debugger, and a finite context window. Every tool description is part of the prompt. Every response consumes tokens. Every round-trip is a chance for the model to lose the thread.

**Design for outcomes, not operations.**

---

## Anti-Pattern Checklist

Use this as a review gate before shipping any MCP server.

| # | Anti-Pattern | Signal | Fix |
|---|---|---|---|
| 1 | API mirror | One MCP tool per REST endpoint | Outcome-oriented tools that orchestrate internally |
| 2 | God tool | Single tool with 10+ params doing multiple things | Split into focused tools with narrow contracts |
| 3 | Chatty protocol | Agent needs 5+ tool calls for one user goal | Combine steps into workflow tools |
| 4 | Vague schemas | Model refuses tool or picks wrong one | Precise descriptions, typed params, examples |
| 5 | Data dumps | Responses > 5K tokens of raw JSON | Paginate, summarize, return CSV for tabular data |
| 6 | Swallowed errors | Generic "Bad Request" or empty response | Structured errors with recovery guidance |
| 7 | No audit gate | Destructive tools with no confirmation | `dry_run` parameter + audit logging |
| 8 | Missing annotations | Client can't distinguish reads from writes | Use `readOnlyHint`, `destructiveHint`, `idempotentHint` |
| 9 | Retry storms | Non-idempotent tools double-fire on timeout | Idempotency keys derived from call inputs |
| 10 | Stdout pollution | JSON-RPC stream corrupted by log output | Route all non-protocol output to stderr/tracing |

---

## Detailed Guidance

### 1. Outcomes Over Operations

```rust
// ❌ BAD: Agent must orchestrate 3 calls
#[tool(description = "Get user by email")]
async fn get_user(&self, email: String) -> Result<User> { ... }

#[tool(description = "List orders for user")]
async fn list_orders(&self, user_id: i64) -> Result<Vec<Order>> { ... }

#[tool(description = "Get shipment for order")]
async fn get_shipment(&self, order_id: i64) -> Result<Shipment> { ... }

// ✅ GOOD: One call, one outcome
#[tool(description = "Track the latest order for a customer. Returns order status, \
    tracking number, and estimated delivery. Use when user asks about order status.")]
async fn track_latest_order(&self, email: String) -> Result<OrderTracking> { ... }
```

**Heuristic**: If a typical user goal requires 5+ tool calls, you've sliced too thin. If a tool takes 5+ parameters or needs 3 paragraphs to explain, you've sliced too thick.

### 2. Tool Descriptions Are Contracts

The schema is what the model negotiates against at every tool-selection step. Treat descriptions as instructions, not documentation.

```rust
// ❌ BAD: Vague, no guidance on when to use
#[tool(description = "Search")]
async fn search(&self, query: String) -> Result<Vec<Item>> { ... }

// ✅ GOOD: States when to use, how to format args, what comes back
#[tool(description = "Search properties by location or price range. Use when user \
    asks to find available units. Returns max 20 results with id, address, price, \
    and status. Use property_id from results with get_property_details for full info.")]
async fn search_properties(
    &self,
    /// City or neighborhood name, e.g. "Santo Domingo Este"
    location: Option<String>,
    /// Minimum monthly rent in DOP
    min_price: Option<f64>,
    /// Maximum monthly rent in DOP
    max_price: Option<f64>,
    /// Number of results (default 20, max 50)
    limit: Option<u32>,
) -> Result<Vec<PropertySummary>> { ... }
```

**Rules for descriptions:**
- State WHEN to use the tool (trigger condition)
- State HOW to format arguments (include examples for non-obvious formats)
- State WHAT the response looks like (shape, not full schema)
- Keep under 50 words per tool description for context efficiency
- Use `///` doc comments on parameters (schemars v1 picks these up)

### 3. Flatten Arguments — No Nested Blobs

```rust
// ❌ BAD: Agent must guess the dict structure
#[tool(description = "Create a contract")]
async fn create_contract(&self, data: serde_json::Value) -> Result<Contract> { ... }

// ✅ GOOD: Flat, typed, constrained
#[tool(description = "Create a new rental contract between a property and tenant.")]
async fn create_contract(
    &self,
    /// Property ID from search_properties results
    property_id: i64,
    /// Tenant ID from search_tenants results
    tenant_id: i64,
    /// Contract start date in ISO 8601 format (YYYY-MM-DD)
    start_date: String,
    /// Contract end date in ISO 8601 format (YYYY-MM-DD)
    end_date: String,
    /// Monthly rent amount
    monthly_amount: f64,
    /// Currency: "DOP" or "USD"
    currency: String,
) -> Result<Contract> { ... }
```

### 4. Errors Are Recovery Instructions

When a tool fails, the model reads the error as an observation and uses it to self-correct. Generic errors cause expensive retry loops.

```rust
// ❌ BAD: Dead end for the model
return Err(anyhow!("Bad Request"));
return Err(anyhow!("Not found"));

// ✅ GOOD: Tells the model what went wrong and what to do next
return Err(anyhow!(
    "Tenant with cedula '001-1234567-8' not found. \
     Use search_tenants with name to find the correct tenant_id."
));

return Err(anyhow!(
    "Cannot create contract: property {} already has an active contract \
     (id: {}) until {}. Cancel or wait for expiry first.",
    property_id, existing.id, existing.end_date
));
```

**Three error categories to handle differently:**
1. **Input errors** (bad params, not found): Return specific guidance on how to fix
2. **Transient failures** (timeout, rate limit): Return retry hint with backoff suggestion
3. **Fatal errors** (auth expired, service down): Return clear "this won't work until X" message

### 5. Paginate and Compress Responses

```rust
// ❌ BAD: 500 rows of verbose JSON
#[tool(description = "List all payments")]
async fn list_payments(&self) -> Result<Vec<Payment>> {
    self.db.find_all().await  // could be thousands
}

// ✅ GOOD: Bounded, with pagination metadata
#[tool(description = "List payments with pagination. Returns up to `limit` results. \
    Use `offset` to page through. Response includes total_count and has_more.")]
async fn list_payments(
    &self,
    /// Number of results per page (default 20, max 50)
    limit: Option<u32>,
    /// Offset for pagination (default 0)
    offset: Option<u32>,
    /// Filter by status: "pendiente", "pagado", "atrasado"
    status: Option<String>,
) -> Result<PaginatedResponse<PaymentSummary>> { ... }
```

**Token-saving strategies:**
- Return summaries by default, full details on explicit lookup
- Use CSV format for tabular data (40-60% token savings over JSON)
- Strip null fields from responses
- Convert internal representations (cents → dollars, epoch → ISO 8601)
- Never return fields the agent won't use

### 6. Destructive Tools Need Audit Gates

Any tool that writes, deletes, sends, or pays needs explicit confirmation.

```rust
#[tool(description = "Delete a property and all associated records. DESTRUCTIVE. \
    Call with dry_run=true first to preview what will be deleted. \
    Agent MUST request user confirmation before calling with dry_run=false.")]
async fn delete_property(
    &self,
    property_id: i64,
    /// Set to true to preview deletion without executing. Default: true.
    dry_run: Option<bool>,
) -> Result<DeletionPreview> {
    let dry = dry_run.unwrap_or(true); // safe default
    if dry {
        // Return what WOULD be deleted
        let preview = self.compute_cascade(property_id).await?;
        Ok(DeletionPreview { executed: false, ..preview })
    } else {
        // Actually delete + audit log
        let result = self.execute_delete(property_id).await?;
        self.audit_log("delete_property", property_id, &result).await;
        Ok(result)
    }
}
```

**Three layers for destructive tools:**
1. `dry_run: true` by default — shows what would happen
2. Audit log entry on every mutation (who, what, when)
3. Description explicitly states the gate: "Agent MUST request user confirmation"

### 7. Idempotency for Side-Effect Tools

Clients retry on timeout. Without idempotency, your tool fires twice.

```rust
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

struct IdempotencyCache {
    cache: Mutex<HashMap<String, (Instant, serde_json::Value)>>,
    ttl: Duration,
}

impl IdempotencyCache {
    fn get_or_execute<F, Fut>(&self, key: &str, f: F) -> Result<serde_json::Value>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<serde_json::Value>>,
    {
        let mut cache = self.cache.lock().unwrap();
        if let Some((ts, result)) = cache.get(key) {
            if ts.elapsed() < self.ttl {
                return Ok(result.clone()); // short-circuit duplicate
            }
        }
        drop(cache);

        let result = f().await?;
        self.cache.lock().unwrap().insert(key.to_string(), (Instant::now(), result.clone()));
        Ok(result)
    }
}

// Derive idempotency key from call inputs
fn idempotency_key(tool: &str, args: &impl Serialize) -> String {
    let hash = sha256(serde_json::to_string(args).unwrap());
    format!("{}:{}", tool, hash)
}
```

### 8. Curate Ruthlessly

- **5–15 tools per server**. More causes selection confusion.
- **One server, one domain**. Don't mix payments and maintenance in one server.
- **Delete unused tools**. Every tool description costs tokens on every request.
- **Don't expose admin internals**. If users don't need it, the agent doesn't need it.

**The five-tool test**: If a typical user goal requires the model to call 5+ tools, you've sliced too thin. If a tool takes 5+ parameters, you've likely sliced too thick.

### 9. Concurrent Access Safety

MCP does NOT serialize tool calls. Multiple calls can arrive in parallel.

```rust
use tokio::sync::Mutex;
use std::collections::HashMap;

struct MyServer {
    // Per-resource locks for mutable state
    resource_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl MyServer {
    async fn with_resource_lock<F, Fut, T>(&self, resource_id: &str, f: F) -> T
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        let lock = {
            let mut locks = self.resource_locks.lock().await;
            locks.entry(resource_id.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };
        let _guard = lock.lock().await;
        f().await
    }
}
```

For database-backed state, prefer optimistic locking (version column + conditional update) over in-process mutexes — the mutex breaks the moment you run multiple server instances.

### 10. Transport Hygiene

```rust
// Ensure all logging goes to stderr, never stdout (stdio transport uses stdout for JSON-RPC)
fn init_logging() {
    // tracing subscriber writes to stderr by default — this is correct
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new("info"))
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .init();
}

// For Streamable HTTP: use the current spec transport, not deprecated SSE
// rmcp's StreamableHttpServerTransport is the correct choice
```

---

## Design Review Questions

Ask these before shipping any MCP server:

1. Can the agent achieve its goal in ≤3 tool calls for the common case?
2. Would a first-time reader of the tool descriptions know which tool to pick?
3. Are all responses under 2K tokens for the common case?
4. Do error messages tell the model what to do next?
5. Can any tool mutate external state? If yes, does it have `dry_run` + audit?
6. Are parameters flat primitives with constrained types (enums, bounded numbers)?
7. Is every tool independently callable (no required ordering between tools)?
8. Would two concurrent calls to the same tool produce correct results?

---

## Sources

Content was rephrased for compliance with licensing restrictions.

- [MCP Best Practices — Phil Schmid](https://www.philschmid.de/mcp-best-practices)
- [How Not to Write an MCP Server — Towards Data Science](https://towardsdatascience.com/how-not-to-write-an-mcp-server/)
- [Building MCP Servers: 7 Mistakes to Avoid — BigData Boutique](https://bigdataboutique.com/blog/building-mcp-servers-with-fastmcp-7-mistakes-to-avoid)
- [MCP Server Anti-Patterns — Digital Applied](https://www.digitalapplied.com/blog/mcp-server-anti-patterns-design-mistakes-2026-developer-guide)
- [Designing an MCP Server from a REST API — WorkOS](https://workos.com/blog/designing-mcp-server-from-rest-api)
- [Production-Ready MCP Servers — Mohammad Khan](https://mohammadkhan.dev/blog/production-ready-mcp-servers)
- [5 Production Gotchas — Albino Geek](https://www.albinogeek.com/posts/mcp-5-production-gotchas)
- [MCP Misconceptions — Docker](https://www.docker.com/blog/mcp-misconceptions-tools-agents-not-api/)
- [Seven Deadly Sins of MCP — DEV Community](https://dev.to/riferrei/the-seven-deadly-sins-of-mcp-18kb)
