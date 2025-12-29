#!/usr/bin/env node

const { spawn } = require('child_process');

const tarball = process.argv[2];
if (!tarball) {
  console.error('❌ Usage: node scripts/mcp_test.js <path-to-tarball>');
  process.exit(1);
}

console.log('📡 Launching MCP server for smoke test...');

const child = spawn('npx', ['-y', `--package=${tarball}`, 'pcg-cc-mcp'], {
  stdio: ['ignore', 'pipe', 'pipe'],
  env: process.env,
});

let sawStartupLog = false;

const handleChunk = (chunk) => {
  const text = chunk.toString();
  process.stdout.write(text);
  if (text.includes('[MCP] Starting')) {
    sawStartupLog = true;
  }
};

child.stdout.on('data', handleChunk);
child.stderr.on('data', handleChunk);

const timeout = setTimeout(() => {
  console.log('⌛️ Stopping MCP server after smoke test...');
  child.kill('SIGINT');
}, 5000);

child.on('exit', (code, signal) => {
  clearTimeout(timeout);
  if (!sawStartupLog) {
    console.error('❌ MCP server did not emit startup logs.');
    process.exit(code && code !== 0 ? code : 1);
  }
  if (code && code !== 0) {
    console.error(`❌ MCP server exited with code ${code}`);
    process.exit(code);
  }
  if (signal && code === null) {
    console.log(`ℹ️ MCP server exited via signal ${signal}`);
  }
  console.log('✅ MCP server smoke test succeeded');
});

child.on('error', (err) => {
  clearTimeout(timeout);
  console.error('❌ Failed to start MCP server:', err);
  process.exit(1);
});
