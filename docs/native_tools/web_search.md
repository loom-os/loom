# Web Search Tool

Search the web using Brave Search API.

## API

Uses [Brave Search API](https://brave.com/search/api/) - a privacy-focused search engine with high-quality results.

### Free Tier
- 2,000 queries/month
- No credit card required

---

## web:search

Search the web for information.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `query` | string | Yes | Search query |
| `limit` | integer | No | Max results (default: 5, max: 20) |

### Returns

```json
{
  "query": "Bitcoin price",
  "count": 5,
  "results": [
    {
      "title": "Bitcoin Price Today | BTC Live...",
      "url": "https://coinmarketcap.com/currencies/bitcoin/",
      "snippet": "The live Bitcoin price today is $93,030.31 USD..."
    },
    {
      "title": "Bitcoin USD (BTC-USD) Price...",
      "url": "https://finance.yahoo.com/quote/BTC-USD/",
      "snippet": "Find the latest Bitcoin USD price..."
    }
  ]
}
```

### Fields

| Field | Type | Description |
|-------|------|-------------|
| `query` | string | Original search query |
| `count` | int | Number of results returned |
| `results` | array | Array of search results |
| `results[].title` | string | Page title |
| `results[].url` | string | Page URL |
| `results[].snippet` | string | Text snippet (may be null) |

### Errors

| Error | Cause |
|-------|-------|
| `ExecutionFailed` | API key not configured |
| `ExecutionFailed` | Request timeout (network issue) |
| `ExecutionFailed` | API error (rate limit, invalid key) |

---

## Configuration

### Required: API Key

Get a free API key from [Brave Search API](https://brave.com/search/api/):

1. Sign up at https://brave.com/search/api/
2. Create an API key (choose "Free" plan)
3. Add to your `.env` file:

```bash
BRAVE_API_KEY='your-api-key-here'
```

### Optional: Proxy

If accessing from regions where Brave is blocked, configure a proxy:

```bash
HTTPS_PROXY='http://127.0.0.1:7897'
```

The tool checks these environment variables in order:
1. `HTTPS_PROXY`
2. `HTTP_PROXY`
3. `ALL_PROXY`
4. `https_proxy` (lowercase)
5. `http_proxy` (lowercase)
6. `all_proxy` (lowercase)

---

## Examples

### Basic Search

```python
result = await ctx.tool("web:search", {
    "query": "Python async programming",
    "limit": 5
})

for item in result["results"]:
    print(f"📄 {item['title']}")
    print(f"   {item['url']}")
    if item.get("snippet"):
        print(f"   {item['snippet'][:100]}...")
    print()
```

### Research Pattern

```python
# Search and synthesize
result = await ctx.tool("web:search", {
    "query": "best practices for REST API design 2024"
})

# Extract relevant information
sources = []
for item in result["results"]:
    sources.append({
        "title": item["title"],
        "url": item["url"],
        "key_point": item.get("snippet", "")
    })

# Use in prompt for LLM
context = "\n".join([f"- {s['title']}: {s['key_point']}" for s in sources])
```

### In a Cognitive Agent

```python
cognitive = CognitiveAgent(
    ctx=agent._ctx,
    llm=llm,
    available_tools=["web:search", "fs:write_file"],
)

# Agent will search, synthesize, and save results
result = await cognitive.run(
    "Research the top 5 JavaScript frameworks in 2024 and save a summary to frameworks.md"
)
```

---

## Best Practices

1. **Be specific**: "Python web frameworks 2024" > "programming"
2. **Use natural language**: The API handles complex queries well
3. **Limit results**: Start with 5 results, increase if needed
4. **Handle missing snippets**: Some results may not have snippets
5. **Cite sources**: When using search results, include URLs

---

## Troubleshooting

### "BRAVE_API_KEY not configured"

Ensure the API key is set in your `.env` file and the file is in the agent's working directory.

### Request Timeout

Usually indicates network issues or proxy problems:
1. Check internet connectivity
2. Verify proxy settings if behind firewall
3. Try increasing timeout (requires code change)

### Rate Limit Exceeded

Free tier allows 2,000 queries/month. Check your usage at the Brave API dashboard.

---

## Comparison with Alternatives

| Feature | Brave Search | DuckDuckGo | Google/SerpAPI |
|---------|--------------|------------|----------------|
| Free tier | 2,000/month | Unlimited* | 100/month |
| Quality | High | Medium | Highest |
| Privacy | Yes | Yes | No |
| API stability | Good | Unofficial | Good |
| Setup | API key | None | API key |

*DuckDuckGo Instant Answer API is unofficial and may break.
