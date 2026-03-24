import type { McpConfig, JsonValue } from 'shared/types';

/** Nested JSON object used for MCP config traversal */
type JsonObject = { [key: string]: JsonValue | undefined };

export class McpConfigStrategyGeneral {
  static createFullConfig(cfg: McpConfig): JsonObject {
    // create a template with servers filled in at cfg.servers
    const fullConfig = JSON.parse(JSON.stringify(cfg.template)) as JsonObject;
    let current = fullConfig;
    for (let i = 0; i < cfg.servers_path.length - 1; i++) {
      const key = cfg.servers_path[i];
      if (!current[key]) {
        current[key] = {};
      }
      current = current[key] as JsonObject;
    }
    if (cfg.servers_path.length > 0) {
      const lastKey = cfg.servers_path[cfg.servers_path.length - 1];
      current[lastKey] = cfg.servers as JsonValue;
    }
    return fullConfig;
  }
  static validateFullConfig(
    mcp_config: McpConfig,
    full_config: JsonObject
  ): void {
    // Validate using the schema path
    let current: JsonValue | undefined = full_config as JsonValue;
    for (const key of mcp_config.servers_path) {
      current = (current as JsonObject)?.[key];
      if (current === undefined) {
        throw new Error(
          `Missing required field at path: ${mcp_config.servers_path.join('.')}`
        );
      }
    }
    if (typeof current !== 'object') {
      throw new Error('Servers configuration must be an object');
    }
  }
  static extractServersForApi(
    mcp_config: McpConfig,
    full_config: JsonObject
  ): JsonObject {
    // Extract the servers object based on the path
    let current: JsonValue | undefined = full_config as JsonValue;
    for (const key of mcp_config.servers_path) {
      current = (current as JsonObject)?.[key];
      if (current === undefined) {
        throw new Error(
          `Missing required field at path: ${mcp_config.servers_path.join('.')}`
        );
      }
    }
    return current as JsonObject;
  }

  static addPreconfiguredToConfig(
    mcp_config: McpConfig,
    existingConfig: JsonObject,
    serverKey: string
  ): JsonObject {
    const preconf = mcp_config.preconfigured as JsonObject;
    if (!preconf || typeof preconf !== 'object' || !(serverKey in preconf)) {
      throw new Error(`Unknown preconfigured server '${serverKey}'`);
    }

    const updated = JSON.parse(JSON.stringify(existingConfig || {})) as JsonObject;
    let current = updated;

    for (let i = 0; i < mcp_config.servers_path.length - 1; i++) {
      const key = mcp_config.servers_path[i];
      if (!current[key] || typeof current[key] !== 'object') current[key] = {};
      current = current[key] as JsonObject;
    }

    const lastKey = mcp_config.servers_path[mcp_config.servers_path.length - 1];
    if (!current[lastKey] || typeof current[lastKey] !== 'object')
      current[lastKey] = {};

    (current[lastKey] as JsonObject)[serverKey] = preconf[serverKey];

    return updated;
  }
}
