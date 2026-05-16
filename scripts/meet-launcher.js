#!/usr/bin/env node
/**
 * meet-launcher.js — host-side bridge for Docker → meet-bot.js
 *
 * The Docker server cannot spawn node/playwright/pactl directly.
 * This service runs on the HOST and receives launch requests from Docker,
 * spawns meet-bot.js, handles PulseAudio routing, and fires lifecycle
 * callbacks back to the server.
 *
 * Usage: node meet-launcher.js [port]   (default port 3006)
 */

const http = require('http');
const { spawn, execSync } = require('child_process');
const path = require('path');

const PORT = parseInt(process.env.MEET_LAUNCHER_PORT || process.argv[2] || '3006', 10);
const SCRIPTS_DIR = path.dirname(path.resolve(__filename));
const BOT_PATH = path.join(SCRIPTS_DIR, 'meet-bot.js');

// POST JSON to a URL, fire-and-forget
function postJson(url, data) {
    try {
        const parsed = new URL(url);
        const body = JSON.stringify(data);
        const opts = {
            hostname: parsed.hostname,
            port: parsed.port || (parsed.protocol === 'https:' ? 443 : 80),
            path: parsed.pathname,
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(body) },
        };
        const req = http.request(opts);
        req.on('error', () => {});
        req.write(body);
        req.end();
    } catch (e) {
        console.error('[LAUNCHER] postJson error:', e.message);
    }
}

function launchMeetBot({ meet_url, session_id, server_url, profile_dir }) {
    const short = session_id.slice(0, 8);
    // MEET_BOT_BACKEND_URL overrides server_url from request body.
    // Docker sends its own BACKEND_URL (localhost:3001) but the bot runs on the host
    // and must reach the host's local server (localhost:3000) for STT/TTS.
    const effective_server_url = process.env.MEET_BOT_BACKEND_URL || server_url;
    const effective_profile = process.env.NORA_CHROME_PROFILE || profile_dir || '/home/pythia/nora-meet-profile';
    console.log(`[LAUNCHER] Spawning meet-bot for session ${short} → ${meet_url} backend=${effective_server_url}`);

    const child = spawn('node', [BOT_PATH, meet_url, session_id, effective_server_url, effective_profile], {
        env: { ...process.env, DISPLAY: process.env.DISPLAY || ':1' },
        stdio: ['ignore', 'pipe', 'pipe'],
    });

    child.stdout.on('data', (chunk) => {
        for (const line of chunk.toString().split('\n').filter(Boolean)) {
            console.log(`[MEET-BOT ${short}] ${line}`);
            try {
                const ev = JSON.parse(line);
                if (ev.type === 'joined') {
                    // Route Chrome's audio sink-inputs to the virtual sink on the host
                    const outSink = `nora-meet-out-${short}`;
                    setTimeout(() => {
                        try {
                            const sinkInputs = execSync('pactl list short sink-inputs').toString();
                            for (const ln of sinkInputs.split('\n').filter(Boolean)) {
                                const id = ln.split(/\s+/)[0];
                                try { execSync(`pactl move-sink-input ${id} ${outSink}`); } catch {}
                            }
                            console.log(`[LAUNCHER] Audio routing done for ${short}`);
                        } catch (e) {
                            console.error('[LAUNCHER] pactl error:', e.message);
                        }
                    }, 2000);

                    // Tell the server Nora joined so it can synthesise the intro
                    postJson(`${effective_server_url}/nora/meet/${session_id}/joined`, { session_id });
                }
                // done/leaving — /nora/meet/:id/left is called directly by meet-bot.js
            } catch {}
        }
    });

    child.stderr.on('data', (chunk) => {
        console.error(`[MEET-BOT-ERR ${short}] ${chunk}`);
    });

    child.on('exit', (code) => {
        console.log(`[LAUNCHER] meet-bot ${short} exited with code ${code}`);
    });
}

const server = http.createServer((req, res) => {
    if (req.method === 'POST' && req.url === '/launch') {
        let body = '';
        req.on('data', (c) => (body += c));
        req.on('end', () => {
            try {
                const params = JSON.parse(body);
                res.writeHead(200, { 'Content-Type': 'application/json' });
                res.end(JSON.stringify({ ok: true, session_id: params.session_id }));
                launchMeetBot(params);
            } catch (e) {
                res.writeHead(400);
                res.end(JSON.stringify({ error: e.message }));
            }
        });
    } else if (req.method === 'GET' && req.url === '/health') {
        res.writeHead(200);
        res.end('ok');
    } else {
        res.writeHead(404);
        res.end();
    }
});

server.listen(PORT, () => {
    console.log(`[MEET-LAUNCHER] Listening on port ${PORT}`);
    console.log(`[MEET-LAUNCHER] Bot path: ${BOT_PATH}`);
});
