# Security Review: GCP Deployment Configuration

## Executive Summary

This document provides a comprehensive security analysis of the xxx cloud deployment configuration in `main.ts` and associated files. Several **CRITICAL** and **HIGH** severity security issues have been identified that require immediate attention.

## Security Issues Identified

### 🔴 CRITICAL Severity Issues

#### 1. Unrestricted SSH Access (main.ts:146)
**Severity**: CRITICAL
**Location**: `main.ts:146` - SSH firewall rule
**Issue**:
```typescript
sourceRanges: ["0.0.0.0/0"],  // ❌ ALLOWS SSH FROM ANYWHERE
```

**Risk**:
- Exposes SSH to the entire internet
- Primary attack vector for brute force attacks
- Increases attack surface significantly

**Recommendation**:
```typescript
// Option 1: Restrict to specific IP/CIDR
sourceRanges: [process.env.ALLOWED_SSH_CIDR || "YOUR_IP/32"],

// Option 2: Use Cloud IAP for SSH (preferred)
// Remove this firewall rule and use gcloud compute ssh with IAP
```

**Remediation Priority**: IMMEDIATE

---

#### 2. Unrestricted Service API Access (main.ts:166)
**Severity**: CRITICAL
**Location**: `main.ts:166` - Service ports firewall rule
**Issue**:
```typescript
sourceRanges: ["0.0.0.0/0"],  // ❌ ALLOWS API ACCESS FROM ANYWHERE
targetTags: ["web"],
```

**Risk**:
- REPL API exposed to the internet WITHOUT authentication
- Allows anyone to execute arbitrary code in containers
- No rate limiting at network level
- Perfect target for abuse, crypto mining, DDoS amplification

**Recommendation**:
```typescript
// Option 1: Restrict to known IPs/CIDRs
sourceRanges: [
  process.env.ALLOWED_API_CIDR_1,
  process.env.ALLOWED_API_CIDR_2,
],

// Option 2: Use Cloud Load Balancer with Cloud Armor for DDoS protection
// Option 3: Implement VPN or Cloud IAP for access
// Option 4: At minimum, add API authentication layer
```

**Immediate Mitigation**:
Even with the security measures implemented in the REPL code, **network-level restrictions are essential** as a defense-in-depth strategy.

**Remediation Priority**: IMMEDIATE

---

#### 3. Privileged Container (compose.yml:47)
**Severity**: CRITICAL
**Location**: `compose.yml:47` - coreos container
**Issue**:
```yaml
coreos:
  privileged: true  # ❌ FULL HOST ACCESS
  command: ["/sbin/init"]
```

**Risk**:
- Container has unrestricted access to host system
- Can escape container isolation
- Can access all host devices and kernel capabilities
- Compromised container = compromised host

**Recommendation**:
```yaml
# Instead of privileged: true, use specific capabilities
security_opt:
  - "no-new-privileges:true"
cap_add:
  - SYS_ADMIN  # Only if absolutely necessary for podman-in-podman
  - NET_ADMIN  # Only if needed
# Drop all other capabilities by default
cap_drop:
  - ALL
```

**Alternative**: Consider rootless podman or gVisor for better isolation.

**Remediation Priority**: HIGH (after network restrictions)

---

### 🟠 HIGH Severity Issues

#### 4. Insecure Docker Compose Download (main.ts:100)
**Severity**: HIGH
**Location**: `main.ts:100` - install-podman-compose.service
**Issue**:
```typescript
ExecStart=/usr/bin/curl -SL "https://github.com/docker/compose/releases/download/v2.39.4/docker-compose-linux-aarch64" -o /usr/local/lib/docker/cli-plugins/docker-compose
```

**Risks**:
- No checksum verification
- No signature validation
- Susceptible to MITM attacks
- Compromised binary could execute malicious code as root

**Recommendation**:
```bash
# Add SHA256 verification
COMPOSE_VERSION="v2.39.4"
COMPOSE_CHECKSUM="<official_sha256_here>"

ExecStartPre=/usr/bin/curl -SL "https://github.com/docker/compose/releases/download/${COMPOSE_VERSION}/docker-compose-linux-aarch64" -o /tmp/docker-compose
ExecStartPre=/usr/bin/bash -c 'echo "${COMPOSE_CHECKSUM}  /tmp/docker-compose" | sha256sum -c -'
ExecStart=/usr/bin/mv /tmp/docker-compose /usr/local/lib/docker/cli-plugins/docker-compose
ExecStartPost=/usr/bin/chmod +x /usr/local/lib/docker/cli-plugins/docker-compose
```

**Remediation Priority**: HIGH

---

#### 5. Unvalidated compose.yml Injection (main.ts:65)
**Severity**: HIGH
**Location**: `main.ts:37-66` - compose.yml file injection
**Issue**:
```typescript
const composeYml = fs.readFileSync(
    path.join(vmAssetsDir, "compose.yml"),
    "utf-8"
);
// ...
source: `data:,${encodeURIComponent(composeYml)}`,  // No validation
```

**Risk**:
- If `compose.yml` is compromised, malicious configuration deployed
- No validation of compose file syntax or content
- Could inject malicious containers or configurations

**Recommendation**:
```typescript
// Add validation
import { parse } from 'yaml';

const composeYml = fs.readFileSync(
    path.join(vmAssetsDir, "compose.yml"),
    "utf-8"
);

// Validate YAML structure
try {
    const parsedCompose = parse(composeYml);

    // Validate no privileged containers (except whitelist)
    for (const [name, service] of Object.entries(parsedCompose.services)) {
        if (service.privileged && !ALLOWED_PRIVILEGED.includes(name)) {
            throw new Error(`Unauthorized privileged container: ${name}`);
        }
    }
} catch (error) {
    throw new Error(`Invalid compose.yml: ${error.message}`);
}
```

**Remediation Priority**: MEDIUM

---

#### 6. etcd Exposed Without Authentication (compose.yml:72-83)
**Severity**: HIGH
**Location**: `compose.yml:72-83` - etcd configuration
**Issue**:
```yaml
command: [
  "etcd",
  "--advertise-client-urls", "http://0.0.0.0:2379",
  "--listen-client-urls", "http://0.0.0.0:2379",
  # No authentication configured
]
```

**Risk**:
- etcd contains service registry data
- No authentication required
- Anyone on the network can read/write
- Could poison service registry, redirect traffic, steal data

**Recommendation**:
```yaml
# Enable client authentication
command: [
  "etcd",
  "--client-cert-auth",
  "--trusted-ca-file=/path/to/ca.crt",
  "--cert-file=/path/to/server.crt",
  "--key-file=/path/to/server.key",
  # ... other flags
]
```

**Alternative**: Use etcd RBAC with username/password authentication.

**Remediation Priority**: HIGH

---

### 🟡 MEDIUM Severity Issues

#### 7. SSH Key Auto-Generation Without Passphrase (main.ts:26)
**Severity**: MEDIUM
**Location**: `main.ts:26`
**Issue**:
```typescript
execSync(`ssh-keygen -t rsa -b 4096 -f ${sshKeyPath} -N "" -C "cdktf-user"`, {
    //                                                     ^^ Empty passphrase
```

**Risk**:
- Private key stored unencrypted on disk
- If developer machine compromised, key is exposed
- No additional security layer for key usage

**Recommendation**:
```typescript
// Option 1: Require passphrase
console.log("Please enter a passphrase for the SSH key:");
execSync(`ssh-keygen -t ed25519 -f ${sshKeyPath} -C "cdktf-user"`, {
    stdio: "inherit",  // Allow interactive passphrase input
});

// Option 2: Use ssh-agent and encrypted keys
// Option 3: Use Google Cloud OS Login instead of SSH keys
```

**Note**: Consider using Ed25519 instead of RSA (smaller, faster, more secure).

**Remediation Priority**: MEDIUM

---

#### 8. Container Images from Unverified Registry
**Severity**: MEDIUM
**Location**: `compose.yml` - all image references
**Issue**:
```yaml
image: ghcr.io/geoffsee/container-api:stable
image: ghcr.io/geoffsee/repl-api:stable
# etc. - using :stable tag, no digest verification
```

**Risk**:
- `:stable` tag can be overwritten
- No guarantee of image integrity
- Potential supply chain attack

**Recommendation**:
```yaml
# Use digest pinning for immutable references
image: ghcr.io/geoffsee/container-api@sha256:abc123...
image: ghcr.io/geoffsee/repl-api@sha256:def456...

# Or at minimum, use specific version tags
image: ghcr.io/geoffsee/container-api:v1.2.3
```

**Additional**: Implement image scanning (Trivy, Clair) in CI/CD.

**Remediation Priority**: MEDIUM

---

#### 9. No TLS/HTTPS for Service Communication
**Severity**: MEDIUM
**Location**: `compose.yml` and `main.ts:218`
**Issue**:
```yaml
# All services use HTTP
environment:
  - SERVICE_REGISTRY_URL=http://service-registry:3003

# External access also HTTP
value: `http://\${${externalIp}}:${process.env.REPL_API_PORT || "3002"}`,
```

**Risk**:
- Traffic sniffable on network
- No encryption of potentially sensitive data
- Man-in-the-middle attacks possible

**Recommendation**:
```typescript
// For external access
value: `https://\${${externalIp}}:${process.env.REPL_API_PORT || "3002"}`,

// Use Let's Encrypt or Cloud Load Balancer with managed certificates
// For internal: Consider service mesh (Istio) or mTLS between services
```

**Note**: Internal HTTP may be acceptable if network is trusted, but external MUST use HTTPS.

**Remediation Priority**: MEDIUM (HIGH for production)

---

### 🔵 LOW Severity Issues

#### 10. Overly Broad Network Tags (main.ts:194)
**Severity**: LOW
**Location**: `main.ts:194`
**Issue**:
```typescript
tags: ["web", "dev"],  // Generic tags
```

**Risk**:
- Tags are used for firewall targeting
- Generic tags might accidentally match other rules
- Least privilege principle violation

**Recommendation**:
```typescript
tags: ["xxx-repl-api", "xxx-v1"],  // Specific, namespaced tags
```

**Remediation Priority**: LOW

---

#### 11. Spot Instance with No Graceful Shutdown Handling
**Severity**: LOW
**Location**: `main.ts:175-179`
**Issue**:
```typescript
scheduling: {
    preemptible: true,
    automaticRestart: false,
    onHostMaintenance: "TERMINATE",
},
```

**Risk**:
- Preemptible VMs can be terminated with 30 seconds notice
- No graceful shutdown handling for services
- Potential data loss or incomplete operations

**Recommendation**:
```yaml
# In compose.yml, add stop_grace_period
services:
  repl-api:
    stop_grace_period: 25s  # Less than 30s preemption notice
    # Handle SIGTERM gracefully in application code
```

**Remediation Priority**: LOW (unless data loss is critical)

---

#### 12. No Resource Limits on Systemd Services (main.ts:88-121)
**Severity**: LOW
**Location**: `main.ts:88-121` - systemd unit definitions
**Issue**:
```typescript
contents: `[Unit]
Description=Download Docker Compose CLI plugin
# No resource limits specified
[Service]
Type=oneshot
```

**Risk**:
- Services could consume excessive resources
- Could impact system stability

**Recommendation**:
```systemd
[Service]
Type=oneshot
CPUQuota=50%
MemoryLimit=512M
TasksMax=100
```

**Remediation Priority**: LOW

---

## Defense in Depth Analysis

The application-level security measures (from SECURITY.md) are excellent:
- ✅ Code validation
- ✅ Rate limiting (application level)
- ✅ Execution timeouts
- ✅ Container isolation

However, **network-level security is critically lacking**:
- ❌ No network firewall restrictions
- ❌ No authentication/authorization
- ❌ No TLS/encryption
- ❌ No DDoS protection
- ❌ No WAF or API gateway

### Defense in Depth Layers Needed:

1. **Network Layer** (MISSING - CRITICAL):
   - Restrict source IPs for SSH and API access
   - Implement Cloud Armor or equivalent WAF
   - DDoS protection

2. **Transport Layer** (MISSING - HIGH):
   - TLS/HTTPS for external access
   - Certificate management

3. **Application Layer** (IMPLEMENTED):
   - Code validation ✅
   - Rate limiting ✅
   - Input sanitization ✅

4. **Container Layer** (PARTIALLY IMPLEMENTED):
   - Network namespacing ✅
   - Privileged container ❌ (needs fix)
   - Resource limits ⚠️ (commented out)

5. **Authentication/Authorization Layer** (MISSING - HIGH):
   - No API keys
   - No OAuth/OIDC
   - No user management

---

## Recommended Security Hardening

### Immediate Actions (Today)

1. **Restrict SSH access** to specific IP ranges:
```typescript
sourceRanges: [process.env.ADMIN_IP_CIDR || "YOUR_IP/32"],
```

2. **Restrict API access** or add authentication:
```typescript
// Minimum: Restrict IPs
sourceRanges: [process.env.ALLOWED_API_CIDRS?.split(",") || ["YOUR_IP/32"]],

// Better: Add API key authentication to repl-api
```

3. **Verify docker-compose checksum** before installation

---

### Short Term (This Week)

4. **Remove privileged flag** from coreos container or justify with detailed security analysis

5. **Enable etcd authentication**:
```bash
etcdctl user add root
etcdctl auth enable
```

6. **Add HTTPS/TLS** for external API access using Cloud Load Balancer

7. **Pin container images** to specific digests

---

### Medium Term (This Month)

8. **Implement API authentication**:
   - API keys with rate limiting per key
   - OAuth 2.0 or JWT tokens
   - Integration with identity provider

9. **Add Cloud Armor** WAF rules:
   - Rate limiting by IP
   - SQL injection detection
   - XSS protection
   - Bot detection

10. **Implement monitoring and alerting**:
    - Cloud Monitoring for suspicious activity
    - Alert on failed SSH attempts
    - Alert on unusual API usage patterns

11. **Security scanning**:
    - Container image scanning in CI/CD
    - Dependency vulnerability scanning
    - Infrastructure as Code scanning (tfsec, checkov)

---

### Long Term (This Quarter)

12. **Consider service mesh** (Istio) for:
    - mTLS between services
    - Fine-grained access control
    - Traffic encryption

13. **Implement secrets management**:
    - Google Secret Manager for sensitive data
    - Rotate credentials regularly
    - Avoid hardcoded credentials

14. **Security audit and penetration testing**

15. **Compliance and documentation**:
    - Security runbook
    - Incident response plan
    - Access control policies

---

## Configuration Examples

### Secure SSH Firewall Rule
```typescript
new ComputeFirewall(this, "AllowSSH", {
    name: "allow-ssh-restricted",
    network: network.name,
    allow: [
        {
            protocol: "tcp",
            ports: ["22"],
        },
    ],
    sourceRanges: [
        process.env.ADMIN_IP_CIDR || "127.0.0.1/32",  // Replace with your IP
    ],
    targetTags: ["xxx-ssh-access"],
    description: "Restricted SSH access from admin IPs only",
});
```

### Secure API Firewall with Cloud Armor
```typescript
// Use Cloud Load Balancer + Cloud Armor instead of direct firewall rule
const backendService = new ComputeBackendService(this, "APIBackend", {
    name: "xxx-api-backend",
    protocol: "HTTPS",
    // ... backend config
});

const securityPolicy = new ComputeSecurityPolicy(this, "APISecurityPolicy", {
    name: "xxx-api-security-policy",
    rule: [
        {
            action: "rate_based_ban",
            priority: 100,
            match: {
                versionedExpr: "SRC_IPS_V1",
                config: {
                    srcIpRanges: ["*"],
                },
            },
            rateLimitOptions: {
                conformAction: "allow",
                exceedAction: "deny(429)",
                rateLimitThreshold: {
                    count: 100,
                    intervalSec: 60,
                },
            },
        },
    ],
});
```

### Secure etcd Configuration
```yaml
coreos-etcd:
  image: ghcr.io/geoffsee/etcd:stable
  environment:
    - ETCD_ROOT_PASSWORD=${ETCD_PASSWORD}
    - ETCD_AUTH_ENABLED=true
  secrets:
    - etcd_ca_cert
    - etcd_server_cert
    - etcd_server_key
```

---

## Risk Assessment Summary

| Issue | Severity | Likelihood | Impact | Risk Score |
|-------|----------|------------|---------|------------|
| Unrestricted SSH | CRITICAL | High | Critical | 🔴 9.5/10 |
| Unrestricted API | CRITICAL | High | Critical | 🔴 9.8/10 |
| Privileged Container | CRITICAL | Medium | Critical | 🔴 8.5/10 |
| Unverified Binary Download | HIGH | Medium | High | 🟠 7.5/10 |
| No etcd Auth | HIGH | Medium | High | 🟠 7.0/10 |
| No TLS/HTTPS | MEDIUM | High | Medium | 🟡 6.5/10 |
| Unverified Images | MEDIUM | Low | High | 🟡 5.5/10 |

**Overall Risk Level**: 🔴 **CRITICAL** - Immediate action required

---

## Conclusion

The deployment configuration has **critical security vulnerabilities** that must be addressed before production use or exposure to the internet. The primary concerns are:

1. **Unrestricted network access** to SSH and APIs
2. **Privileged container** with full host access
3. **No authentication** on public-facing APIs
4. **No encryption** for data in transit

While the application-level security (code validation, rate limiting, container timeouts) is well-implemented, **network-level security is severely lacking**. This creates a scenario where attackers can easily reach and abuse the system.

### Recommended Immediate Actions:
1. ✅ Restrict firewall rules to specific IPs immediately
2. ✅ Add HTTPS/TLS for external access
3. ✅ Implement API authentication
4. ✅ Remove or justify privileged container access
5. ✅ Enable etcd authentication

**DO NOT deploy this configuration to production** or expose it to the internet without addressing at least the CRITICAL severity issues.