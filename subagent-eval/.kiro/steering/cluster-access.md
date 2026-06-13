---
inclusion: fileMatch
fileMatchPattern: ["infra/**", ".github/workflows/deploy.yml", ".github/actions/k8s-deploy/**"]
---

# Cluster Access

## Nodes

- **coreos** (control-plane): 192.168.88.112, Fedora CoreOS, user `core`
- **inference** (GPU worker): 192.168.88.115, Fedora CoreOS, user `core`, Intel Arc (xe driver)

## How to run commands

- **kubectl**: runs natively in PowerShell. `kubectl get pods -n realestate`
- **SSH to nodes**: `wsl ssh core@coreos.local "<cmd>"` or `wsl ssh core@192.168.88.115 "<cmd>"`. Always pass commands inline.
- **Inference service**: `vllm-inference.realestate.svc.cluster.local:8000` (ClusterIP). Port-forward for local testing: `kubectl port-forward svc/vllm-inference 8000:8000 -n realestate`
