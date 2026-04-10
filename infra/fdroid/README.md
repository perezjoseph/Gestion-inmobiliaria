# F-Droid Private Repo — PropManager Dev Builds

Self-hosted F-Droid repository for distributing dev builds to your phone with auto-updates.

## Architecture

```
GitHub Actions CI (on push to main, android/** changes)
  → builds release APK
  → runs fdroid update to generate signed repo index
  → copies repo files to K8s PVC

Kubernetes (realestate namespace):
  → nginx pod serves the F-Droid repo at http://fdroid.local/repo

Phone:
  → F-Droid client with custom repo added
  → auto-updates silently on Android 12+
```

## Setup

### 1. Create the repo signing keystore secret

Generate a password and store it as a GitHub Actions secret:

```bash
# Generate a strong password
openssl rand -base64 32

# Add as GitHub secret: FDROID_KEYSTORE_PASS
```

The CI workflow will generate the keystore on first run. For subsequent runs,
you'll need to persist the keystore (see "Keystore Persistence" below).

### 2. Deploy the K8s resources

```bash
kubectl apply -k infra/k8s/app/
```

### 3. DNS setup

Add `fdroid.local` to your local DNS (or `/etc/hosts` / router config)
pointing to your K8s ingress IP.

### 4. Configure F-Droid on your phone

1. Install F-Droid from https://f-droid.org
2. Open F-Droid → Settings → Repositories → Add repository
3. Enter: `http://fdroid.local/repo`
4. Accept the repo signing fingerprint
5. Grant F-Droid "Install unknown apps" permission
6. On Android 12+: Settings → Apps → F-Droid → Advanced → Allow unattended updates

### 5. Trigger first build

Push a change to `android/` on `main`, or manually trigger the workflow:

```bash
gh workflow run android-fdroid.yml
```

## Keystore Persistence

The repo signing keystore must remain the same across builds (F-Droid clients
reject repos whose signing key changes). Options:

1. **Store in a GitHub Actions secret** (base64-encoded):
   ```bash
   base64 -w0 keystore.p12 | gh secret set FDROID_KEYSTORE_B64
   ```
   Then decode it in CI before `fdroid update`.

2. **Store in the K8s PVC** alongside the repo files and download it in CI.

## Updating

The workflow runs automatically on every push to `main` that touches `android/**`.
The F-Droid client on your phone checks for updates every ~1 hour (or pull-to-refresh).

## Troubleshooting

- **"Repo signature mismatch"**: The keystore changed between builds. You need to
  remove and re-add the repo in F-Droid client.
- **APK not showing**: Check that `fdroid update` ran successfully in CI logs.
  The APK must be signed (release build).
- **Can't reach repo**: Verify DNS resolves `fdroid.local` to your K8s
  ingress IP. Check the nginx pod is running: `kubectl get pods -n realestate -l app=fdroid-repo`
