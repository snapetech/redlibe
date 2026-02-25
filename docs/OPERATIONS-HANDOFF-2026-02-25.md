# Operations Handoff (2026-02-25)

This documents the Redlibe, Open WebUI, and OpenBao operational work completed in this session.

## Redlibe (`redlibe.home`)

### UX / media / power-user features (live)

- Command palette / "Power mode" (`Ctrl/Cmd+K`, `?`)
- Keyboard comment navigation/collapse (`j`, `k`, `[`, `]`, `Shift+C`, `Shift+E`)
- Local `Read later` + tagging (browser-local; no Reddit account required)
- Settings profiles (browser-local saved bundles of prefs/subscriptions/filters)
- Settings export/import UX improvements, including `.env`-style export
- Media UX hardening (`playsinline`, HLS helper UX cleanup, fallback hints)
- Accessibility baseline improvements (skip link, focus states)
- Humane upstream failure pages (retry / alternate routes / open original)

### Resilience / caching / observability (live)

- Anonymous Reddit JSON retries with exponential backoff + jitter
- Anonymous upstream circuit breaker (brownout protection)
- Response-shape validation improvements (clearer parse failures)
- Short-lived anonymous render cache for hot listing pages
- ETag / validator forwarding in media proxy
- Request coalescing for identical in-flight anonymous JSON requests
- Privacy-preserving upstream counters (no user identifiers)

### New operational endpoints (live)

- `https://redlibe.home/upstream-metrics.json`
  - JSON snapshot of upstream failures/successes, circuit state, coalescing counters, OAuth backend/rate-limit state
- `https://redlibe.home/metrics`
  - Prometheus text-format metrics (upstream + render cache)
- `https://redlibe.home/diagnostics/upstream`
  - Minimal diagnostics page (token refresh/upstream health/cache hit ratio)

## Open WebUI (`openwebui.home`)

### API key auth (enabled)

- Deployment env updated: `ENABLE_API_KEYS=true`
- UI remains Authentik-protected
- API paths can be called directly with `Authorization: Bearer <key>`

### API routing (live)

Direct agent/API access is enabled for:

- `/api/chat/completions`
- `/api/chat/completed`
- `/api/models`
- `/ollama/*`

OpenAI-compat rewrites added:

- `/v1/chat/completions` -> `/api/chat/completions`
- `/v1/models` -> `/api/models`

### Verified endpoints

- Open WebUI path (UI-like pipeline): `POST /api/chat/completions`
- Ollama passthrough (raw perf + metrics): `POST /ollama/api/generate`
- OpenAI-compat alias: `POST /v1/chat/completions`

### Open WebUI API key storage

Generated Open WebUI admin API key is stored in OpenBao at:

- `secret/openwebui`

Stored fields include:

- `api_key`
- `admin_email`
- `api_base_url`
- endpoint references (`chat_endpoint`, `openai_compat_chat`, `ollama_passthrough`)

Do not keep the key in shell history or docs. Read it from OpenBao when needed.

## OpenBao (`openbao`)

### HA state (restored)

OpenBao server resources had disappeared from the cluster while services remained. The Helm manifest was re-applied, restoring:

- `StatefulSet/openbao`
- `Deployment/openbao-agent-injector`

Current HA state:

- `openbao-0`: raft leader (active)
- `openbao-1`: raft follower (standby)

### Ingress

- Applied `../k3s/openbao/ingress.yaml`
- `http://openbao.home` is reachable and redirects to `/ui/`

### Unseal / recovery notes

The cluster is Shamir-sealed and uses manual unseal after restart.

- Init output file (contains unseal keys + root token): `../k3s/openbao/openbao-init-output.txt`
- Threshold: 3 of 5 unseal keys

Unseal commands (repeat for each pod if sealed):

```bash
kubectl -n openbao exec openbao-0 -- bao operator unseal "<key1>"
kubectl -n openbao exec openbao-0 -- bao operator unseal "<key2>"
kubectl -n openbao exec openbao-0 -- bao operator unseal "<key3>"
```

### Secret population (completed)

Synced standard cluster secrets via:

- `../k3s/scripts/sync-secrets-to-openbao.sh`

Confirmed secret paths updated:

- `secret/authentik`
- `secret/gitlab`
- `secret/velero/minio`
- `secret/proton`
- `secret/cloudflare`
- `secret/iptv` (if `XTREAM_*` vars exist locally)

Added session-generated Open WebUI API key path:

- `secret/openwebui`

## GLM 32B model work (Ollama)

Target quant families validated on Hugging Face (bartowski GGUF repos):

- `bartowski/THUDM_GLM-4-32B-0414-GGUF`
- `bartowski/THUDM_GLM-Z1-32B-0414-GGUF`

Confirmed low-VRAM quant files exist:

- `IQ2_XS`, `IQ2_S`, `IQ2_M`, `Q2_K`, `Q2_K_L`

Pulls started on `ollama-amd` for:

- `hf.co/bartowski/THUDM_GLM-4-32B-0414-GGUF:IQ2_M`
- `hf.co/bartowski/THUDM_GLM-Z1-32B-0414-GGUF:IQ2_M`

These may still be in progress depending on when this file is read.

## Retrieval examples (no secret values)

```bash
# OpenBao (root/admin token required)
export BAO_ADDR=http://openbao.home
export BAO_TOKEN=<token>
bao kv get secret/openwebui

# Open WebUI API call (use api_key from OpenBao)
curl -sS https://openwebui.home/v1/chat/completions \
  -H "Authorization: Bearer <api_key>" \
  -H "Content-Type: application/json" \
  -d '{"model":"qwen3:1.7b","messages":[{"role":"user","content":"Reply with OK."}],"stream":false}'
```
