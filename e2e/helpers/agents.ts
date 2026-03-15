import { APIRequestContext, expect } from "@playwright/test";
import { apiLogin } from "./auth";

/** Ensure core agents (including ORCHA QA) exist by hitting the seed endpoint. */
export async function ensureAgentsSeeded(request: APIRequestContext) {
  await apiLogin(request);
  const res = await request.post("/api/agents/seed");
  // 201 = seeded, 200 = already existed — both are fine
  expect(res.status()).toBeLessThan(300);
}
