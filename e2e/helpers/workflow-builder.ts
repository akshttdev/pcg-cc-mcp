/**
 * Workflow Builder Helpers — shared UI automation for adding nodes in the workflow editor.
 *
 * Used by demo specs that build workflows via the UI (e.g., CRM pipeline, Spanish pipeline).
 */
import { expect, type Page } from "@playwright/test";
import { t, demoPause } from "./timing";

/** Add an LLM Extract node: open picker, click LLM Extract, rename, fill prompt, connect to input. */
export async function addExtractNode(
  page: Page,
  name: string,
  prompt: string,
  connectTo: string
) {
  await page.getByRole("button", { name: "Add Node" }).click();
  await page.getByRole("button", { name: /^LLM Extract Extract/ }).click();
  await expect(page.getByText("Node Name")).toBeVisible({ timeout: t(5_000) });

  const nameInput = page.getByText("Node Name", { exact: true }).locator("..").getByRole("textbox");
  await nameInput.fill(name);

  const promptTextarea = page.getByRole("textbox", { name: /extraction instructions|Analyze the following/ });
  await expect(promptTextarea).toBeVisible({ timeout: t(5_000) });
  await promptTextarea.fill(prompt);

  await page.getByRole("combobox").filter({ hasText: /Add input connection/ }).click();
  await page.getByRole("option", { name: new RegExp(connectTo) }).first().click();

  await expect(page.getByText(`from: ${connectTo}`).last()).toBeVisible({ timeout: t(5_000) });
  await page.waitForTimeout(demoPause.medium);
}

/** Add an output node and connect it to the specified upstream node. */
export async function addOutputNode(
  page: Page,
  outputType: string,
  connectTo: string
) {
  await page.getByRole("button", { name: "Add Node" }).click();
  await page.getByRole("button", { name: new RegExp(`Output: ${outputType}`) }).click();

  await expect(page.getByText("Input Connections")).toBeVisible({ timeout: t(5_000) });

  const connectionCombo = page.getByRole("combobox").filter({ hasText: /Add input connection/ });
  await expect(connectionCombo).toBeVisible({ timeout: t(5_000) });
  await connectionCombo.click();
  await page.getByRole("option", { name: new RegExp(connectTo) }).first().click();

  await expect(page.getByText(`from: ${connectTo}`).last()).toBeVisible({ timeout: t(5_000) });
  await page.waitForTimeout(demoPause.medium);
}
