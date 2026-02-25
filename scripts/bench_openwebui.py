#!/usr/bin/env python3
import argparse
import json
import os
import statistics as stats
import sys
import time

import requests


def ns_to_s(value):
    if isinstance(value, (int, float)):
        return value / 1e9
    return None


def percentile(values, p):
    if not values:
        return None
    if len(values) == 1:
        return values[0]
    ordered = sorted(values)
    idx = int(round((p / 100.0) * (len(ordered) - 1)))
    return ordered[idx]


def make_headers(api_key):
    return {
        "Authorization": f"Bearer {api_key}",
        "Content-Type": "application/json",
    }


def ollama_generate(base_url, api_key, model, prompt, options, keep_alive, timeout):
    payload = {
        "model": model,
        "prompt": prompt,
        "stream": False,
        "keep_alive": keep_alive,
    }
    if options:
        payload["options"] = options
    r = requests.post(
        f"{base_url.rstrip('/')}/ollama/api/generate",
        headers=make_headers(api_key),
        json=payload,
        timeout=timeout,
    )
    r.raise_for_status()
    return r.json()


def chat_completion(base_url, api_key, model, prompt, timeout, stream):
    payload = {
        "model": model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": stream,
    }
    url = f"{base_url.rstrip('/')}/api/chat/completions"
    t0 = time.perf_counter()
    if not stream:
        r = requests.post(url, headers=make_headers(api_key), json=payload, timeout=timeout)
        r.raise_for_status()
        body = r.json()
        return {
            "wall_s": time.perf_counter() - t0,
            "ttft_s": None,
            "response": body,
        }

    with requests.post(
        url,
        headers=make_headers(api_key),
        json=payload,
        timeout=timeout,
        stream=True,
    ) as r:
        r.raise_for_status()
        ttft_s = None
        bytes_seen = 0
        for line in r.iter_lines(decode_unicode=True):
            if line is None:
                continue
            if not line:
                continue
            bytes_seen += len(line)
            if ttft_s is None and line.startswith("data:"):
                ttft_s = time.perf_counter() - t0
        return {
            "wall_s": time.perf_counter() - t0,
            "ttft_s": ttft_s,
            "response": {"stream_bytes": bytes_seen},
        }


def run_ollama_mode(args):
    for _ in range(args.warmup):
        ollama_generate(
            args.base_url,
            args.api_key,
            args.model,
            args.prompt,
            args.options,
            args.keep_alive,
            args.timeout,
        )

    totals = []
    walls = []
    eval_s_list = []
    tps = []
    load_s = []
    eval_counts = []
    last = None
    for _ in range(args.runs):
        t0 = time.perf_counter()
        j = ollama_generate(
            args.base_url,
            args.api_key,
            args.model,
            args.prompt,
            args.options,
            args.keep_alive,
            args.timeout,
        )
        wall = time.perf_counter() - t0
        last = j
        total_s = ns_to_s(j.get("total_duration")) or wall
        eval_s = ns_to_s(j.get("eval_duration"))
        load_duration_s = ns_to_s(j.get("load_duration"))
        eval_count = j.get("eval_count")
        walls.append(wall)
        totals.append(total_s)
        if eval_s is not None:
            eval_s_list.append(eval_s)
        if load_duration_s is not None:
            load_s.append(load_duration_s)
        if isinstance(eval_count, int):
            eval_counts.append(eval_count)
        if eval_s and eval_count:
            tps.append(eval_count / eval_s)

    out = {
        "mode": "ollama",
        "base_url": args.base_url,
        "model": args.model,
        "runs": args.runs,
        "warmup": args.warmup,
        "options": args.options,
        "keep_alive": args.keep_alive,
        "wall_s": {
            "p50": stats.median(walls),
            "p95": percentile(walls, 95),
        },
        "total_s": {
            "p50": stats.median(totals),
            "p95": percentile(totals, 95),
        },
        "tok_s": {
            "p50": stats.median(tps) if tps else None,
            "p95": percentile(tps, 95) if tps else None,
        },
        "eval_count": {
            "p50": stats.median(eval_counts) if eval_counts else None,
        },
        "load_duration_s": {
            "p50": stats.median(load_s) if load_s else None,
        },
    }
    if args.include_last and last is not None:
        out["last_response"] = last
    return out


def run_chat_mode(args):
    for _ in range(args.warmup):
        chat_completion(
            args.base_url,
            args.api_key,
            args.model,
            args.prompt,
            args.timeout,
            args.stream,
        )

    walls = []
    ttfts = []
    last = None
    for _ in range(args.runs):
        result = chat_completion(
            args.base_url,
            args.api_key,
            args.model,
            args.prompt,
            args.timeout,
            args.stream,
        )
        last = result["response"]
        walls.append(result["wall_s"])
        if result["ttft_s"] is not None:
            ttfts.append(result["ttft_s"])

    out = {
        "mode": "chat",
        "stream": args.stream,
        "base_url": args.base_url,
        "model": args.model,
        "runs": args.runs,
        "warmup": args.warmup,
        "wall_s": {
            "p50": stats.median(walls),
            "p95": percentile(walls, 95),
        },
        "ttft_s": {
            "p50": stats.median(ttfts) if ttfts else None,
            "p95": percentile(ttfts, 95) if ttfts else None,
        },
    }
    if args.include_last and last is not None:
        out["last_response"] = last
    return out


def parse_args():
    p = argparse.ArgumentParser(description="Benchmark Open WebUI chat or Ollama passthrough via Open WebUI.")
    p.add_argument("--base-url", default=os.getenv("OWUI_BASE_URL", "http://openwebui.home"))
    p.add_argument("--api-key", default=os.getenv("OWUI_API_KEY"))
    p.add_argument("--mode", choices=["chat", "ollama"], default="ollama")
    p.add_argument("--model", required=True)
    p.add_argument("--prompt", default="Explain TCP in one paragraph.")
    p.add_argument("--runs", type=int, default=8)
    p.add_argument("--warmup", type=int, default=1)
    p.add_argument("--timeout", type=int, default=600)
    p.add_argument("--include-last", action="store_true", help="Include last API response in output JSON.")
    p.add_argument("--stream", action="store_true", help="Chat mode only: use streaming and measure TTFT.")
    p.add_argument("--keep-alive", type=int, default=-1, help="Ollama mode only.")
    p.add_argument(
        "--options-json",
        default=os.getenv("OWUI_OLLAMA_OPTIONS_JSON"),
        help='Ollama mode only. JSON object, e.g. \'{"num_ctx":4096,"num_predict":128,"temperature":0.2}\'',
    )
    args = p.parse_args()
    if not args.api_key:
        p.error("Missing API key. Set --api-key or OWUI_API_KEY.")
    args.options = None
    if args.options_json:
        try:
            parsed = json.loads(args.options_json)
        except json.JSONDecodeError as e:
            p.error(f"Invalid --options-json: {e}")
        if not isinstance(parsed, dict):
            p.error("--options-json must decode to a JSON object")
        args.options = parsed
    return args


def main():
    args = parse_args()
    try:
        if args.mode == "ollama":
            result = run_ollama_mode(args)
        else:
            result = run_chat_mode(args)
    except requests.HTTPError as e:
        body = None
        try:
            body = e.response.text
        except Exception:
            pass
        print(
            json.dumps(
                {
                    "error": "http_error",
                    "status": e.response.status_code if e.response is not None else None,
                    "body": body,
                },
                indent=2,
            ),
            file=sys.stderr,
        )
        sys.exit(1)
    except requests.RequestException as e:
        print(json.dumps({"error": "request_error", "detail": str(e)}, indent=2), file=sys.stderr)
        sys.exit(1)

    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
