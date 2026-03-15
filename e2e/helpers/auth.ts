import { Page, APIRequestContext, expect } from "@playwright/test";

/** Default test credentials */
export const TEST_USER = {
  username: "admin",
  password: "admin123",
};

/** Login and wait for dashboard to load. Skips if already authenticated. */
export async function login(page: Page) {
  // If we're already on an app page (not login, not about:blank), skip login
  const url = page.url();
  if (url && !url.includes("/login") && url !== "about:blank") {
    return;
  }

  await page.goto("/login");
  await page.getByRole("textbox", { name: "Username or Email" }).fill(TEST_USER.username);
  await page.getByRole("textbox", { name: "Password" }).fill(TEST_USER.password);
  await page.getByRole("button", { name: "Sign in" }).click();

  // Wait for redirect away from login
  await expect(page).not.toHaveURL(/\/login/, { timeout: 10_000 });
}

/** Login via API and return authenticated request context */
export async function apiLogin(request: APIRequestContext) {
  const res = await request.post("/api/auth/login", {
    data: { username: TEST_USER.username, password: TEST_USER.password },
  });
  expect(res.ok()).toBeTruthy();
  return res;
}

/**
 * Ensure page is authenticated. Call this in beforeEach when using shared context.
 * If the page ended up on /login (e.g. after cookie loss), re-authenticates.
 */
export async function ensureAuthenticated(page: Page) {
  await page.goto("/");
  await page.waitForLoadState("domcontentloaded");
  // Check if we got redirected to login
  try {
    await page.waitForURL(/\/login/, { timeout: 1_000 });
    // We're on login — need to re-auth
    await login(page);
  } catch {
    // Not on login — we're authenticated, good
  }
}
