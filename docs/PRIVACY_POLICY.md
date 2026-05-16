# Privacy Policy

**Effective date:** 2026-04-28
**Last updated:** 2026-04-28

This Privacy Policy describes how the **PCG Dashboard** ("the Dashboard", "we", "us") collects, uses, stores, and shares information when you install, run, or connect third-party services to the Dashboard.

The Dashboard is a self-hosted task and project management tool that runs locally on your machine and integrates with AI coding agents and third-party services through OAuth.

---

## 1. Who is the Data Controller?

The Dashboard is a self-hosted application. **You are the data controller** for any information you put into your local instance, including tasks, projects, OAuth tokens, and connected accounts. We (the maintainers) do not operate any centralized backend that stores your task data.

If you are an organization deploying the Dashboard for your team, you are the data controller for your team's data. Your team members should be informed of how you operate that instance.

---

## 2. Information We Collect

### 2.1 Information stored locally on your machine

The Dashboard stores the following on your local filesystem (typically in `dev_assets/db.sqlite` or a path you configure via `DATABASE_URL`):

- **Account profile**: GitHub username, email, avatar URL, and OAuth identifiers obtained via GitHub OAuth device flow.
- **Project and task data**: Project names, descriptions, task titles, descriptions, statuses, comments, attachments, and any content you create in the Dashboard.
- **AI agent activity**: Logs and outputs of AI coding agent runs (Claude, Gemini, and other configured executors), including prompts, responses, and process logs streamed via Server-Sent Events.
- **Git worktree state**: Paths to git worktrees created for task execution, branch names, and diffs.
- **Integration tokens**: OAuth access tokens, refresh tokens, and metadata for third-party services you choose to connect (e.g., Google Drive, Gmail, Google Calendar, TikTok, and other social/OAuth integrations). Tokens are stored encrypted at rest using the OAuth token manager.
- **Configuration and preferences**: Environment variables, feature flags, and per-user UI preferences.

### 2.2 Information processed via connected third-party services

When you connect a third-party service through OAuth, the Dashboard accesses only the scopes you explicitly grant. Examples:

- **GitHub**: Repository metadata, issues, pull requests, and commit information for repositories you authorize.
- **Google Drive / Gmail / Google Calendar**: Files, emails, or events as scoped by Google's OAuth consent screen.
- **TikTok and social platforms**: Profile information and content publishing endpoints as scoped by the platform's OAuth consent.
- Any other OAuth provider you configure.

The Dashboard does **not** copy this data into a central database. Data is fetched on demand to fulfill the action you requested (e.g., scheduling a post, reading a calendar event) and may be cached locally for the duration of your session or as needed for the feature.

### 2.3 Optional analytics

If `POSTHOG_API_KEY` is configured at build time, the Dashboard sends product-usage events (page views, feature usage, errors) to PostHog. Analytics are **off by default** in self-hosted builds unless you have explicitly configured an API key. No task content or token material is sent to PostHog.

### 2.4 Information we do **not** collect

- We do not collect passwords. All authentication is via OAuth.
- We do not run a centralized cloud backend that stores your tasks.
- We do not sell, rent, or share your data with advertisers.
- We do not train AI models on your task content.

---

## 3. How We Use Information

The Dashboard uses the information described above to:

- Authenticate you and maintain your session.
- Display, edit, and synchronize your projects and tasks.
- Execute AI coding agents on your behalf and stream results back to your browser.
- Manage git worktrees and apply changes to your repositories.
- Call third-party APIs you have connected via OAuth, only for the action you requested.
- Send transactional emails (e.g., notifications) if you configure an email provider.
- Diagnose errors and improve product quality (only when analytics is explicitly enabled).

---

## 4. How Information Is Shared

- **AI agent providers**: When you run an AI agent action, the prompt and relevant repository context are sent to the agent provider you selected (e.g., Anthropic, Google). Their privacy policies apply to that processing.
- **OAuth providers**: Requests you initiate are sent to the connected provider (GitHub, Google, TikTok, etc.) under their terms.
- **No third-party sale**: We do not sell or share your data with advertisers, data brokers, or for cross-context behavioral advertising.
- **Legal**: As a self-hosted operator, you are responsible for responding to lawful requests for data on your instance.

---

## 5. Data Retention

- All Dashboard data lives on your machine until you delete it. Uninstalling the Dashboard or deleting `dev_assets/db.sqlite` removes the local database.
- OAuth tokens are retained until you disconnect the integration in the Dashboard UI or revoke access from the provider's account settings.
- Git worktrees are cleaned up automatically by the `WorktreeManager` service unless `DISABLE_WORKTREE_ORPHAN_CLEANUP` is set.
- Analytics events (if enabled) follow PostHog's retention configuration.

---

## 6. Your Rights and Controls

Because the Dashboard is self-hosted, you have direct control over your data:

- **Access**: All data is in the local SQLite database and the filesystem.
- **Correction**: Edit any record through the Dashboard UI or directly in the database.
- **Deletion**: Delete tasks, disconnect integrations, or remove the database file.
- **Portability**: Export data via the API or by copying the SQLite file.
- **Revocation**: Revoke OAuth tokens from the provider (e.g., GitHub → Settings → Applications, Google → Account → Security → Third-party access, TikTok → Settings → Authorized apps) and remove the corresponding integration in the Dashboard.

For users of an instance operated by an organization, contact your administrator to exercise these rights.

---

## 7. Security

- OAuth tokens are encrypted at rest by the OAuth token manager (`crates/services/src/services/oauth_crypto.rs`).
- Authentication uses industry-standard OAuth flows (GitHub device flow; provider-native OAuth for integrations). The Dashboard never sees or stores your provider passwords.
- Network communication with third-party APIs is encrypted in transit (TLS).
- Because the Dashboard runs locally, the security of the host machine is your responsibility. Use full-disk encryption, screen lock, and keep the OS patched.

No system is perfectly secure. Report suspected vulnerabilities to the maintainers via the project's issue tracker (security-sensitive issues should be reported privately).

---

## 8. Children's Privacy

The Dashboard is not directed to children under 13 (or under 16 in jurisdictions where that is the relevant threshold). We do not knowingly collect data from children. If you believe a child has used an instance you operate, delete the data via the controls in Section 6.

---

## 9. International Use

The Dashboard runs wherever you install it. If you connect third-party services, those providers may transfer data internationally under their own terms. You are responsible for compliance with local data-protection laws (GDPR, UK GDPR, CCPA/CPRA, LGPD, etc.) for the instance you operate.

---

## 10. Third-Party Services

The Dashboard integrates with services including, but not limited to:

- GitHub — https://docs.github.com/en/site-policy/privacy-policies/github-general-privacy-statement
- Google (Drive, Gmail, Calendar) — https://policies.google.com/privacy
- TikTok — https://www.tiktok.com/legal/page/row/privacy-policy/en
- Anthropic (Claude) — https://www.anthropic.com/legal/privacy
- Google Gemini — https://policies.google.com/privacy
- PostHog (optional analytics) — https://posthog.com/privacy

When you use these integrations, the third party's privacy policy governs their handling of your data.

---

## 11. Changes to This Policy

We may update this Privacy Policy as the Dashboard evolves. Material changes will be noted by updating the **Last updated** date at the top of this document and, where appropriate, surfaced in release notes.

---

## 12. Contact

For questions about this Privacy Policy or the Dashboard's data practices, contact the maintainers via the project's issue tracker, or — if you are a user of an instance operated by an organization — contact that organization's administrator.
