#!/bin/bash
# Publishes site/ to the shared Hostinger VPS at https://cadenza.devops-monk.com
#
# Built to leave the other apps on the server alone:
#   - files go only to /var/www/cadenza
#   - nginx gets one new file, sites-available/cadenza.devops-monk.com.conf
#     (refused if another server block already claims the domain)
#   - nginx is only *reloaded* (graceful, no dropped connections), and only
#     after `nginx -t` passes; a failing test removes our file again
#   - certbot touches only this domain's certificate and server block
#   - Postgres and every other service are never touched
set -euo pipefail

HOST="${DEPLOY_HOST:-root@168.231.79.163}"
DOMAIN="cadenza.devops-monk.com"
WEBROOT="/var/www/cadenza"
EMAIL="${CERTBOT_EMAIL:-}"
root="$(cd "$(dirname "$0")/.." && pwd)"
ssh_opts=(-o BatchMode=yes -o ConnectTimeout=15)

# Images, CSS and JS are cached for days, so every reference in the pages
# gets ?v=<content hash>: a changed file is a new URL and browsers fetch it.
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
rsync -a --exclude tools/ "$root/site/" "$stage/"
for page in "$stage"/*.html; do
    SITE="$stage" perl -i -MDigest::MD5 -pe '
        s{((?:images/[\w.-]+\.(?:png|jpe?g|webp|svg|ico|mp4|webm))|style\.css|main\.js)(?=["\x27)\s])}{
            my $f = "$ENV{SITE}/$1";
            if (open(my $fh, "<", $f)) { binmode $fh; "$1?v=" . substr(Digest::MD5->new->addfile($fh)->hexdigest, 0, 8) } else { $1 }
        }ge' "$page"
done

echo "==> Uploading site to $HOST:$WEBROOT"
ssh "${ssh_opts[@]}" "$HOST" "mkdir -p $WEBROOT"
rsync -az --delete -e "ssh ${ssh_opts[*]}" "$stage/" "$HOST:$WEBROOT/"
scp "${ssh_opts[@]}" -q "$root/deploy/$DOMAIN.conf" "$HOST:/tmp/$DOMAIN.conf"

ssh "${ssh_opts[@]}" "$HOST" DOMAIN="$DOMAIN" WEBROOT="$WEBROOT" EMAIL="$EMAIL" 'bash -s' <<'REMOTE'
set -euo pipefail
chown -R www-data:www-data "$WEBROOT" 2>/dev/null || true
find "$WEBROOT" -type d -exec chmod 755 {} +
find "$WEBROOT" -type f -exec chmod 644 {} +

available=/etc/nginx/sites-available/$DOMAIN.conf
enabled=/etc/nginx/sites-enabled/$DOMAIN.conf

# Another config already serving this domain? Then stop: never fight it.
others=$(grep -rlsE "server_name[^;]*\b$DOMAIN\b" /etc/nginx/sites-enabled/ /etc/nginx/conf.d/ 2>/dev/null | grep -v "$DOMAIN.conf" || true)
if [ -n "$others" ]; then
    echo "!! $DOMAIN is already served by: $others — leaving nginx unchanged." >&2
    exit 1
fi

if [ -f "$available" ] && grep -q "managed by Certbot" "$available"; then
    echo "==> nginx config exists and has TLS; keeping it"
else
    [ -f "$available" ] && cp "$available" "/tmp/$DOMAIN.conf.bak.$(date +%s)"
    cp "/tmp/$DOMAIN.conf" "$available"
    ln -sf "$available" "$enabled"
fi

if ! nginx -t 2>&1; then
    echo "!! nginx -t failed: removing the Cadenza config, nothing reloaded" >&2
    rm -f "$enabled"
    nginx -t >/dev/null 2>&1 && echo "   (the server's config is valid again)"
    exit 1
fi
systemctl reload nginx
echo "==> nginx reloaded"

if ! grep -q "managed by Certbot" "$available"; then
    if command -v certbot >/dev/null; then
        mail=(--register-unsafely-without-email)
        [ -n "$EMAIL" ] && mail=(-m "$EMAIL")
        certbot --nginx -d "$DOMAIN" --non-interactive --agree-tos --redirect "${mail[@]}" \
            && echo "==> HTTPS enabled" \
            || echo "!! certbot failed (is the DNS A record pointing here?) — site stays on HTTP"
        nginx -t && systemctl reload nginx
    else
        echo "!! certbot is not installed; site is on HTTP only"
    fi
fi
REMOTE

echo "==> Checking"
curl -sS -o /dev/null -w "https://$DOMAIN -> %{http_code}\n" "https://$DOMAIN/" || \
curl -sS -o /dev/null -w "http://$DOMAIN -> %{http_code}\n" "http://$DOMAIN/"
