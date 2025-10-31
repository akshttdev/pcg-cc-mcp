# Cloudflare Tunnel Setup Guide

This guide walks you through setting up Cloudflare Tunnel for secure public access to your PCG-CC-MCP deployment.

## Why Cloudflare Tunnel?

- ✅ **No port forwarding** - No need to configure your router
- ✅ **Free SSL certificates** - Automatic HTTPS
- ✅ **DDoS protection** - Built-in Cloudflare security
- ✅ **Access control** - Optional authentication layer
- ✅ **Free tier available** - No credit card required

## Step-by-Step Setup

### 1. Create Cloudflare Account

1. Go to [cloudflare.com](https://cloudflare.com) and sign up (free)
2. Add your domain (or use a free Cloudflare Pages domain)

### 2. Access Zero Trust Dashboard

1. Log in to Cloudflare
2. Navigate to [Zero Trust Dashboard](https://one.dash.cloudflare.com/)
3. If first time, you'll need to create a team name

### 3. Create a Tunnel

1. In Zero Trust dashboard, go to **Networks** > **Tunnels**
2. Click **Create a tunnel**
3. Choose **Cloudflared** as the connector type
4. Name your tunnel (e.g., `pcg-cc-mcp-production`)
5. Click **Save tunnel**

### 4. Get Tunnel Token

After creating the tunnel, you'll see installation instructions. Look for a command like:

```bash
cloudflared service install eyJhIjoiNzE5M2Y4Y2YtZjU5Zi00OTBiLWFhMTQtMzI5ZWI5YjQ0YWNhIiwidCI6IjUzNjEyOWM2LWZmM2QtNDk1Mi05ZDk2LWFlMzJmZGVhZGNlZiIsInMiOiJOVFkzWkRGak9UQTRNR0ZqTnpKa01UQTRZVE0yTVRZMk1HSXhNakUxWW1JNU9HWXlNRGszT1dZNCJ9
```

The long string starting with `eyJ...` is your **tunnel token**. Copy it!

### 5. Configure Public Hostname

1. In the tunnel configuration page, click **Public Hostname** tab
2. Click **Add a public hostname**
3. Configure:
   - **Subdomain**: `pcg` (or whatever you want)
   - **Domain**: Select your domain
   - **Path**: Leave empty
   - **Service Type**: `HTTP`
   - **URL**: `app:3001` (this is the Docker service name and port)

Example: `pcg.yourdomain.com` → `http://app:3001`

4. Click **Save hostname**

### 6. Add Token to Environment

Edit your `.env` file:

```env
CLOUDFLARE_TUNNEL_TOKEN=eyJhIjoiNzE5M2Y4Y2YtZjU5Zi00OTBiLWFhMTQtMzI5ZWI5YjQ0YWNhIiwidCI6IjUzNjEyOWM2LWZmM2QtNDk1Mi05ZDk2LWFlMzJmZGVhZGNlZiIsInMiOiJOVFkzWkRGak9UQTRNR0ZqTnpKa01UQTRZVE0yTVRZMk1HSXhNakUxWW1JNU9HWXlNRGszT1dZNCJ9
```

### 7. Deploy

```bash
./deploy.sh
```

Select option 1 (Build and start).

### 8. Verify Connection

1. Check Cloudflare dashboard - tunnel should show as "Healthy"
2. Visit your configured URL (e.g., `https://pcg.yourdomain.com`)
3. You should see the login page!

## Advanced Configuration

### Add Authentication Layer

You can add Cloudflare Access for extra security:

1. In Zero Trust dashboard, go to **Access** > **Applications**
2. Click **Add an application**
3. Choose **Self-hosted**
4. Configure:
   - **Application name**: PCG Dashboard
   - **Session duration**: 24 hours (or your preference)
   - **Application domain**: Your tunnel URL
5. Add policies (e.g., email whitelist, GitHub auth, etc.)

### Multiple Environments

You can create separate tunnels for staging and production:

```env
# .env.production
CLOUDFLARE_TUNNEL_TOKEN=your_prod_token

# .env.staging
CLOUDFLARE_TUNNEL_TOKEN=your_staging_token
```

Then run with specific env file:
```bash
docker-compose --env-file .env.production up -d
```

### Custom Domains

If you want to use your own domain:

1. Add domain to Cloudflare (it will give you nameservers)
2. Update your domain's nameservers at your registrar
3. Wait for DNS propagation (usually a few hours)
4. Create tunnel with your custom domain

### Tunnel Status Monitoring

Check tunnel health:
```bash
# View cloudflared logs
docker-compose logs -f cloudflared

# Check tunnel status in dashboard
# Go to Networks > Tunnels > Your Tunnel
```

## Troubleshooting

### Tunnel shows "Down"

1. Check cloudflared container is running: `docker-compose ps`
2. Check logs: `docker-compose logs cloudflared`
3. Verify token is correct in `.env`
4. Restart: `docker-compose restart cloudflared`

### 502 Bad Gateway

1. Check app container is running: `docker-compose ps app`
2. Verify app is healthy: `docker-compose exec app wget -O- http://localhost:3001/api/auth/me`
3. Check service URL in tunnel config is `http://app:3001`
4. Restart app: `docker-compose restart app`

### Cannot access URL

1. Verify DNS has propagated: `nslookup pcg.yourdomain.com`
2. Check tunnel is connected in Cloudflare dashboard
3. Try incognito/private mode (clear browser cache)
4. Check hostname configuration in tunnel settings

### SSL/TLS Errors

1. Ensure tunnel config uses `http://` not `https://` for the service
2. Cloudflare handles SSL termination - your app doesn't need SSL
3. Try switching SSL mode in Cloudflare: Full → Flexible

## Free Cloudflare Pages Alternative

If you don't have a domain:

1. Use Cloudflare Pages free domain: `your-tunnel.pages.dev`
2. Or use Cloudflare's free Workers domain
3. Both work with Zero Trust tunnels

## Cost

- **Free tier**: 50 users, unlimited traffic
- **Paid tier** (optional): More users, advanced features like device posture checks

For most deployments, the free tier is sufficient!

## Security Best Practices

1. ✅ **Enable Access policies** for sensitive environments
2. ✅ **Use strong admin passwords** in the application
3. ✅ **Rotate tunnel tokens** periodically
4. ✅ **Monitor tunnel logs** for suspicious activity
5. ✅ **Enable Cloudflare WAF** (Web Application Firewall)

## Support

- [Cloudflare Tunnel Docs](https://developers.cloudflare.com/cloudflare-one/connections/connect-apps/)
- [Cloudflare Community](https://community.cloudflare.com/)
- [Zero Trust Dashboard](https://one.dash.cloudflare.com/)
