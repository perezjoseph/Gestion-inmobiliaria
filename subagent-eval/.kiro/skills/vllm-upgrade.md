# vLLM Upgrade Procedure

## When to Use

When upgrading the `intel/llm-scaler-vllm` image tag in `infra/k8s/app/shared/vllm.yml`.

## Steps

1. Check [intel/llm-scaler-vllm tags on Docker Hub](https://hub.docker.com/r/intel/llm-scaler-vllm/tags) for the latest stable tag.
2. Verify compatibility with the current model (`Qwen/Qwen3-Coder-30B-A3B-Instruct`) and quantization settings (`sym_int4`).
3. Update the image tag in `infra/k8s/app/shared/vllm.yml`.
4. Apply and restart:
   ```sh
   kubectl apply -f infra/k8s/app/shared/vllm.yml
   kubectl rollout restart deployment/vllm -n realestate
   kubectl rollout status deployment/vllm -n realestate --timeout=600s
   ```
5. Test startup via port-forward and hit `/health`:
   ```sh
   kubectl port-forward svc/vllm-inference 8000:8000 -n realestate
   curl http://localhost:8000/health
   ```
6. Verify pod logs show model loaded successfully with sym_int4 quantization.
7. Commit the manifest change only after health check passes.
