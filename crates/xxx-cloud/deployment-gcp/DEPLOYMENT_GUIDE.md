# Secure Deployment Guide for xxx Cloud

This guide will help you securely deploy the xxx REPL system to Google Cloud Platform.

## Prerequisites

1. Google Cloud Platform account with billing enabled
2. `gcloud` CLI installed and configured
3. Node.js and npm installed
4. Terraform CDK (cdktf) installed: `npm install -g cdktf-cli`

## Security First Deployment

### Step 1: Configure Environment Variables

```bash
# Copy the example environment file
cp .env.example .env

# Find your current IP address
curl ifconfig.me

# Edit .env with your secure values
nano .env
```

**Required Configuration**:
```bash
# Your GCP project
GCP_PROJECT_ID=your-actual-project-id

# CRITICAL: Restrict access to your IP only
ALLOWED_SSH_CIDR=YOUR.IP.ADDRESS/32
ALLOWED_API_CIDR=YOUR.IP.ADDRESS/32

# Generate a secure password
ETCD_ROOT_PASSWORD=$(openssl rand -base64 32)
```

### Step 2: Review Security Settings

Before deployment, verify these security measures are in place:

✅ **Firewall Rules**:
- SSH access restricted to `ALLOWED_SSH_CIDR` (not 0.0.0.0/0)
- API access restricted to `ALLOWED_API_CIDR` (not 0.0.0.0/0)

✅ **Container Security**:
- No privileged containers (CoreOS uses capabilities instead)
- All containers have resource limits
- etcd authentication enabled

✅ **Binary Verification**:
- Docker Compose downloaded with checksum verification

### Step 3: Deploy

```bash
# Install dependencies
npm install

# Generate Terraform configuration
cdktf synth

# Review the Terraform plan
cdktf diff

# Deploy (you'll be asked to confirm)
cdktf deploy
```

### Step 4: Post-Deployment Verification

After deployment completes:

```bash
# Note the external IP from output
EXTERNAL_IP=$(cdktf output instance_external_ip)

# Test SSH access (should work only from your IP)
ssh -i cdktf-ssh-key core@$EXTERNAL_IP

# Wait for services to start (~2-3 minutes)
ssh -i cdktf-ssh-key core@$EXTERNAL_IP 'journalctl -u start-services.service -f'

# Verify all services are running
ssh -i cdktf-ssh-key core@$EXTERNAL_IP 'podman ps'

# Test API access (should work only from your IP)
curl http://$EXTERNAL_IP:3002/api/repl/languages
```

## Security Hardening

### Enable HTTPS (Recommended for Production)

For production deployments, add TLS/HTTPS:

1. **Option 1: Cloud Load Balancer**
   - Create a Google Cloud Load Balancer
   - Use Google-managed SSL certificates
   - Point backend to your instance

2. **Option 2: Let's Encrypt**
   ```bash
   # SSH into instance
   ssh -i cdktf-ssh-key core@$EXTERNAL_IP
   
   # Install certbot (if not using load balancer)
   # Configure nginx/traefik as reverse proxy with SSL
   ```

### Additional Security Measures

1. **Enable Cloud Armor** (DDoS Protection):
```bash
gcloud compute security-policies create xxx-api-policy \
    --description "Rate limiting for xxx API"

gcloud compute security-policies rules create 100 \
    --security-policy xxx-api-policy \
    --expression "true" \
    --action "rate-based-ban" \
    --rate-limit-threshold-count 100 \
    --rate-limit-threshold-interval-sec 60 \
    --ban-duration-sec 600
```

2. **Enable Cloud Monitoring**:
```bash
# Create log-based metrics for security events
gcloud logging metrics create security_violations \
    --description="Security violation attempts" \
    --log-filter='jsonPayload.message=~"security violations"'
```

3. **Regular Updates**:
```bash
# Update container images regularly
ssh -i cdktf-ssh-key core@$EXTERNAL_IP 'podman-compose pull && podman-compose up -d'
```

## Monitoring and Maintenance

### Check Security Logs

```bash
# SSH into instance
ssh -i cdktf-ssh-key core@$EXTERNAL_IP

# View repl-api security logs
journalctl -u podman -t repl-api | grep -i "security\|violation\|blocked"

# View failed SSH attempts
journalctl -u sshd | grep -i "failed\|invalid"
```

### Update Firewall Rules

If you need to add/change allowed IPs:

```bash
# Update .env file
nano .env

# Add new CIDR (comma-separated)
ALLOWED_API_CIDR=OLD.IP/32,NEW.IP/32

# Redeploy
cdktf deploy
```

## Troubleshooting

### Services Not Starting

```bash
ssh -i cdktf-ssh-key core@$EXTERNAL_IP

# Check systemd service status
systemctl status start-services.service

# View detailed logs
journalctl -u start-services.service -xe

# Check individual container logs
podman logs repl-api
podman logs container-api
```

### Cannot Access API

1. **Check firewall rules**:
```bash
# Verify your current IP
curl ifconfig.me

# Ensure it matches ALLOWED_API_CIDR in .env
```

2. **Check service is running**:
```bash
ssh -i cdktf-ssh-key core@$EXTERNAL_IP 'podman ps | grep repl-api'
```

3. **Test from correct IP**:
```bash
# API only accessible from allowed IPs
curl http://$EXTERNAL_IP:3002/api/repl/languages
```

### Docker Compose Checksum Failure

If the install-podman-compose service fails due to checksum mismatch:

```bash
# SSH into instance
ssh -i cdktf-ssh-key core@$EXTERNAL_IP

# Check the service status
systemctl status install-podman-compose.service

# If checksum failed, this is a SECURITY FEATURE preventing MITM attacks
# Verify the correct checksum from official Docker Compose releases:
# https://github.com/docker/compose/releases/tag/v2.39.4
```

## Backup and Recovery

### Backup etcd Data

```bash
ssh -i cdktf-ssh-key core@$EXTERNAL_IP

# Backup etcd data
podman exec coreos-etcd etcdctl snapshot save /etcd-data/backup.db

# Copy backup locally
podman cp coreos-etcd:/etcd-data/backup.db ./etcd-backup-$(date +%Y%m%d).db
```

### Restore from Backup

```bash
# Stop services
podman-compose down

# Restore etcd data
podman run --rm -v etcd-data:/etcd-data \
    ghcr.io/geoffsee/etcd:stable \
    etcdctl snapshot restore /etcd-data/backup.db

# Start services
podman-compose up -d
```

## Decommissioning

To safely tear down the deployment:

```bash
# Destroy all resources
cdktf destroy

# Confirm when prompted

# Clean up local files
rm -f cdktf-ssh-key cdktf-ssh-key.pub
```

## Security Checklist

Before going to production, ensure:

- [ ] `ALLOWED_SSH_CIDR` is set to specific IPs (not 0.0.0.0/0)
- [ ] `ALLOWED_API_CIDR` is set to specific IPs (not 0.0.0.0/0)
- [ ] `ETCD_ROOT_PASSWORD` changed from default
- [ ] HTTPS/TLS enabled for external API access
- [ ] Cloud Armor or equivalent WAF configured
- [ ] Monitoring and alerting set up
- [ ] Regular backup schedule configured
- [ ] Incident response plan documented
- [ ] Security review completed (see SECURITY_REVIEW.md)
- [ ] Container images updated to latest versions

## Additional Resources

- [SECURITY_REVIEW.md](./SECURITY_REVIEW.md) - Comprehensive security analysis
- [../../SECURITY.md](../../SECURITY.md) - Application-level security measures
- [Google Cloud Security Best Practices](https://cloud.google.com/security/best-practices)
- [Container Security Best Practices](https://cloud.google.com/architecture/best-practices-for-operating-containers)

## Support

For security issues or concerns:
1. Review SECURITY_REVIEW.md for known issues and mitigations
2. Check application logs for security events
3. Ensure all environment variables are properly configured
4. Test from allowed IP addresses only

**Remember**: Security is a continuous process. Regularly review and update your security posture.
