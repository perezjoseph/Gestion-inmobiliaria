$ErrorActionPreference = 'Stop'
$pod = (kubectl get pods -n realestate -l app.kubernetes.io/name=backend -o jsonpath='{.items[0].metadata.name}')
$body = '{"model":"Qwen3-30B-A3B-Instruct-2507-int4-ov","messages":[{"role":"system","content":"Eres asistente del bot de bienes raices."},{"role":"user","content":"Si el inquilino no paga el dia 5, ¿qué pasa? Responde en una oración."}],"max_tokens":80,"temperature":0.2,"stream":false}'
$body | kubectl exec -i -n realestate $pod -- sh -c 'cat > /tmp/req.json'
Write-Host '--- ovms service reachable from backend pod ---'
kubectl exec -n realestate $pod -- curl -s -o /dev/null -w 'HTTP %{http_code} time %{time_total}s len %{size_download}b' -X POST 'http://ovms.realestate.svc.cluster.local:8000/v3/chat/completions' -H 'Content-Type: application/json' --data-binary '@/tmp/req.json'
Write-Host ''
Write-Host '--- response body ---'
kubectl exec -n realestate $pod -- curl -s -X POST 'http://ovms.realestate.svc.cluster.local:8000/v3/chat/completions' -H 'Content-Type: application/json' --data-binary '@/tmp/req.json'
