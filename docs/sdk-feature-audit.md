# SDK feature audit and implementation plan

Audited **2026-10-05** against fetched `main`
`c239d0844ec73a90487e45f98c91cd3a940e5bab` (crate 0.10.0). This is an audit
and a proposed follow-up plan. The modernization change does not implement the
features or protocol repairs described here.

The comparison covers the crate's existing generative AI scope: Gemini on
Vertex AI, context caching, text embeddings, model discovery, OAuth, and Claude
hosted on Vertex. It does not propose reproducing the entire Vertex AI service
catalog. Google's current documentation redirects some Vertex pages to Gemini
Enterprise Agent Platform; the audited REST service remains `aiplatform`.
Gemini Developer API endpoints, credentials, and Files resources are separate.

## Evidence and limits

Both official discovery documents were fetched and parsed, rather than inferred
from search excerpts:

| Source | Retrieved evidence |
| --- | --- |
| [Vertex REST v1 discovery][discovery-v1] | Revision `20260930`; 3,891,087 bytes; SHA-256 `1f477df5b247243f734c0086b79eda11dace4b41f9249e4d4b7b8650fb5b684e` |
| [Vertex REST v1beta1 discovery][discovery-beta] | Revision `20260930`; 5,275,610 bytes; SHA-256 `869114a0f86ef50728fd98645579d7e37348578011a066931be80f393b273897` |
| [Google API protobuf definitions][google-source] | Immutable commit `03a91044136a014466d4293eb1fe91f2b02075d2`; read content, tools, prediction, cache, cache service, publisher model, and model garden definitions |
| [Official Anthropic Vertex client][claude-vertex-source] and generated types | Immutable commit `18f25547f20cf5f01da69ac611e700e3bc9ebf21`; read Vertex routing, tool choice, adaptive thinking, output config, document, and citation types |

Findings below come from comparing those schemas and official feature guides
with source code. No authenticated cloud requests, inference, cache mutations,
or external tool execution were performed. Suggested regressions are future
work, not tests claimed to have passed in this audit. Discovery fields establish
a schema, not availability on every model, region, project, or launch stage.

P1 means an existing advertised path can send the wrong body, lose output, or
fail to decode a valid response. P2 means a missing current field, operation, or
convenience that should follow core correctness. Deferred means separate scope
or a surface needing model-specific or preview validation.

## Verified gaps

| Priority / kind | Existing behavior and code | Authoritative comparison | Proposed repair and compatibility |
| --- | --- | --- | --- |
| P1 / Gemini wire and replay bug | [`Part`](../src/types/content.rs) is an untagged enum with `Thinking { thought: String }`. A real `{text, thought: true}` becomes ordinary `Text`; `thoughtSignature` is discarded. Signed function-call parts also lose their signatures when replayed by [`execute_function_calling_flow`](../src/api/functions.rs). | [`Part`][content-rest] has boolean `thought` and a separate opaque `thoughtSignature` beside its data. [Replay guidance][thought-signatures] requires retaining returned parts. | Introduce a current part envelope with data, boolean thought, signature, and metadata. Keep the legacy enum usable through explicit adapters; its variants and struct literals cannot gain fields without consumer impact. Preserve signed parts verbatim and distinguish thoughts from answer text. |
| P1 / cache wire bug | [`CachedContent`](../src/cache.rs) has no `model`, so `create_cache` cannot send a model selection. The same DTO requires `contents` in create/get/list/update responses even though it is input-only; update_cache_ttl always uses updateMask=ttl even if the public request sets expireTime. | [Cache creation][cache-create] includes a full model resource and demonstrates a response without contents. [`CachedContent`][cache-proto] distinguishes input-only content, tools, and system instruction from output metadata. | Split current create and resource DTOs, require a model for creation, retain model/tool config/encryption fields, and default omitted repeated output fields in the legacy decoder. Add a new model-aware create method; do not silently guess from unrelated client defaults. |
| P1 / grounding wire bug | [`with_google_search`](../src/models.rs) sends only legacy `googleSearchRetrieval`. `GroundingConfig` emits `disableAttribution`, which is not in that retrieval schema. Response helpers look at root `groundingMetadata`; [`Candidate`](../src/types/safety.rs) omits it. [`GroundingChunk` and `GroundingSupport`](../src/types/grounding.rs) expect flat URL/text/span fields. | [Current Search examples][search-guide] use `googleSearch`. [`Candidate`][candidate-rest] owns grounding metadata. [Grounding schema][grounding-rest] uses nested chunk sources such as `web` and support `segment`/`confidenceScores`; discovery marks `googleSearchRetrieval` deprecated and its only property is `dynamicRetrievalConfig`. | Add current Search tooling and candidate-level nested grounding DTOs. Keep the legacy retrieval helper explicitly labeled. Stop promising an attribution-disable option without endpoint evidence. Retain source types rather than decoding nested sources into empty records. |
| P1 / response decoding and text-loss bug | [`GenerateContentResponse` / `StreamingResponse`](../src/models.rs) require `candidates`; candidate `content` requires role/parts. Prompt-blocked or content-filtered output may omit these. Text accessors use `find_map`, so only the first text part is returned. Several [`FinishReason`](../src/types/safety.rs) values incorrectly include a `FINISH_REASON_` prefix and fall into `Unknown`. | [Generation response][response-rest] includes prompt feedback and optional generated candidates; [candidate schema][candidate-rest] documents filtered content and uses `BLOCKLIST`, `PROHIBITED_CONTENT`, `SPII`, and `MALFORMED_FUNCTION_CALL`. | Add current response/candidate DTOs with optional content, prompt feedback, response/model IDs, and lossless finish reasons; preserve legacy enum variants through correct serde names and aliases where possible. Provide full ordered text extraction, avoiding thinking text. Keep known malformed data distinct from absent content. |
| P1 / shared streaming bug | [`SseStreamState::advance`](../src/streaming_support.rs) decodes each transport chunk with `from_utf8_lossy`, corrupting split UTF-8. One skipped comment/empty event causes both [`Gemini`](../src/api/stream.rs) and [`Claude`](../src/api/claude.rs) unfold loops to read more network data before draining buffered events. Multiple skipped events at EOF can discard later buffered data. Buffer and collected text have no configured bounds. | The [SSE standard][sse-standard] decodes a UTF-8 stream and dispatches framed events; network chunk boundaries are not text or event boundaries. | Use bounded bytes/incremental UTF-8 and drain all complete frames before awaiting more bytes. Cover CR/LF/CRLF and multiline data. Add completion-aware collectors: Gemini must track candidate finish state, Claude `message_stop`; unexpected EOF should not be reported as complete output. Preserve raw/unknown events and cancellation. |
| P1 / routing and configuration bug | [`ModelDescriptor::parse`](../src/model_descriptor.rs) accepts full resource names but strips project/location. [`model_request_context` / `location_for_model`](../src/client.rs) rebuild them from client settings and force selected names to global before publisher overrides. [`Config::api_version`](../src/config.rs) validates `v1beta1`, but API methods hard-code versions. | [`GenerateContentRequest.model`][prediction-proto] identifies the full publisher or tuned endpoint resource. [Cloud endpoint guidance][claude-cloud] makes regional, global, and multi-region selection an explicit choice. | Preserve full resource identity, validate resource segments, and define explicit resource/config precedence. Honor API version where an operation supports it. Keep short-name aliases as conveniences and never silently override an explicitly chosen location. Add current typed resource references rather than changing legacy descriptor fields blindly. |
| P1 / text embedding wire bug | [`EmbeddingInstance`](../src/api/embeddings.rs) serializes `taskType`. `EmbeddingRequest::batch` sends every instance in one call, including the default documented `gemini-embedding-001` use case. `CODE_RETRIEVAL_QUERY` is missing. | The [Vertex predict reference][embeddings-guide] requires `task_type`, lists the code retrieval task, and limits `gemini-embedding-001` to one input text per request. This is not Gemini Developer API `embedContent`. | Fix the serde key, retaining a decode alias if useful. Add a current task representation and model-aware validation. A separate bounded batch helper can make individual predict calls while preserving order and partial failures; do not advertise arbitrary instances as supported batching for this model. |
| P1 / inconsistent HTTP errors | [`embed`](../src/api/embeddings.rs), [`generate_content_impl`](../src/api/generate.rs), and [`count_tokens_impl`](../src/api/tokens.rs) parse success-shaped JSON without first checking HTTP status. Other entry points do check it. Several errors embed the full response body; the [`Gemini SSE parser`](../src/streaming.rs) logs the entire failed payload. | The [prediction service][prediction-proto] has distinct responses and service errors. The code itself proves entry-point inconsistency and raw payload logging. | Centralize bounded status-aware decoding and sanitized error context across normal/streaming/resource calls. A non-2xx success-shaped body must stay an error. Remove raw model/customer payloads from default logs; retain safe diagnostics and explicit opt-in inspection. |
| P1 / incomplete ADC behavior | [`from_env`](../src/auth.rs) treats every `GOOGLE_APPLICATION_CREDENTIALS` file as a service-account private key. Its `ApplicationDefaultCredentials` fallback checks an access-token variable, `gcloud auth print-access-token`, then metadata; it does not read the normal local ADC file. | [Google ADC search order][adc-guide] includes multiple credential-file types and the local file created by `gcloud auth application-default login`. CLI account credentials are distinct. | Use a verified maintained ADC provider or implement explicit supported credential kinds, including user refresh tokens and federation as selected scope. Preserve documented custom service-account precedence. Distinguish partial configuration errors from fallback and carry quota-project headers when applicable. |
| P2 / refresh and credential hardening | The three [`auth providers`](../src/auth.rs) check expiry under a read lock and refresh outside it, allowing concurrent refreshes. Static access tokens receive a guessed one-hour expiry. API 401 does not invalidate a rejected cached token. Credential-file reads are unbounded; direct `env::var` errors can retain non-Unicode values in their source chain. | [ADC guidance][adc-guide] defines supported credential sources; these race/error-boundary observations follow directly from source. No secret exposure was induced during this audit. | Share refresh work, respect actual expiry and non-refreshable token semantics, define one eligible refresh/retry policy, bound files, and sanitize credential errors at the boundary. Mark Authorization headers sensitive. Preserve the current absence of public Debug on auth providers. |
| P1 / chat system-instruction bug | [`chat_impl`, `stream_chat`, and `ChatConversation::to_contents`](../src/api/chat.rs) put `role: system` in ordinary contents. `chat_with_context` persists only extracted answer text, losing structured model parts/signatures; it already appends an assistant message internally. | [`Content`][content-rest] specifies user/model roles; [`GenerateContentRequest`][prediction-proto] has separate `system_instruction`. [Thought replay][thought-signatures] requires original structured parts. | Extract system messages into systemInstruction, with a documented multiple-system policy. Add a structured chat history retaining full parts. Keep text chat explicitly limited and clarify automatic history append behavior to avoid duplicate caller entries. |
| P2 / current Gemini generation fields | [`GenerationConfig` / `ThinkingConfig`](../src/types/config.rs) lack `includeThoughts`, medium/minimal thinking levels, seed, penalties, logprobs, response modalities, speech/media controls, and `responseJsonSchema`. The budget helper clamps values to one universal range. | [Content/config proto][content-proto] includes thinking and generation fields. Discovery also contains new `responseFormat` and marks legacy MIME/schema/image fields deprecated; that schema change alone does not prove every model supports the replacement. | Add current config DTOs with model-specific validation, explicit unset/default behavior, and structured-output exclusivity. Do not silently convert unsupported budgets or levels. Validate new responseFormat availability before making it the default; retain existing working legacy schema controls. |
| P2 / counting fields and operation | [`CountTokensRequest`](../src/models.rs) carries only contents; response retains only totalTokens. There is no computeTokens operation. It cannot represent counting the system instruction or tool declarations of a real generation request. | [`CountTokens` / `ComputeTokens`][prediction-proto] define endpoint-specific fields and token detail responses. | Add current counting DTOs and an allowlisted generation-to-count projection. Expose computeTokens separately for models that support it. Keep local character estimates clearly approximate and never route Claude counting through Gemini countTokens. |
| P2 / Gemini tools and replay fields | [`Tool`](../src/types/tools.rs) omits current Search, URL context, retrieval, Maps, and other provider-specific tool shapes. [`FunctionCall` / `FunctionResponse` / `FunctionDeclaration`](../src/types/function_calling.rs) omit IDs, partial arguments, continuation, multimodal results, and JSON-schema variants. | [Tool and function schemas][tool-rest] describe these fields; nonblocking behavior is explicitly tied to bidirectional generation. | Prioritize current Search and IDs/signature-safe ordinary function replay. Add other tools only with model/launch-stage evidence. Treat incremental calls as partial data, never complete executable calls. Keep Live-only scheduling/behavior in a distinct future surface. |
| P2 / response usage and metadata | [`UsageMetadata`](../src/types/usage.rs) uses a `modalityTokenCount` map that does not match the provider detail arrays and omits cache, thought, and tool-use counts. [`RequestMetadata`](../src/types/metadata.rs) is deliberately not serialized by generation, despite its field comments saying forwarded. | [Generation usage][response-rest] and [prediction request][prediction-proto] define usage detail arrays and billing labels. Arbitrary request metadata is not a Gemini field. | Preserve usage distinctions and cumulative stream semantics in current DTOs. Add real validated billing labels if needed; document custom metadata as local. Do not pretend that missing usage values are observed zero. |
| P2 / discovery semantics and pagination | [`ModelsApi`](../src/api/models.rs) decodes Vertex PublisherModel into a DTO resembling the Developer API model shape, ignoring launch stage/actions/schema/versionId. Missing supportedGenerationMethods becomes an empty vector; feature filtering treats it as unsupported and only reads the first page. List tokens for models, locations, and caches are concatenated without URL encoding. | [PublisherModel][publisher-proto] is a Model Garden resource, not a Gemini Developer API capability declaration. The v1beta1 discovery confirms publisher list with filter/view/order/version options. [Cache service][cache-service-proto] defines opaque paging tokens. | Add current publisher-resource DTOs and three-state capability results. Encode query parameters. Add lazy bounded pagination preserving filters, including empty continuing pages and repeated-token errors. Built-in model metadata remains a dated snapshot, not live availability. |
| P1 / Claude tool-choice wire bug | [`claude::ToolChoice`](../src/claude/types.rs) is untagged: Auto/Any/None serialize as null; Tool serializes only name. The existing None unit test asserts null. | [Official tool-choice type][claude-choice-source] requires a type discriminator; [tool use examples][claude-tools] show `{"type":"auto"}` and parallel-use controls. | Correct serialization without changing the Rust variant shapes, and use tagged decoding with explicit compatibility policy. Test every choice and forced named selection at the HTTP boundary; null must not be described as disabling tools. |
| P1 / Claude thinking effort wire bug | [`claude::ThinkingConfig::adaptive`](../src/claude/types.rs) writes effort inside thinking; OutputConfig requires format and has no effort. `allow_tool_use` is not a current thinking configuration field. | [Adaptive-thinking type][claude-thinking-source] contains type/display, while [output config][claude-output-source] owns optional effort and format. [Thinking guidance][claude-thinking] describes the same placement. | Add a current output config supporting effort without a JSON schema and an adapter for legacy adaptive helpers. Serialize effort in the correct location, reject contradictory duplicate configuration, and use current model-specific thinking/tool-choice rules. |
| P1 / Claude citations wire and decoding bug | [`enable_citations`](../src/claude/types.rs) emits a root citations field; Document lacks citations/title/context. The Citation DTO requires URL and title for every type. A citations_delta retains its type string but drops its nested citation. | [Citations guide][claude-citations] places enablement on documents and defines character/page/block location citations plus streaming citation deltas. | Add current document/citation unions and retain citation deltas. Keep web-search citations separate. A document citation without URL must decode, and enabling citations must change the document body rather than send an unverified root option. |
| P1 / Claude content and forward compatibility | [`ContentBlock`](../src/claude/types.rs) lacks redacted_thinking and rejects unknown block types. ToolResult accepts only optional strings, preventing structured text/image result blocks. RequestTool is untagged; a web-search value can deserialize as a plain function if it also supplies overlapping function fields. | [Messages schema][claude-messages] includes opaque redacted thinking and typed result content. Hosted [feature support][claude-cloud] limits which surfaces are available. | Add current request/response unions with discriminator dispatch, opaque replay preservation, multimodal tool results, and raw unknown blocks/events. Validate known malformed payloads rather than treating every decode failure as a future variant. Add a complete bounded Claude stream collector. |
| P1 / Claude multi-region routing | [`endpoint_for_region`](../src/config.rs) formats us/eu as ordinary regional hosts; selected Claude model names in [`location_for_model`](../src/client.rs) force global. A custom base URL provides an escape hatch but does not repair default routing or resource precedence. | [Official Vertex client][claude-vertex-source] maps us/eu to `aiplatform.us.rep.googleapis.com` / `aiplatform.eu.rep.googleapis.com`. [Cloud guide][claude-cloud] distinguishes those from regional/global endpoints. | Add explicit endpoint kinds and preserve chosen routing. Test us/eu/global/regional hosts and the resource location together. Do not infer required global routing just from a model-family name. |
| P2 / Claude count operation | Only [`claude_message` / `claude_stream`](../src/api/claude.rs) exist; generic countTokens is Gemini-shaped. | [Official Vertex client routing][claude-vertex-source] implements `/publishers/anthropic/models/count-tokens:rawPredict`; [counting guide][claude-count] describes accepted inputs and exclusions. | Add an endpoint-specific Claude counting request/response and allowlist conversion from messages. Keep model in the counting body, preserve Vertex anthropic_version, and reject excluded server tools/source kinds. Confirm project/model eligibility before claiming universal availability. |
| P2 / convenience and manual tool flow | [`execute_function_calling_flow`](../src/api/functions.rs) explicitly receives a callback and makes one follow-up request. It has no call/byte/deadline bounds or declaration/schema validation; it does not iterate further tool rounds. Claude has automatic cache_control fields but no typed per-content/tool manual breakpoints. | [Claude caching][claude-cache] distinguishes automatic and explicit cache breakpoints; existing callback behavior is a source observation, not a missing provider endpoint. | Add a separately named bounded runner with explicit callback registration, allowed-call validation, durable partial transcripts, and cancellation boundaries. Never automatically execute server tools or retry callbacks. Add current manual cache controls without claiming existing automatic caching is absent. |

## Proposed implementation sequence

1. **Repair shared transport and existing wire behavior.** Own status/error
   helpers, SSE byte framing and buffering, query encoding, credential error
   boundaries, existing serde key/discriminator fixes, and chat system-message
   projection. Preserve OAuth and feature flags. Define retry eligibility and a bounded total
   deadline, including Retry-After, across JSON/streaming/resource methods. Test
   on mock servers with no
   credentials. Small serializer fixes can preserve Rust signatures even when
   they intentionally repair the HTTP payload.
2. **Introduce current Gemini DTOs and replay.** Add current part envelopes,
   optional/blocked candidates, grounding unions, usage, generation, tool-call
   IDs, full resource references, and counting types. Route new generation/chat
   APIs through these DTOs. Preserve complete signed history. Keep legacy public
   enums/struct literals through explicit adapters or document a versioned
   migration where an adapter cannot be lossless.
3. **Make cache, embeddings, and discovery faithful.** Split cache creation and
   resources, require model selection, support only valid expiration updates,
   add bounded model-aware embedding batching and proper publisher DTOs, then
   add lazy pagination. Extend ADC through a maintained provider with explicit
   precedence/expiry/refresh tests. The auth owner must coordinate any new
   dependency and public error contract with the modernization integrator.
4. **Repair hosted Claude and add selected current operations.** Correct tool
   choice, effort placement, document citations, content unions, and endpoint
   routing; add structured stream collection and Claude count tokens. Preserve
   Vertex body/version/auth/beta handling. Treat extra_params as an escape hatch,
   not a substitute for correct typed fields or hosted-platform eligibility.
5. **Select optional expansions separately.** Current grounding providers,
   multimodal response controls, explicit tool runners, Live sessions, batch
   generation, or dedicated media APIs each need their own endpoint and model
   evidence. Preview discovery fields must retain their preview qualification.

Parallel implementation can use one transport/auth agent, one Gemini/cache
agent, and one hosted-Claude agent. Assign one integrator for shared exports,
public compatibility, Cargo/lock changes, release versioning, and cross-review.
Have a separate reviewer check the signed replay and credential boundaries.
The current task authorizes this plan as documentation, not its implementation.

## Regression acceptance criteria

- SSE fixtures split every byte of multibyte text and JSON, include multiple
  events with comments in one chunk, skipped events before EOF, mixed supported
  line endings, multiline data, malformed known events, and missing terminal
  signals. Enforce frame/response limits and test dropped-body cancellation.
- Gemini fixtures contain thought text plus signatures, signed parallel calls,
  multiple text parts, promptFeedback without candidates, a filtered candidate
  without content, provider finish reasons, nested grounding sources/spans, and
  cumulative usage updates including zero/absent values.
- Cache mocks inspect the exact model and input fields, return metadata without
  contents, return an empty list object, enforce the selected expiration mask,
  and exercise full-resource names, opaque encoded cursors, and paging cycles.
- Embedding mocks require task_type, verify code retrieval, and reject a
  multi-instance gemini-embedding-001 wire request. A batch helper preserves
  ordering and reports partial errors explicitly.
- Transport mocks return non-2xx success-shaped JSON and malformed error bodies.
  Auth fixtures cover supported ADC kinds, static-token expiry, concurrent
  refresh, rejected-token handling, non-Unicode environments, and bounded files;
  inspect formatted errors/logs for synthetic credential/customer markers.
- Claude fixtures inspect each tagged tool choice, output_config.effort without
  format, adaptive display, document citations, each citation location type,
  citations_delta, redacted thinking, unknown versus malformed known events,
  structured tool results, hosted count routing, and us/eu/global endpoints.
- Compile a downstream sample using old struct literals and exhaustive enums
  alongside the new APIs; run API compatibility checks against fetched main.
  Verify Cargo packaging includes new source modules/examples/docs and use the
  repository's existing release owner for any required version change.

## Scope boundaries and historical claims

Batch prediction/generation, Live/bidirectional sessions, tuning/deployment,
RAG/Vector Search/evaluation, dedicated image/video/music services, and a
synchronous facade remain separate unimplemented surfaces. Cloud API-key auth
is a valid [Vertex option][cloud-api-key], but it needs a distinct credential and
endpoint design; a Gemini Developer API key or Files API cannot be substituted.

Do not count Anthropic direct-API Files, Message Batches, Admin, code execution,
web fetch, MCP/Agent Skills, or other [unsupported hosted features][claude-cloud]
as mandatory Vertex gaps. Current web-search support does not establish support
for every server tool. Google compatibility APIs and newly discovered
Interactions/Responses surfaces need separate protocol and launch-stage audits.

The March 2026 [analysis](gap_analysis_mar_2026.md) and built-in model tables
are dated snapshots. They do not establish current model availability, preview
status, token limits, or endpoint eligibility. Short model IDs can already pass
through the SDK without a catalog update; missing catalog entries are a
convenience gap. Avoid guessed limits and inferred capability claims.

[discovery-v1]: https://aiplatform.googleapis.com/$discovery/rest?version=v1
[discovery-beta]: https://aiplatform.googleapis.com/$discovery/rest?version=v1beta1
[google-source]: https://github.com/googleapis/googleapis/tree/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1
[content-proto]: https://github.com/googleapis/googleapis/blob/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1/content.proto
[prediction-proto]: https://github.com/googleapis/googleapis/blob/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1/prediction_service.proto
[cache-proto]: https://github.com/googleapis/googleapis/blob/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1/cached_content.proto
[cache-service-proto]: https://github.com/googleapis/googleapis/blob/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1/gen_ai_cache_service.proto
[publisher-proto]: https://github.com/googleapis/googleapis/blob/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1/publisher_model.proto
[content-rest]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/reference/rest/v1/Content
[candidate-rest]: https://github.com/googleapis/googleapis/blob/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1/content.proto#L698
[response-rest]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/reference/rest/v1/GenerateContentResponse
[grounding-rest]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/reference/rest/v1/GroundingMetadata
[tool-rest]: https://github.com/googleapis/googleapis/blob/03a91044136a014466d4293eb1fe91f2b02075d2/google/cloud/aiplatform/v1/tool.proto
[thought-signatures]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/thought-signatures
[cache-create]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/context-cache/context-cache-create
[search-guide]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/grounding/grounding-with-google-search
[embeddings-guide]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/model-reference/text-embeddings-api
[adc-guide]: https://docs.cloud.google.com/docs/authentication/application-default-credentials
[cloud-api-key]: https://docs.cloud.google.com/vertex-ai/generative-ai/docs/start/api-keys
[sse-standard]: https://html.spec.whatwg.org/multipage/server-sent-events.html#event-stream-interpretation
[claude-vertex-source]: https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/lib/vertex/_client.py
[claude-choice-source]: https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/tool_choice_auto_param.py
[claude-thinking-source]: https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/thinking_config_adaptive_param.py
[claude-output-source]: https://github.com/anthropics/anthropic-sdk-python/blob/18f25547f20cf5f01da69ac611e700e3bc9ebf21/src/anthropic/types/output_config_param.py
[claude-cloud]: https://platform.claude.com/docs/en/build-with-claude/claude-on-vertex-ai
[claude-tools]: https://platform.claude.com/docs/en/agents-and-tools/tool-use/overview
[claude-thinking]: https://platform.claude.com/docs/en/build-with-claude/adaptive-thinking
[claude-citations]: https://platform.claude.com/docs/en/build-with-claude/citations
[claude-messages]: https://platform.claude.com/docs/en/api/messages/create
[claude-count]: https://platform.claude.com/docs/en/build-with-claude/token-counting
[claude-cache]: https://platform.claude.com/docs/en/build-with-claude/prompt-caching
