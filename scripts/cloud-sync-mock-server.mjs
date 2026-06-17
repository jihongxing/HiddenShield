import { createServer } from 'node:http';
import { randomUUID } from 'node:crypto';

const port = Number.parseInt(process.env.HIDDENSHIELD_CLOUD_PORT ?? '43187', 10);
const host = process.env.HIDDENSHIELD_CLOUD_HOST ?? '127.0.0.1';

const accountsByIdentifier = new Map();
const sessions = new Map();
const events = [];
let sequence = 0;

const server = createServer(async (request, response) => {
  try {
    const url = new URL(request.url ?? '/', `http://${request.headers.host}`);
    if (request.method === 'GET' && url.pathname === '/v1/health') {
      return sendJson(response, 200, {
        ok: true,
        service: 'hidden-shield-cloud-sync-mock',
        accounts: accountsByIdentifier.size,
        events: events.length,
        cloudSync: true,
      });
    }

    if (request.method === 'POST' && url.pathname === '/v1/auth/continue') {
      return handleAuthContinue(request, response);
    }

    if (request.method === 'POST' && url.pathname === '/v1/sync/events:batch') {
      return handleEventsBatch(request, response);
    }

    if (request.method === 'GET' && url.pathname === '/v1/sync/changes') {
      return handleChanges(request, response, url);
    }

    sendJson(response, 404, { error: 'not_found' });
  } catch (error) {
    sendJson(response, 500, { error: 'internal_error', message: `${error}` });
  }
});

server.on('error', (error) => {
  console.error(`Cloud sync mock failed to listen on ${host}:${port}: ${error.message}`);
  process.exit(1);
});

server.listen(port, host, () => {
  console.log(`HiddenShield cloud sync mock listening on http://${host}:${port}`);
  console.log('Endpoints: /v1/health, /v1/auth/continue, /v1/sync/events:batch, /v1/sync/changes');
});

async function handleAuthContinue(request, response) {
  const body = await readJson(request);
  const identifier = String(body.identifier ?? '').trim().toLowerCase();
  if (!identifier) {
    return sendJson(response, 400, { error: 'identifier_required' });
  }

  let account = accountsByIdentifier.get(identifier);
  if (!account) {
    const suffix = stableSuffix(identifier);
    account = {
      id: `acct_${suffix}`,
      displayName: identifier,
      workspace: { id: `ws_${suffix}`, name: '个人空间' },
      creatorProfile: {
        id: `creator_${suffix}`,
        displayName:
          body.localCreatorProfile?.displayName?.trim() || identifier,
        isDefault: true,
      },
      entitlement: {
        id: `ent_${suffix}`,
        planName: '免费版',
        planCode: 'free',
        status: 'free',
        features: {
          cloud_sync: true,
          batch_processing: false,
          cloud_video_processing: false,
        },
      },
      devices: new Map(),
    };
    accountsByIdentifier.set(identifier, account);
  }

  const requestedDevice = body.device ?? {};
  const deviceId = String(requestedDevice.clientDeviceId ?? '').trim() ||
    `device_${stableSuffix(randomUUID())}`;
  const device = {
    id: deviceId,
    name: String(requestedDevice.name ?? '当前设备'),
    platform: String(requestedDevice.platform ?? 'unknown'),
    registered: true,
  };
  account.devices.set(deviceId, device);

  const accessToken = `mock_${account.id}_${deviceId}_${Date.now()}`;
  sessions.set(accessToken, {
    accountId: account.id,
    deviceId,
    workspaceId: account.workspace.id,
  });

  sendJson(response, 200, {
    accessToken,
    refreshToken: `refresh_${account.id}_${deviceId}`,
    account: { id: account.id, displayName: account.displayName },
    workspace: account.workspace,
    device,
    creatorProfile: account.creatorProfile,
    entitlement: account.entitlement,
  });
}

async function handleEventsBatch(request, response) {
  const session = authenticate(request);
  if (!session) {
    return sendJson(response, 401, { error: 'unauthorized' });
  }

  const body = await readJson(request);
  const deviceId = String(body.deviceId ?? '').trim();
  const incomingEvents = Array.isArray(body.events) ? body.events : [];
  if (!deviceId) {
    return sendJson(response, 400, { error: 'device_id_required' });
  }
  if (session.deviceId !== deviceId) {
    return sendJson(response, 401, { error: 'unauthorized' });
  }
  const workspaceId = String(body.workspaceId ?? '').trim();
  if (!workspaceId) {
    return sendJson(response, 400, { error: 'workspace_id_required' });
  }
  if (session.workspaceId !== workspaceId) {
    return sendJson(response, 403, { error: 'forbidden' });
  }
  if (incomingEvents.length === 0) {
    return sendJson(response, 400, { error: 'events_required' });
  }

  const acceptedEventIds = [];
  for (const event of incomingEvents) {
    const clientEventId = String(event.clientEventId ?? '').trim();
    const entityType = String(event.entityType ?? '').trim();
    const entityId = String(event.entityId ?? '').trim();
    if (!clientEventId || !entityType || !entityId) {
      continue;
    }
    acceptedEventIds.push(clientEventId);
    events.push({
      sequence: ++sequence,
      accountId: session.accountId,
      sourceDevice: deviceId,
      clientEventId,
      entityType,
      operation: String(event.operation ?? 'upsert'),
      entityId,
      entity: normalizeEntity(entityId, event.payload),
      createdAt: new Date().toISOString(),
    });
  }

  sendJson(response, 200, {
    accepted: acceptedEventIds.length,
    acceptedEventIds,
    nextCursor: cursorFromSequence(sequence),
    resolutions: [],
  });
}

function handleChanges(request, response, url) {
  const session = authenticate(request);
  if (!session) {
    return sendJson(response, 401, { error: 'unauthorized' });
  }
  const workspaceId = String(url.searchParams.get('workspaceId') ?? '').trim();
  if (!workspaceId) {
    return sendJson(response, 400, { error: 'workspace_id_required' });
  }
  if (session.workspaceId !== workspaceId) {
    return sendJson(response, 403, { error: 'forbidden' });
  }

  const cursor = url.searchParams.get('cursor');
  const sinceSequence = sequenceFromCursor(cursor);
  const changes = events
    .filter((event) => event.accountId === session.accountId)
    .filter((event) => event.sequence > sinceSequence)
    .map((event) => ({
      cursor: cursorFromSequence(event.sequence),
      entityType: event.entityType,
      operation: cloudOperation(event.operation),
      sourceDevice: event.sourceDevice,
      entity: event.entity,
    }));

  sendJson(response, 200, {
    nextCursor: cursorFromSequence(sequence),
    changes,
  });
}

function authenticate(request) {
  const header = request.headers.authorization ?? '';
  const token = header.startsWith('Bearer ') ? header.slice('Bearer '.length).trim() : '';
  return sessions.get(token);
}

function cloudOperation(operation) {
  return operation.startsWith('upsert') ? 'upsert' : operation;
}

function normalizeEntity(entityId, payload) {
  const entity = isObject(payload) ? payload : {};
  return {
    id: String(entity.id ?? entityId),
    kind: String(entity.kind ?? 'image'),
    title: String(entity.title ?? 'Cloud sync record'),
    watermark_uid: String(entity.watermark_uid ?? entity.watermarkUid ?? entityId),
    revision: Number(entity.revision ?? 1),
    sha256: entity.sha256 == null ? undefined : String(entity.sha256),
    parent_watermark_uid: optionalString(entity.parent_watermark_uid ?? entity.parentWatermarkUid),
    rewrite_reason: optionalString(entity.rewrite_reason ?? entity.rewriteReason),
    extracted_timestamp: optionalNumber(entity.extracted_timestamp ?? entity.extractedTimestamp),
    extracted_device_id_hex: optionalString(entity.extracted_device_id_hex ?? entity.extractedDeviceIdHex),
    extracted_file_hash_hex: optionalString(entity.extracted_file_hash_hex ?? entity.extractedFileHashHex),
    source: optionalString(entity.source),
    created_at: String(entity.created_at ?? entity.createdAt ?? new Date().toISOString()),
  };
}

function optionalString(value) {
  return value == null || value === '' ? undefined : String(value);
}

function optionalNumber(value) {
  if (value == null || value === '') {
    return undefined;
  }
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : undefined;
}

function isObject(value) {
  return value != null && typeof value === 'object' && !Array.isArray(value);
}

async function readJson(request) {
  const chunks = [];
  for await (const chunk of request) {
    chunks.push(chunk);
  }
  const raw = Buffer.concat(chunks).toString('utf8').trim();
  if (!raw) {
    return {};
  }
  return JSON.parse(raw);
}

function sendJson(response, statusCode, body) {
  const json = JSON.stringify(body);
  response.writeHead(statusCode, {
    'content-type': 'application/json; charset=utf-8',
    'content-length': Buffer.byteLength(json),
  });
  response.end(json);
}

function stableSuffix(input) {
  let hash = 2166136261;
  for (const char of input) {
    hash ^= char.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16);
}

function cursorFromSequence(value) {
  return `cursor_${value}`;
}

function sequenceFromCursor(cursor) {
  if (!cursor) {
    return 0;
  }
  const match = /^cursor_(\d+)$/.exec(cursor);
  return match ? Number.parseInt(match[1], 10) : 0;
}
