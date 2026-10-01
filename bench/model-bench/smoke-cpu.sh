#!/usr/bin/env bash
# CPU-only smoke test of the harness (no GPU). Small subset, at most 8 threads.
set -u
B=C:/Users/sethy/Projects/vault/target/release/model-bench.exe
M=C:/Users/sethy/Projects/vault-models
CPU=$M/llama.cpp/llama-b11310-cpu/llama-server.exe
D=C:/Users/sethy/Projects/vault-eval/external/bright/subset-smoke
OUT=C:/Users/sethy/Projects/vault/docs/phase0/model-bench/smoke
mkdir -p "$OUT"
common=(--server-bin "$CPU" --build b11310 --backend cpu --results "$OUT/results.jsonl" --log-dir "$OUT" \
  --dataset "$D" --splits psychology --np 2 --slot-ctx 1024 --ubatch 2048 --ngl 0 --threads 8 \
  --ready-timeout-s 300 --server-arg=--no-webui)
if [ "${1:-all}" != "extras" ]; then
$B embed "${common[@]}" --config-id smoke-embed-qwen3-0.6b-q8_0-cpu --model "$M/embedders/Qwen__Qwen3-Embedding-0.6B-GGUF__Qwen3-Embedding-0.6B-Q8_0.gguf" \
  --model-name qwen3-embedding-0.6b --quant Q8_0 --pooling last \
  --embed-batch 8 --concurrency 2 --autotune --save-candidates "$OUT/candidates.json" || exit $?
$B rerank "${common[@]}" --config-id smoke-rerank-qwen3-0.6b-q8_0-cpu --model "$M/rerankers/ggml-org__Qwen3-Reranker-0.6B-Q8_0-GGUF__qwen3-reranker-0.6b-q8_0.gguf" \
  --model-name qwen3-reranker-0.6b --quant Q8_0 --pooling rank \
  --candidates "$OUT/candidates.json" --concurrency 1
echo "rerank exit: $?"
fi
# BGE-M3 load/embed check (cls pooling, raw queries) and the parity code path (CPU vs the same CPU build).
$B embed "${common[@]}" --config-id smoke-embed-bge-m3-q8_0-cpu --model "$M/embedders/ggml-org__bge-m3-Q8_0-GGUF__bge-m3-q8_0.gguf"   --model-name bge-m3 --quant Q8_0 --pooling cls --query-format none --embed-batch 8 --concurrency 2 || exit $?
$B parity "${common[@]}" --config-id smoke-parity-qwen3-0.6b-q8_0-cpu-vs-cpu --model "$M/embedders/Qwen__Qwen3-Embedding-0.6B-GGUF__Qwen3-Embedding-0.6B-Q8_0.gguf"   --model-name qwen3-embedding-0.6b --quant Q8_0 --pooling last --server-bin-b "$CPU" --backend-b cpu --build-b b11310 --n 30 --embed-batch 8 --concurrency 2
echo "parity exit: $?"
