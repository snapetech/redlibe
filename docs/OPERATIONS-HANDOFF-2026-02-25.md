# Operations Handoff (2026-02-25)

This documents the Redlibe, Open WebUI, Ollama, and OpenBao work completed in this session, including live cluster changes and follow-up operational guidance.

## Redlibe (`redlibe.home`)

### UX / UI / Media (live)

- Feed cards redesigned for better spacing, hierarchy, and responsive behavior
- Settings page theme picker with visual previews (42 total themes after palette expansion)
- Settings page JS moved to external file to satisfy CSP
- Top nav/search responsiveness and mobile sizing fixes
- HLS helper warning removed from feed cards; post-page helper made less alarming
- Video playback hardening (`playsinline`, safer HLS init/fallback handling)
- Fixed top nav overlap with post content
- Improved failure-page copy for upstream breakage

### Power-user UX (live)

- Command palette / "Power mode" (`Ctrl/Cmd+K`, `?`)
- Keyboard comment navigation/collapse (`j`, `k`, `[`, `]`, `Shift+C`, `Shift+E`)
- Local `Read later` + tagging (browser-local; no Reddit account required)
- Settings "Profiles" (browser-local bundles of subscriptions/filters/preferences)
- Settings export/import UX improvements
- `.env`-style settings export endpoint (`/settings/export-env`)

### Resilience / caching / observability (live)

- Anonymous Reddit JSON retries with exponential backoff + jitter
- Anonymous upstream circuit breaker (brownout protection)
- Response-shape validation for upstream JSON (clearer HTML/403/429 failures)
- Short-lived anonymous render cache for hot listing pages
- Request coalescing for identical in-flight anonymous upstream requests
- ETag / validator forwarding in media proxy
- Privacy-preserving in-memory upstream counters (no user identifiers)
- Prometheus `/metrics` endpoint
- Diagnostics page for upstream/circuit/cache state

### Operational endpoints (live)

- `https://redlibe.home/upstream-metrics.json`
  - JSON snapshot of upstream failures/successes, circuit status, coalescing counters, cache stats
- `https://redlibe.home/metrics`
  - Prometheus text metrics for upstream failures and cache hit/miss counts
- `https://redlibe.home/diagnostics/upstream`
  - Human-readable diagnostics page (circuit state, cache hit ratio, upstream error counts)

### Authentik notes (current state)

- `redlibe.home` was temporarily unprotected during Authentik outpost/provider hostname troubleshooting
- Root Authentik DB permission issue was fixed (`GRANT SELECT` on `public.authentik_core_groupancestry` to `authentik`)
- Separate Authentik outpost/provider mapping issue for `redlibe.home` was still the blocker when auth was bypassed for testing

## Open WebUI (`openwebui.home`)

### API key auth + routing (live)

- `ENABLE_API_KEYS=true` enabled on `deployment/openwebui`
- UI route remains Authentik-protected
- API routes are directly callable with `Authorization: Bearer <api_key>`

Direct agent/API access exposed:

- `/api/chat/completions`
- `/api/chat/completed`
- `/api/models`
- `/ollama/*`

OpenAI-compatible rewrites (Traefik):

- `/v1/chat/completions` -> `/api/chat/completions`
- `/v1/models` -> `/api/models`

### Verified API behavior (live)

- `POST /api/chat/completions` works for standard models and GLM alias models
- `POST /ollama/api/generate` works and returns Ollama timing/token metrics
- `POST /v1/chat/completions` works through rewrite
- `GET /v1/models` works through rewrite

### Open WebUI API key storage (OpenBao)

Generated Open WebUI admin API key is stored in OpenBao:

- `secret/openwebui`

Stored fields include:

- `api_key`
- `admin_email`
- `api_base_url`
- endpoint references (`chat_endpoint`, `openai_compat_chat`, `ollama_passthrough`)

Do not store bearer tokens in shell history or docs. Read from OpenBao only.

### Model presets / aliases (current state)

Created Open WebUI preset model entries (API-created) with AMD-safe defaults:

- `amd-glm47flash-ablit-fast`
- `amd-glm4-32b-balanced`
- `amd-glmz1-32b-reasoning`

Current behavior verified:

- Preset IDs are visible in `/api/models`
- `POST /api/chat/completions` with preset ID now resolves correctly to the backing Ollama model and returns `200`
- Raw Ollama alias ID also works on `/api/chat/completions`

## Ollama (AMD node / `9070 XT 16GB`)

### Pulled models (live on `ollama-amd`)

- `hf.co/bartowski/THUDM_GLM-4-32B-0414-GGUF:IQ2_M`
- `hf.co/bartowski/THUDM_GLM-Z1-32B-0414-GGUF:IQ2_M`
- `hf.co/mradermacher/Huihui-GLM-4.7-Flash-abliterated-i1-GGUF:i1-IQ3_M`

### Custom AMD-safe alias (created)

- `glm47flash-ablit-iq3m-9070xt:latest`

Alias defaults were set for a 16GB-friendly runtime profile:

- `num_ctx=4096`
- `num_predict=256`
- `temperature=0.2`
- `top_p=0.9`
- `repeat_penalty=1.05`

### Benchmarks (measured on AMD Ollama service)

Benchmark method:

- Endpoint: Ollama direct generate API (via `svc/ollama-amd`, same prompt/options)
- Typical options: `num_ctx=4096`, `num_predict=128`, `temperature=0.2`
- Metrics source: Ollama response (`total_duration`, `eval_duration`, `eval_count`)

All three test models fit fully in VRAM at `num_ctx=4096` (`size_vram == size` in `/api/ps`).

#### `glm47flash-ablit-iq3m-9070xt:latest`

- ~14.14 GB loaded (`size_vram == size`)
- Warm load: ~4.47s
- Throughput: ~31.9 tok/s (p50)
- Total latency: ~4.20s (p50 for 128 generated tokens)

#### `hf.co/bartowski/THUDM_GLM-4-32B-0414-GGUF:IQ2_M`

- ~13.17 GB loaded (`size_vram == size`)
- Warm load: ~5.20s
- Throughput: ~22.75 tok/s (p50)
- Total latency: ~5.82s (p50)

#### `hf.co/bartowski/THUDM_GLM-Z1-32B-0414-GGUF:IQ2_M`

- ~13.17 GB loaded (`size_vram == size`)
- Warm load: ~5.00s
- Throughput: ~22.74 tok/s (p50)
- Total latency: ~5.82s (p50)

### 9070 XT recommendation summary

- `glm47flash-ablit-iq3m-9070xt:latest` is the fastest tested option on the current AMD node while still fitting fully in VRAM
- GLM 32B `IQ2_M` models also fit and are stable, but slower
- For GLM-4.7 Flash family on 16GB VRAM, prefer `IQ3`/`Q3`/`IQ2`-class quants and keep `num_ctx` modest (`4k` or `8k`)
- Avoid assuming very large advertised contexts are practical on 16GB due KV cache growth

## OpenBao (`openbao`)

### HA state (restored and verified)

OpenBao server resources had disappeared while services remained. Re-applying the Helm manifest restored:

- `StatefulSet/openbao`
- `Deployment/openbao-agent-injector`

Current HA raft state:

- `openbao-0`: leader (active)
- `openbao-1`: follower (standby)

Ingress:

- `http://openbao.home` reachable and redirects to `/ui/`

### Unseal / recovery notes

The cluster is Shamir-sealed and uses manual unseal after restart.

- Init output file (contains unseal keys + root token): `../k3s/openbao/openbao-init-output.txt`
- Threshold: 3 of 5 unseal keys

Example:

```bash
kubectl -n openbao exec openbao-0 -- bao operator unseal "<key1>"
kubectl -n openbao exec openbao-0 -- bao operator unseal "<key2>"
kubectl -n openbao exec openbao-0 -- bao operator unseal "<key3>"
```

### Secret population (completed)

Synced standard cluster secrets via:

- `../k3s/scripts/sync-secrets-to-openbao.sh`

Confirmed paths updated:

- `secret/authentik`
- `secret/gitlab`
- `secret/velero/minio`
- `secret/proton`
- `secret/cloudflare`
- `secret/iptv` (if `XTREAM_*` vars exist locally)
- `secret/openwebui` (Open WebUI API key + metadata)

### Least-privilege policies/tokens for Open WebUI (completed)

Policies created:

- `openwebui-kv-ro`
- `openwebui-kv-rw`

Tokens created and stored in Kubernetes (token values not printed):

- `openwebui/openbao-token-openwebui-ro`
- `openwebui/openbao-token-openwebui-rw`

Token metadata path in OpenBao (no token values):

- `secret/openwebui_token_meta`

Scope validation performed:

- RO token can read `secret/openwebui`
- RO token denied on unrelated paths (example: `secret/gitlab`)
- RW token can read/write `secret/openwebui`
- RW token denied on unrelated paths

### External Secrets Operator (ESO) integration (repaired + live)

ESO was partially installed (missing controller/CRDs). It was repaired via Helm and now syncs OpenBao secrets into Kubernetes.

OpenWebUI namespace resources created:

- `SecretStore/openbao-openwebui-ro`
- `ExternalSecret/openwebui-api-from-openbao`

Synced Kubernetes secret:

- `openwebui/openwebui-api-auth`

Synced fields include:

- `OPENWEBUI_API_KEY`
- `OPENWEBUI_ADMIN_EMAIL`
- `OPENWEBUI_API_BASE_URL`

## Scripts added in this repo (operator tooling)

### `scripts/bench_openwebui.py`

Purpose:

- Bench Open WebUI chat endpoint (`/api/chat/completions`)
- Bench Ollama passthrough via Open WebUI (`/ollama/api/generate`) and report token/sec using returned timings

Inputs:

- `OWUI_API_KEY` env var (or `--api-key`)
- `OWUI_BASE_URL` env var (or `--base-url`)

Examples:

```bash
# Pull key from OpenBao (example only; requires valid BAO_TOKEN)
export BAO_ADDR=http://openbao.home
export BAO_TOKEN=<token>
export OWUI_API_KEY="$(bao kv get -field=api_key secret/openwebui)"
export OWUI_BASE_URL="http://openwebui.home"

# Raw model perf through Open WebUI's Ollama proxy (recommended for throughput)
python3 scripts/bench_openwebui.py \
  --mode ollama \
  --model glm47flash-ablit-iq3m-9070xt:latest \
  --runs 8 --warmup 1 \
  --options-json '{"num_ctx":4096,"num_predict":128,"temperature":0.2}'

# End-to-end Open WebUI chat path (stream=false)
python3 scripts/bench_openwebui.py \
  --mode chat \
  --model amd-glm47flash-ablit-fast \
  --runs 5 --warmup 1

# Chat path with TTFT measurement (stream=true)
python3 scripts/bench_openwebui.py \
  --mode chat --stream \
  --model amd-glm47flash-ablit-fast \
  --runs 5 --warmup 1
```

## Retrieval examples (no secret values)

```bash
# OpenBao read (root/admin token or scoped token with matching policy)
export BAO_ADDR=http://openbao.home
export BAO_TOKEN=<token>
bao kv get secret/openwebui

# Open WebUI API call through OpenAI-compat route
curl -sS http://openwebui.home/v1/chat/completions \
  -H "Authorization: Bearer <api_key>" \
  -H "Content-Type: application/json" \
  -d '{"model":"glm47flash-ablit-iq3m-9070xt:latest","messages":[{"role":"user","content":"Reply with OK."}],"stream":false}'
```

## Remaining follow-up work (not completed here)

- Formal versioned Reddit upstream adapter abstraction + fixture replay contract tests
- Redlib request logging/metrics expansion (opt-in structured logs) if needed beyond current privacy-preserving counters
- Redlib public-instance hardening profile docs + CSP regression tests
- Open WebUI model preset/default behavior hardening if future versions regress preset resolution again
