#!/usr/bin/env bash
# Removes orphaned IngressRoute CRDs that reference non-existent services.
# These cause repeating Traefik errors:
#   - "kubernetes service not found: stalwart/stalwart"
#   - "middleware mail-redirect-https@kubernetescrd does not exist"
#
# Run once: bash infra/k8s/cleanup-orphaned-ingress.sh

set -euo pipefail

echo "=== Cleaning up orphaned Stalwart IngressRoutes ==="

# Delete stalwart IngressRoutes if they exist
if kubectl get ingressroute -n stalwart stalwart-http 2>/dev/null; then
  kubectl delete ingressroute -n stalwart stalwart-http
  echo "Deleted stalwart-http IngressRoute"
else
  echo "stalwart-http IngressRoute not found (already clean)"
fi

if kubectl get ingressroute -n stalwart stalwart-https 2>/dev/null; then
  kubectl delete ingressroute -n stalwart stalwart-https
  echo "Deleted stalwart-https IngressRoute"
else
  echo "stalwart-https IngressRoute not found (already clean)"
fi

echo ""
echo "=== Cleaning up orphaned Mail/Roundcube IngressRoutes ==="

# Find and delete mail-related IngressRoutes (the middleware error references mail-roundcube-http)
if kubectl get ingressroute --all-namespaces -o name 2>/dev/null | grep -qi roundcube; then
  kubectl get ingressroute --all-namespaces -o custom-columns='NAMESPACE:.metadata.namespace,NAME:.metadata.name' --no-headers | \
    grep -i roundcube | while read -r ns name; do
      kubectl delete ingressroute -n "$ns" "$name"
      echo "Deleted $name IngressRoute in namespace $ns"
    done
else
  echo "No roundcube IngressRoutes found (already clean)"
fi

echo ""
echo "=== Cleaning up orphaned Middleware ==="

# Delete the missing middleware if it exists as a CRD resource
if kubectl get middleware -n stalwart stalwart-redirect-https 2>/dev/null; then
  kubectl delete middleware -n stalwart stalwart-redirect-https
  echo "Deleted stalwart-redirect-https Middleware"
fi

if kubectl get middleware --all-namespaces -o name 2>/dev/null | grep -qi mail-redirect; then
  kubectl get middleware --all-namespaces -o custom-columns='NAMESPACE:.metadata.namespace,NAME:.metadata.name' --no-headers | \
    grep -i mail-redirect | while read -r ns name; do
      kubectl delete middleware -n "$ns" "$name"
      echo "Deleted $name Middleware in namespace $ns"
    done
fi

echo ""
echo "=== Optionally remove empty stalwart namespace ==="
if kubectl get namespace stalwart 2>/dev/null; then
  # Check if namespace is empty
  resources=$(kubectl api-resources --verbs=list --namespaced -o name | \
    xargs -I {} kubectl get {} -n stalwart --no-headers 2>/dev/null | wc -l)
  if [ "$resources" -eq 0 ]; then
    kubectl delete namespace stalwart
    echo "Deleted empty stalwart namespace"
  else
    echo "stalwart namespace still has resources, skipping deletion"
  fi
else
  echo "stalwart namespace does not exist (already clean)"
fi

echo ""
echo "Done. Traefik will pick up the changes within a few seconds."
