/**
 * PCG Discord voice bot — discord.js 14 + @discordjs/voice 0.19 (DAVE E2EE)
 *
 * Replaces the Rust twilight/songbird bot which lacks DAVE protocol support.
 * The Rust PCG server remains the "brain" — this bot is a thin audio adapter
 * that handles: voice join/leave, STT pipeline, agent REST calls, TTS playback.
 *
 * Slash commands per agent:
 *   /{agent}-join [project]  — join the invoker's voice channel
 *   /{agent}-leave           — leave voice, archive session
 *   /{agent}-ask <message>   — text-based agent query (plays TTS response too)
 *   /meeting-summary         — generate AI summary via agent
 *   /meeting-note <text>     — add timestamped note to transcript
 */

import { Client, Events, GatewayIntentBits } from 'discord.js';
import {
  joinVoiceChannel,
  createAudioPlayer,
  createAudioResource,
  StreamType,
  VoiceConnectionStatus,
  entersState,
  EndBehaviorType,
  AudioPlayerStatus,
  generateDependencyReport,
} from '@discordjs/voice';
import { opus as Opus } from 'prism-media';
import { createReadStream } from 'fs';
import { writeFile, unlink } from 'fs/promises';
import { randomUUID } from 'crypto';
import Database from 'better-sqlite3';

import {
  downsampleAndEncodeWav,
  transcribeWav,
  detectWakeWord,
  callAgent,
  synthesizeTts,
} from './audio.js';

// ─── Configuration ────────────────────────────────────────────────────────────

const SERVER_PORT = parseInt(process.env.BACKEND_PORT ?? process.env.SERVER_PORT ?? '3000', 10);
const DB_PATH =
  process.env.DISCORD_BOT_DB_PATH ??
  '/home/pythia/pcg-cc-mcp/dev_assets/db.sqlite';
const SILENCE_TIMEOUT_MS = 1000; // ms of silence before utterance is complete

const NORA_TOKEN = process.env.JS_DISCORD_BOT_TOKEN ?? process.env.DISCORD_BOT_TOKEN;
const TOPSI_TOKEN = process.env.JS_DISCORD_TOPSI_BOT_TOKEN ?? process.env.DISCORD_TOPSI_BOT_TOKEN;

// ─── Database ─────────────────────────────────────────────────────────────────

const db = new Database(DB_PATH, { readonly: false });
db.pragma('journal_mode = WAL');
db.pragma('busy_timeout = 5000');

// ─── Session state ────────────────────────────────────────────────────────────
// Key: `${guildId}:${agentName}` (matches Rust implementation)

const sessions = new Map();

// ─── Rate limits ──────────────────────────────────────────────────────────────

const userLastJoin = new Map();   // userId → timestamp
const guildLastJoin = new Map();  // guildId → timestamp

function checkRateLimit(guildId, userId) {
  const now = Date.now();
  if (userLastJoin.has(userId) && now - userLastJoin.get(userId) < 5000) {
    return 'Please wait a few seconds before using this command again.';
  }
  if (guildLastJoin.has(guildId) && now - guildLastJoin.get(guildId) < 30000) {
    return 'Another join was just initiated in this server. Please wait 30 seconds.';
  }
  userLastJoin.set(userId, now);
  guildLastJoin.set(guildId, now);
  return null;
}

// ─── Bot factory ──────────────────────────────────────────────────────────────

async function runBot(token, agentName) {
  const client = new Client({
    intents: [GatewayIntentBits.Guilds, GatewayIntentBits.GuildVoiceStates],
  });

  client.once(Events.ClientReady, async (c) => {
    console.log(`[${agentName}] Connected as ${c.user.tag}`);
    console.log(generateDependencyReport());
    await registerCommands(c, agentName);
  });

  client.on(Events.InteractionCreate, (interaction) => {
    handleInteraction(interaction, agentName).catch((e) =>
      console.error(`[${agentName}] Unhandled interaction error:`, e)
    );
  });

  await client.login(token);
}

// ─── Command registration ─────────────────────────────────────────────────────

async function registerCommands(client, agentName) {
  const ag = agentName.toLowerCase();
  const commands = [
    {
      name: `${ag}-join`,
      description: `${agentName} joins your current voice channel`,
      options: [
        {
          type: 3, // STRING
          name: 'project',
          description: 'Project ID to link this session to (optional)',
          required: false,
        },
      ],
    },
    {
      name: `${ag}-leave`,
      description: `${agentName} leaves the voice channel`,
    },
    {
      name: `${ag}-ask`,
      description: `Ask ${agentName} a question directly (also plays response via TTS)`,
      options: [
        {
          type: 3,
          name: 'message',
          description: 'Your message',
          required: true,
        },
      ],
    },
    {
      name: 'meeting-summary',
      description: 'Generate an AI summary of the current voice session',
    },
    {
      name: 'meeting-note',
      description: 'Add a timestamped note to the current session transcript',
      options: [
        {
          type: 3,
          name: 'text',
          description: 'Your note',
          required: true,
          min_length: 1,
          max_length: 500,
        },
      ],
    },
  ];

  await client.application.commands.set(commands);
  console.log(`[${agentName}] Slash commands registered`);
}

// ─── Interaction dispatch ─────────────────────────────────────────────────────

async function handleInteraction(interaction, agentName) {
  if (!interaction.isChatInputCommand()) return;
  if (!interaction.guildId) return;

  const { commandName: cmd } = interaction;
  const ag = agentName.toLowerCase();

  if (cmd === `${ag}-join`) {
    const rateLimitMsg = checkRateLimit(interaction.guildId, interaction.user.id);
    if (rateLimitMsg) {
      await interaction.reply({ content: rateLimitMsg, ephemeral: true });
      return;
    }
    await interaction.deferReply();
    await handleJoin(interaction, agentName);
  } else if (cmd === `${ag}-leave`) {
    await interaction.deferReply();
    await handleLeave(interaction, agentName);
  } else if (cmd === `${ag}-ask`) {
    await interaction.deferReply();
    await handleAsk(interaction, agentName);
  } else if (cmd === 'meeting-summary') {
    await interaction.deferReply();
    await handleSummary(interaction, agentName);
  } else if (cmd === 'meeting-note') {
    await interaction.deferReply();
    await handleNote(interaction, agentName);
  }
}

// ─── Join voice ───────────────────────────────────────────────────────────────

async function handleJoin(interaction, agentName) {
  // Resolve user's current voice channel
  const member = await interaction.guild.members.fetch(interaction.user.id);
  const voiceChannel = member.voice.channel;
  if (!voiceChannel) {
    await interaction.editReply(
      'You need to **join a voice channel first**, then run this command.'
    );
    return;
  }

  const sessionKey = `${interaction.guildId}:${agentName}`;

  // End any existing session for this guild+agent
  if (sessions.has(sessionKey)) {
    destroySession(sessionKey);
  }

  // Resolve project_id — try admin's first project, fall back to empty
  let projectId = interaction.options.getString('project') ?? '';
  if (!projectId) {
    try {
      const row = db
        .prepare(
          `SELECT hex(pm.project_id) AS pid_hex
           FROM project_members pm
           JOIN users u ON u.id = pm.user_id
           WHERE u.is_admin = 1
           ORDER BY pm.granted_at ASC LIMIT 1`
        )
        .get();
      if (row?.pid_hex) {
        const h = row.pid_hex.toLowerCase();
        projectId = `${h.slice(0, 8)}-${h.slice(8, 12)}-${h.slice(12, 16)}-${h.slice(16, 20)}-${h.slice(20)}`;
      }
    } catch (_) {
      // Non-critical; continue with empty projectId
    }
  }

  // Create meeting_session in DB
  const meetingId = randomUUID();
  const title = `${agentName} — #${voiceChannel.name} (Discord)`;
  const metadata = JSON.stringify({
    discord_guild_id: interaction.guildId,
    discord_channel_id: voiceChannel.id,
    discord_channel_name: voiceChannel.name,
    agent: agentName,
  });

  db.prepare(
    `INSERT INTO meeting_sessions
       (id, project_id, title, started_by, source_type, metadata, status)
     VALUES (?, ?, ?, 'discord-bot', 'discord', ?, 'active')`
  ).run(meetingId, projectId, title, metadata);

  // Join the voice channel (DAVE handled automatically by @discordjs/voice + @snazzah/davey)
  const connection = joinVoiceChannel({
    channelId: voiceChannel.id,
    guildId: interaction.guildId,
    adapterCreator: interaction.guild.voiceAdapterCreator,
    selfDeaf: false,
    selfMute: false,
    debug: process.env.VOICE_DEBUG === '1',
  });

  if (process.env.VOICE_DEBUG === '1') {
    connection.on('debug', (msg) => console.log(`[${agentName}][VoiceDebug]`, msg));
  }

  try {
    await entersState(connection, VoiceConnectionStatus.Ready, 30_000);
  } catch (e) {
    connection.destroy();
    db.prepare('DELETE FROM meeting_sessions WHERE id = ?').run(meetingId);
    await interaction.editReply(
      `Failed to join **#${voiceChannel.name}**: \`${e.message}\`\n\nCheck bot permissions (Connect + Speak) in that channel.`
    );
    return;
  }

  const player = createAudioPlayer();
  connection.subscribe(player);

  // Seed participant list from current channel members (excluding bots)
  const participants = new Map(); // userId → displayName
  for (const [memberId, member] of voiceChannel.members) {
    if (!member.user.bot) {
      participants.set(memberId, member.displayName || member.user.username);
    }
  }

  const session = {
    meetingId,
    guildId: interaction.guildId,
    channelId: voiceChannel.id,
    channelName: voiceChannel.name,
    projectId,
    agentName,
    startedAt: Date.now(),
    segmentCount: 0,
    connection,
    player,
    guild: interaction.guild,
    participants,
  };

  sessions.set(sessionKey, session);

  // Track participants joining/leaving while bot is in channel
  const voiceStateHandler = (oldState, newState) => {
    if (oldState.channelId === newState.channelId) return; // mute/deafen only — ignore
    const member = newState.member ?? oldState.member;
    if (!member || member.user.bot) return;
    const userId = member.id;
    if (newState.channelId === voiceChannel.id) {
      // Joined this channel
      participants.set(userId, member.displayName || member.user.username);
      console.log(`[${agentName}] ${member.displayName} joined #${voiceChannel.name}`);
    } else if (oldState.channelId === voiceChannel.id) {
      // Left this channel
      participants.delete(userId);
      console.log(`[${agentName}] ${member.displayName} left #${voiceChannel.name}`);
    }
  };
  interaction.guild.client.on('voiceStateUpdate', voiceStateHandler);
  session.voiceStateHandler = voiceStateHandler;
  session.guildClient = interaction.guild.client;

  // Announce arrival with TTS greeting
  const greetings = {
    nora: `Hello, I'm Nora — PowerClub Global's Executive AI Agent. I'm in the channel and listening. Just say my name to speak with me.`,
    topsi: `Hi there, I'm Topsi, PowerClub Global's technical AI agent. I'm listening — just say my name to get my attention.`,
  };
  const greeting = greetings[agentName.toLowerCase()] ?? `Hello, I'm ${agentName} and I'm listening.`;
  playTts(session, greeting).catch(() => {});

  setupReceivePipeline(connection, session);

  // Handle unexpected disconnects
  connection.on(VoiceConnectionStatus.Disconnected, async () => {
    try {
      // Try to reconnect on brief disconnects
      await Promise.race([
        entersState(connection, VoiceConnectionStatus.Signalling, 5_000),
        entersState(connection, VoiceConnectionStatus.Connecting, 5_000),
      ]);
    } catch {
      destroySession(sessionKey);
    }
  });

  // Update bot nickname to signal active listening
  try {
    await interaction.guild.members.me.setNickname(`[Listening] ${agentName}`);
  } catch (_) {
    // Missing Manage Nicknames permission — non-fatal
  }

  console.log(
    `[${agentName}] Joined #${voiceChannel.name} in guild ${interaction.guildId} | session ${meetingId}`
  );

  await interaction.editReply(
    `**${agentName}** has joined **#${voiceChannel.name}** and is listening.\n` +
    `Say my name to speak with me directly.\n` +
    `_Session ID: \`${meetingId.slice(0, 8)}\`_`
  );
}

// ─── Audio receive pipeline ───────────────────────────────────────────────────

function setupReceivePipeline(connection, session) {
  // Track which users already have an active subscription to avoid duplicates
  const activeSubs = new Set();

  connection.receiver.speaking.on('start', (userId) => {
    if (activeSubs.has(userId)) return;
    activeSubs.add(userId);

    // Resolve display name — fetch from guild if not yet cached
    if (!session.participants.has(userId)) {
      session.guild.members.fetch(userId).then((member) => {
        session.participants.set(userId, member.displayName || member.user.username);
      }).catch(() => {
        session.participants.set(userId, `User ${userId.slice(-4)}`);
      });
    }

    const opusStream = connection.receiver.subscribe(userId, {
      end: { behavior: EndBehaviorType.AfterSilence, duration: SILENCE_TIMEOUT_MS },
    });

    // Decode Opus → 48kHz stereo PCM
    const decoder = new Opus.Decoder({ rate: 48000, channels: 2, frameSize: 960 });
    const pcmStream = opusStream.pipe(decoder);

    const chunks = [];
    pcmStream.on('data', (chunk) => chunks.push(chunk));

    pcmStream.on('end', () => {
      activeSubs.delete(userId);
      if (chunks.length === 0) return;
      const pcm = Buffer.concat(chunks);
      const displayName = session.participants.get(userId) ?? `User ${userId.slice(-4)}`;
      processUtterance(pcm, userId, displayName, session).catch((e) =>
        console.error(`[${session.agentName}] Pipeline error for user ${userId}:`, e.message)
      );
    });

    pcmStream.on('error', (e) => {
      activeSubs.delete(userId);
      // DAVE decryption failures during key transitions are expected and transient
      if (!e.message?.includes('decrypt') && !e.message?.includes('DecryptionFailed')) {
        console.error(`[${session.agentName}] Decoder error for ${userId}:`, e.message);
      }
    });
  });
}

async function processUtterance(pcm, userId, displayName, session) {
  const startMs = Date.now() - session.startedAt;

  // 48kHz stereo → 16kHz mono WAV → Whisper
  const wavBuffer = downsampleAndEncodeWav(pcm);
  const transcript = await transcribeWav(wavBuffer);
  if (!transcript) return;

  console.log(`[${session.agentName}] [${displayName}]: ${transcript.slice(0, 100)}`);

  const addressed = detectWakeWord(transcript, session.agentName);
  const isAddressed = addressed !== null;
  const endMs = Date.now() - session.startedAt;

  let agentResponse = null;
  if (isAddressed) {
    try {
      const cmd = addressed || transcript;
      // Build participant context for the agent
      const others = [...session.participants.entries()]
        .filter(([id]) => id !== userId)
        .map(([, name]) => name);
      const participantCtx = others.length
        ? `Channel participants: ${[displayName, ...others].join(', ')}. Speaking now: ${displayName}.`
        : `Speaking: ${displayName}.`;

      agentResponse = await callAgent(
        SERVER_PORT,
        session.agentName,
        cmd,
        session.projectId,
        session.meetingId,
        participantCtx
      );
    } catch (e) {
      console.warn(`[${session.agentName}] Agent call failed:`, e.message);
    }
  }

  // Play TTS response (non-blocking)
  if (agentResponse) {
    playTts(session, agentResponse).catch((e) =>
      console.warn(`[${session.agentName}] TTS playback failed:`, e.message)
    );
  }

  // Persist user utterance
  session.segmentCount++;
  db.prepare(
    `INSERT INTO meeting_segments
       (id, meeting_session_id, segment_index, speaker_label, text, confidence,
        start_time_ms, end_time_ms, is_topsi_addressed, metadata)
     VALUES (?, ?, ?, ?, ?, 0.9, ?, ?, ?, ?)`
  ).run(
    randomUUID(),
    session.meetingId,
    session.segmentCount,
    displayName,
    transcript,
    startMs,
    endMs,
    isAddressed ? 1 : 0,
    JSON.stringify({ discord_user_id: userId, discord_display_name: displayName, agent_response: agentResponse })
  );

  // Persist agent response as its own segment
  if (agentResponse) {
    session.segmentCount++;
    db.prepare(
      `INSERT INTO meeting_segments
         (id, meeting_session_id, segment_index, speaker_label, text, confidence,
          start_time_ms, end_time_ms, is_topsi_addressed, metadata)
       VALUES (?, ?, ?, ?, ?, 1.0, ?, ?, 0, ?)`
    ).run(
      randomUUID(),
      session.meetingId,
      session.segmentCount,
      session.agentName,
      agentResponse,
      endMs,
      endMs,
      JSON.stringify({ is_agent_response: true })
    );
  }
}

async function playTts(session, text) {
  const audioBuffer = await synthesizeTts(text, session.agentName);
  // Use .wav for Chatterbox (Topsi), .mp3 for ElevenLabs (Nora) — FFmpeg handles both
  const ext = session.agentName.toLowerCase() === 'topsi' ? 'wav' : 'mp3';
  const tmpPath = `/tmp/pcg_tts_${randomUUID()}.${ext}`;
  await writeFile(tmpPath, audioBuffer);

  const resource = createAudioResource(createReadStream(tmpPath), {
    inputType: StreamType.Arbitrary,
  });

  session.player.play(resource);

  // Wait for playback to finish before cleaning up temp file
  await entersState(session.player, AudioPlayerStatus.Idle, 120_000).catch(() => {});
  await unlink(tmpPath).catch(() => {});
}

// ─── Leave voice ──────────────────────────────────────────────────────────────

async function handleLeave(interaction, agentName) {
  const sessionKey = `${interaction.guildId}:${agentName}`;
  if (!sessions.has(sessionKey)) {
    await interaction.editReply("I'm not in a voice channel in this server.");
    return;
  }

  destroySession(sessionKey);

  try {
    await interaction.guild.members.me.setNickname(agentName);
  } catch (_) {}

  await interaction.editReply(
    "I've left the voice channel. The session has been archived to the dashboard."
  );
}

function destroySession(sessionKey) {
  const session = sessions.get(sessionKey);
  if (!session) return;

  try {
    session.connection.destroy();
  } catch (_) {}

  // Remove voice state listener
  if (session.guildClient && session.voiceStateHandler) {
    session.guildClient.off('voiceStateUpdate', session.voiceStateHandler);
  }

  sessions.delete(sessionKey);

  db.prepare(
    `UPDATE meeting_sessions
     SET status = 'ended',
         ended_at = datetime('now', 'subsec'),
         updated_at = datetime('now', 'subsec')
     WHERE id = ?`
  ).run(session.meetingId);

  console.log(`[${session.agentName}] Session ${session.meetingId} ended`);

  // Save session transcript as a knowledge source for PCG Organization
  saveSessionAsKnowledgeSource(session);
}

function saveSessionAsKnowledgeSource(session) {
  try {
    // Pull full transcript from DB
    const segments = db.prepare(
      `SELECT speaker_label, text, is_topsi_addressed
       FROM meeting_segments
       WHERE meeting_session_id = ?
       ORDER BY segment_index`
    ).all(session.meetingId);

    if (segments.length === 0) return;

    // Build readable transcript
    const transcript = segments
      .map((s) => `${s.speaker_label}: ${s.text}`)
      .join('\n');

    const addressedCount = segments.filter((s) => s.is_topsi_addressed).length;
    const durationMin = ((Date.now() - session.startedAt) / 60000).toFixed(1);
    const dateStr = new Date().toISOString().slice(0, 10);

    const title = `Discord Voice — #${session.channelName} — ${session.agentName} — ${dateStr}`;
    const summary =
      `${session.agentName} voice session in #${session.channelName} on ${dateStr}. ` +
      `Duration: ${durationMin} min. Segments: ${segments.length}. ` +
      `${session.agentName} addressed ${addressedCount} time(s).\n\n` +
      `TRANSCRIPT:\n${transcript.slice(0, 4000)}`;

    // Look up PCG Organization ID
    const orgRow = db.prepare(
      `SELECT hex(id) AS org_hex FROM organizations WHERE name LIKE '%PowerClub%' OR name LIKE '%Power Club%' LIMIT 1`
    ).get();

    // Convert session projectId (UUID string) to BLOB bytes for the foreign key
    const projectIdHex = session.projectId?.replace(/-/g, '') ?? '';
    if (!projectIdHex || projectIdHex.length !== 32) return;
    const projectIdBuf = Buffer.from(projectIdHex, 'hex');

    // Upsert into project_knowledge_sources
    db.prepare(
      `INSERT INTO project_knowledge_sources
         (id, project_id, source_type, source_id, source_title, source_summary,
          coverage_score, is_active, is_stale, auto_registered, owner_type, owner_id)
       VALUES (randomblob(16), ?, 'conversation', ?, ?, ?, 0.7, 1, 0, 1, ?, ?)
       ON CONFLICT(project_id, source_type, source_id) DO UPDATE SET
         source_title    = excluded.source_title,
         source_summary  = excluded.source_summary,
         coverage_score  = excluded.coverage_score,
         is_stale        = 0,
         updated_at      = datetime('now', 'subsec')`
    ).run(
      projectIdBuf,
      session.meetingId,
      title,
      summary,
      orgRow ? 'organization' : 'project',
      orgRow ? orgRow.org_hex.toLowerCase() : null,
    );

    console.log(`[${session.agentName}] Session saved to knowledge graph (org: ${orgRow?.org_hex ?? 'none'})`);
  } catch (e) {
    console.warn(`[${session.agentName}] Knowledge source save failed:`, e.message);
  }
}

// ─── Ask (text + TTS fallback) ────────────────────────────────────────────────

async function handleAsk(interaction, agentName) {
  const message = interaction.options.getString('message', true);
  const sessionKey = `${interaction.guildId}:${agentName}`;
  const session = sessions.get(sessionKey);

  const projectId = session?.projectId ?? '';
  const sessionId = session?.meetingId ?? `ask-${randomUUID()}`;

  try {
    const response = await callAgent(SERVER_PORT, agentName, message, projectId, sessionId);
    await interaction.editReply(`**${agentName}**: ${response.slice(0, 1900)}`);

    // Also play the response via TTS if bot is in a voice channel
    if (session) {
      playTts(session, response).catch(() => {});
    }
  } catch (e) {
    await interaction.editReply(`Failed to reach ${agentName}: ${e.message}`);
  }
}

// ─── Meeting summary ──────────────────────────────────────────────────────────

async function handleSummary(interaction, agentName) {
  const sessionKey = `${interaction.guildId}:${agentName}`;
  const session = sessions.get(sessionKey);

  if (!session) {
    await interaction.editReply('No active voice session in this server.');
    return;
  }

  const rows = db
    .prepare(
      `SELECT COALESCE(speaker_label, 'Unknown') AS speaker, text
       FROM meeting_segments
       WHERE meeting_session_id = ?
       ORDER BY segment_index ASC`
    )
    .all(session.meetingId);

  if (rows.length === 0) {
    await interaction.editReply('No transcript yet — keep talking!');
    return;
  }

  const transcript = rows.map((r) => `[${r.speaker}]: ${r.text}`).join('\n');
  const prompt =
    `Provide a concise structured summary: key topics, decisions, and action items.\n\nTranscript:\n${transcript.slice(0, 4000)}`;

  try {
    const summary = await callAgent(
      SERVER_PORT,
      agentName,
      prompt,
      session.projectId,
      session.meetingId
    );
    await interaction.editReply(`**Meeting Summary**\n${summary.slice(0, 1900)}`);
  } catch (e) {
    await interaction.editReply(`Failed to generate summary: ${e.message}`);
  }
}

// ─── Meeting note ─────────────────────────────────────────────────────────────

async function handleNote(interaction, agentName) {
  const noteText = interaction.options.getString('text', true);
  const sessionKey = `${interaction.guildId}:${agentName}`;
  const session = sessions.get(sessionKey);

  if (!session) {
    await interaction.editReply('No active voice session in this server.');
    return;
  }

  const elapsedMs = Date.now() - session.startedAt;
  session.segmentCount += 1000; // high index so notes sort after speech segments

  db.prepare(
    `INSERT INTO meeting_segments
       (id, meeting_session_id, segment_index, speaker_label, text, confidence,
        start_time_ms, end_time_ms, is_topsi_addressed, metadata)
     VALUES (?, ?, ?, ?, ?, 1.0, ?, ?, 0, ?)`
  ).run(
    randomUUID(),
    session.meetingId,
    session.segmentCount,
    `Note (${interaction.user.username})`,
    `[NOTE] ${noteText}`,
    elapsedMs,
    elapsedMs,
    JSON.stringify({ is_note: true, discord_user_id: interaction.user.id })
  );

  await interaction.editReply(
    `Note added at ${Math.floor(elapsedMs / 1000)}s: _${noteText}_`
  );
}

// ─── Main ─────────────────────────────────────────────────────────────────────

// Determine which bots to run based on AGENT env var (or run both if unset)
const AGENT_FILTER = process.env.AGENT; // 'Nora' | 'Topsi' | undefined

const botConfigs = [
  { name: 'Nora', token: NORA_TOKEN },
  { name: 'Topsi', token: TOPSI_TOKEN },
].filter(({ name, token }) => {
  if (!token) {
    const envVar = name === 'Nora' ? 'JS_DISCORD_BOT_TOKEN' : 'JS_DISCORD_TOPSI_BOT_TOKEN';
    console.warn(`[${name}] No token configured — set ${envVar} in .env`);
    return false;
  }
  if (AGENT_FILTER && AGENT_FILTER.toLowerCase() !== name.toLowerCase()) return false;
  return true;
});

if (botConfigs.length === 0) {
  console.error('No bot tokens configured. Set DISCORD_BOT_TOKEN and/or DISCORD_TOPSI_BOT_TOKEN in .env');
  process.exit(1);
}

for (const { name, token } of botConfigs) {
  runBot(token, name).catch((e) => {
    console.error(`[${name}] Fatal startup error:`, e);
    process.exit(1);
  });
}

// Graceful shutdown
process.on('SIGTERM', () => {
  console.log('SIGTERM received — ending all sessions');
  for (const key of sessions.keys()) destroySession(key);
  process.exit(0);
});

process.on('SIGINT', () => {
  for (const key of sessions.keys()) destroySession(key);
  process.exit(0);
});
