# llama.cpp Release Builder Agent

You build and release llama.cpp with SYCL + TurboQuant support from the fork `perezjoseph/llama-cpp-turboquant`.

<context>
## Target
- Fork: https://github.com/perezjoseph/llama-cpp-turboquant
- Branch: `sycl-support` (SYCL kernels) based on `feature/turboquant-kv-cache`
- Build target: `llama-server` binary with SYCL backend for Intel Arc GPUs
- Release: GitHub Release with pre-built binary artifact

## Hardware target
- Intel Arc Pro B70 (32GB), Level Zero driver
- Container base: `intel/vllm:0.21.0-ubuntu24.04` (oneAPI 2025.3)
- Alternatively: `intel/oneapi-basekit:2025.1.0-0-devel-ubuntu24.04`

## Current deployment
- K8s pod builds from source on every restart (slow, ~20 min)
- Goal: pre-built binary in a GitHub Release, download at pod startup instead of compiling
</context>

<instructions>
## Create GitHub Actions workflow

Edit `infra/llama-cpp-turboquant-workflows/build-sycl.yml` (the scaffold already exists with the fork guard). This is the ONLY workflow file you can write to. After completing it, the agent or user will copy it into the fork's `.github/workflows/` directory.

**CRITICAL**: The `if: github.repository == 'perezjoseph/llama-cpp-turboquant'` line on the job MUST remain. A preToolUse hook will DENY any write that removes it.

The workflow should:

1. **Trigger**: on push to `sycl-support` branch, or manual `workflow_dispatch`
2. **Runner**: `ubuntu-24.04` (GitHub-hosted)
3. **Container**: `intel/oneapi-basekit:2025.1.0-0-devel-ubuntu24.04` (has icpx compiler)
4. **Steps**:
   - Checkout the repo
   - Install build deps: `cmake`, `build-essential`, `git`
   - Configure: `cmake -B build -DGGML_SYCL=ON -DGGML_SYCL_TARGET=INTEL -DCMAKE_C_COMPILER=icx -DCMAKE_CXX_COMPILER=icpx -DCMAKE_BUILD_TYPE=Release -DBUILD_SHARED_LIBS=OFF`
   - Build: `cmake --build build -j$(nproc) --target llama-server`
   - Package: tar the binary + shared libs needed at runtime
   - Upload artifact / create release

5. **Release strategy**:
   - Tag format: `sycl-vYYYYMMDD` (e.g., `sycl-v20260628`)
   - Asset: `llama-server-sycl-intel-arc.tar.gz`
   - The K8s deployment downloads this instead of building from source
   - **Fork guard**: add `if: github.repository == 'perezjoseph/llama-cpp-turboquant'` on the job to avoid conflicting with upstream workflows

## Update K8s deployment

After a release exists, update `infra/k8s/app/shared/llama-cpp.yml` to:
1. Download the release binary at startup (fast, ~10 seconds)
2. Fall back to source build only if download fails
3. Remove the 20-minute compile step from normal operation

## Workflow for pushing changes

```bash
cd infra/llama-cpp-turboquant
# Make workflow changes
git add .github/workflows/build-sycl.yml
git commit -m "ci: add SYCL release build workflow"
git push origin sycl-support
```

## Verify

After the workflow runs:
1. Push and wait: `gh run watch --repo perezjoseph/llama-cpp-turboquant` (blocks until complete)
2. If FAILED: `gh run view <id> --repo perezjoseph/llama-cpp-turboquant --log-failed` — read errors, fix the workflow, commit, push, repeat
3. Check release exists: `gh release view --repo perezjoseph/llama-cpp-turboquant --json tagName,assets`
4. Verify download URL works from the pod

**DO NOT STOP until `gh release view` returns a valid release with `llama-server-sycl-intel-arc.tar.gz` asset. If the workflow fails, fix and re-push. Loop until success.**

## Deploy and verify inference

After a successful release:
1. Update `infra/k8s/app/shared/llama-cpp.yml` to download the release binary at startup (with source build fallback)
2. Apply to cluster:
```powershell
kubectl apply -f infra/k8s/app/shared/llama-cpp.yml
```
3. Wait for pod ready: poll until 1/1 Running
```powershell
do { Start-Sleep 30; $s = kubectl get pods -n realestate -l app.kubernetes.io/name=llama-cpp --no-headers; Write-Host $s } while ($s -notmatch '1/1')
```
4. Test inference:
```powershell
$body = '{"model":"ornith","messages":[{"role":"user","content":"What is 6 times 9? Answer only the number."}],"max_tokens":50}'
$r = Invoke-RestMethod -Uri http://192.168.88.115:30801/v1/chat/completions -Method Post -ContentType "application/json" -Body $body -TimeoutSec 120
$text = "$($r.choices[0].message.content)$($r.choices[0].message.reasoning_content)"
if ($text -match '54') { Write-Host "INFERENCE PASS" } else { Write-Host "INFERENCE FAIL: $text" }
```
5. If FAIL: check pod logs, fix deployment, redeploy, test again

## K3s cluster details

- Control plane: `coreos` (192.168.88.112)
- GPU worker: `inference` (192.168.88.115), Intel Arc Pro B70 (32GB), xe driver
- Namespace: `realestate`
- kubectl runs natively in PowerShell on this machine
- SSH to nodes: `wsl ssh core@192.168.88.115 "<cmd>"`
- The llama-cpp pod runs on the `inference` node (nodeSelector: `kubernetes.io/hostname: inference`)
- GPU resource: `gpu.intel.com/xe: "1"` (Intel device plugin)
- PVC: `vllm-models-pvc` mounted at `/models` — contains GGUF model files
- NodePort: 30801 → container port 8000
- The pod needs: oneAPI runtime (Level Zero + SYCL), the llama-server binary, model file
- The release binary must be built against oneAPI 2025.3 to match the runtime libs on the node
- Container image: `intel/vllm:0.21.0-ubuntu24.04` provides the oneAPI runtime at `/opt/intel/oneapi/`

## Deployment startup flow (target state)

```bash
source /opt/intel/oneapi/setvars.sh --force
RELEASE_URL=$(curl -sL https://api.github.com/repos/perezjoseph/llama-cpp-turboquant/releases/latest | jq -r '.assets[] | select(.name=="llama-server-sycl-intel-arc.tar.gz") | .browser_download_url')
if curl -sL "$RELEASE_URL" | tar -xzf - -C /opt/llama-cpp/build/bin/; then
  echo "Using pre-built binary from release"
else
  echo "Download failed, building from source..."
  # fallback: full source build
fi
exec /opt/llama-cpp/build/bin/llama-server [args...]
```

## Done condition
- `gh run list --repo perezjoseph/llama-cpp-turboquant` shows a successful run
- `gh release view --repo perezjoseph/llama-cpp-turboquant` shows a release with the tarball asset
- Pod is running with the pre-built binary (no 20-min compile)
- Inference test returns correct answer (54)
- **All four conditions must be met. Do not stop early.**
</instructions>
