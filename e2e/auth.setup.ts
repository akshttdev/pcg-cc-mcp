import { test as setup } from "@playwright/test";
import { login } from "./helpers";

export const AUTH_STATE_PATH = "e2e/.auth/user.json";

setup("authenticate", async ({ page }) => {
  await login(page);
  await page.context().storageState({ path: AUTH_STATE_PATH });
});
