# Vertex AI Chat CLI

Interactive command-line chat interface for testing the Vertex AI SDK.

## Build

```bash
# Build the CLI with optional features
cargo build --bin vertex-chat --features cli

# Or build in release mode for better performance
cargo build --release --bin vertex-chat --features cli
```

## Requirements

### Authentication

The CLI requires Google Cloud Platform authentication credentials. Set these environment variables:

```bash
export GCP_PRIVATE_KEY="-----BEGIN PRIVATE KEY-----\n...\n-----END PRIVATE KEY-----\n"
export GCP_CLIENT_EMAIL="service-account@project.iam.gserviceaccount.com"
export GCP_CLIENT_ID="your-client-id"
```

**Alternatively**, use Application Default Credentials:

```bash
gcloud auth application-default login
```

### Project Configuration

```bash
export VERTEX_PROJECT="your-gcp-project-id"
export VERTEX_LOCATION="us-central1"  # or your preferred region
```

## Usage

### Basic Usage

```bash
cargo run --bin vertex-chat --features cli -- --project your-project-id
```

### With All Options

```bash
cargo run --bin vertex-chat --features cli -- \
  --project your-project-id \
  --location us-central1 \
  --model gemini-2.5-flash \
  --temperature 0.9 \
  --max-tokens 8192 \
  --system "You are a helpful assistant"
```

### Using Environment Variables

If you've set `VERTEX_PROJECT` and `VERTEX_LOCATION`:

```bash
cargo run --bin vertex-chat --features cli
```

## Command-Line Options

```
Options:
  -p, --project <PROJECT>           Project ID
  -l, --location <LOCATION>         Location/region [default: us-central1]
  -m, --model <MODEL>               Model to use [default: gemini-2.5-flash]
  -t, --temperature <TEMPERATURE>   Temperature (0.0 to 2.0) [default: 0.9]
  -o, --max-tokens <MAX_TOKENS>     Max output tokens [default: 8192]
  -s, --system <SYSTEM>             System instruction
  -d, --debug                       Enable debug logging
  -h, --help                        Print help
  -V, --version                     Print version
```

## Interactive Commands

While chatting, you can use these commands:

- `help` - Show available commands
- `clear` - Clear conversation history
- `stats` - Show conversation statistics
- `temp` - Change temperature setting
- `quit` / `exit` / `bye` - Exit the chat

## Examples

### Simple Chat

```
$ cargo run --bin vertex-chat --features cli

=== Vertex AI Chat ===
Project: my-project
Location: us-central1
Model: gemini-2.5-flash
Temperature: 0.9

Commands: 'help', 'clear', 'stats', 'temp', 'quit'

You: Hello! What can you help me with?
Assistant: Hello! I'm an AI assistant powered by Google's Gemini model...

(tokens: 12 in, 85 out, 97 total)

You: quit
Goodbye!
```

### With System Instruction

````bash
cargo run --bin vertex-chat --features cli -- \
  --system "You are a Python expert. Always provide code examples."

You: How do I read a file in Python?
Assistant: Here's how to read a file in Python:

```python
# Read entire file
with open('file.txt', 'r') as f:
    content = f.read()
```...
````

### Adjusting Temperature

```
You: temp
New temperature (0.0-2.0): 1.5
Temperature set to 1.5

You: Tell me a creative story
Assistant: Once upon a time in a digital realm...
```

## Token Usage

The CLI displays token usage after each response:

```
(tokens: 45 in, 128 out, 173 total)
```

- **in**: Prompt tokens (your input + conversation history)
- **out**: Completion tokens (model's response)
- **total**: Total tokens consumed

## Troubleshooting

### Authentication Errors

```
Error creating client: Authentication { message: "..." }

Make sure you have set the following environment variables:
  GCP_PRIVATE_KEY
  GCP_CLIENT_EMAIL
  GCP_CLIENT_ID

Or run: gcloud auth application-default login
```

**Solution**: Verify your GCP credentials are set correctly.

### Project Not Found

```
thread 'main' panicked at: Project ID required (--project or VERTEX_PROJECT env var)
```

**Solution**: Provide project ID via `--project` flag or `VERTEX_PROJECT` environment variable.

### Model Not Available

```
Error: Api { message: "Status 404: Model not found", code: "404" }
```

**Solution**: Check that the model name is correct and available in your region. Try:

- `gemini-3-pro-preview`
- `gemini-2.5-flash`
- `gemini-2.5-pro`

### Rate Limiting

```
Error: Api { message: "Status 429: Resource exhausted", code: "429" }
```

**Solution**: Wait a moment before continuing. Consider reducing request frequency.

## Development

### Enable Debug Logging

```bash
cargo run --bin vertex-chat --features cli -- --debug
```

This shows detailed HTTP requests, authentication flow, and API responses.

### Build for Distribution

```bash
# Build optimized binary
cargo build --release --bin vertex-chat --features cli

# Binary location
./target/release/vertex-chat --help
```

## Features

✅ Multi-turn conversations with history ✅ Interactive command mode ✅ Token usage tracking ✅ Adjustable temperature
✅ System instructions ✅ Error handling with helpful messages ✅ Color-coded output ✅ Conversation statistics

## Architecture

The CLI is built on top of `threatflux-vertex-rust-sdk` and demonstrates:

- Client initialization with authentication
- Request/response handling
- Conversation history management
- Error handling and recovery
- Token usage monitoring

See [`src/bin/chat.rs`](src/bin/chat.rs) for implementation details.

## Streaming Support

The CLI now supports **real-time streaming** of responses! Tokens are displayed as they're generated, providing a more
interactive experience.

### How It Works

The CLI automatically uses streaming by default. You'll see tokens appear in real-time as the model generates them:

```
You: Write a short story about a robot
Assistant: Once upon a time, in a world where technology and humanity
intertwined seamlessly, there lived a robot named Unit-7X...
```

### Streaming Features

- **Real-time token delivery**: Tokens appear immediately as generated
- **Progress feedback**: See the model "thinking" in real-time
- **Token usage tracking**: Final chunk includes complete usage statistics
- **Error handling**: Graceful handling of network interruptions

### Technical Details

The streaming implementation uses Server-Sent Events (SSE) with the Vertex AI API:

- URL parameter: `?alt=sse` enables SSE format
- Incremental chunks: Each chunk contains a portion of the response
- Final chunk: Includes `usageMetadata` with token counts

## Additional Features

- ✅ **Real-time streaming** (SSE-based)
- Implement conversation save/load (future)
- Add multi-modal support (images, audio) (future)
- Support function calling (future)
- Context caching for long conversations (future)
