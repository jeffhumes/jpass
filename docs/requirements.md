# JPass Requirements

This document contains planned product, security, platform, and usability requirements for JPass.

New requirements are recorded here and in the session project memory so they remain available both in source control and across assistant sessions.

## Requirement Status

- `[ ]` Planned or not yet implemented
- `[~]` In progress or partially implemented
- `[x]` Implemented, but retained here for future review

Completed requirements should remain in this document. Revisit them when related code, dependencies, platform adapters, or security assumptions change.

Existing requirements without a marker are treated as planned until explicitly reviewed and given a status.

## Product Scope

- Support Windows, macOS, Linux, Android, iOS, and web-compatible builds where feasible.
- All changes must preserve shared behavior across supported platforms. Platform-specific implementations must follow shared contracts.
- Provide installers or native distribution packages for every supported platform in addition to standalone build artifacts.

## Vault and Data

- Store more than username/password pairs, including SSH keys, API tokens, recovery codes, certificates, and other sensitive fields.
- Support multiple separate vaults, let users choose a vault at startup, and allow switching between vaults through an application menu option.
- [~] Allow optional, user-controlled vault synchronization across supported platforms. Shared encrypted sync envelopes, a desktop local-folder provider, native desktop folder picker, Settings controls, Sync Now with remote-newer protection, and desktop remote restore are implemented; full conflict resolution and other transports remain pending.
- [~] Support local vault export and backup. Encrypted desktop backups are implemented; native file destinations and additional export formats remain pending.
- [~] Support restoring from a backup with validation before applying changes. Desktop local-file and sync-based restore now validate before replacement and create a safety backup; cherry-pick restore and other transports remain pending.
- Allow users to cherry-pick individual entries, folders, or selected backup content to restore instead of requiring a full-vault restore.
- Let users choose encrypted/password-protected or plain-text export output.
- Support formats such as XML, JSON, CSV, TSV, and raw data.
- Support credential export and import across supported platforms and formats.
- Validate imported data and provide safe conflict handling.
- [~] Support nested folders with parent and child levels, such as `Personal/Finances`, instead of limiting entries to a single folder level. Nested creation, indented navigation, descendant filtering, folder selection, and individual/all expand-collapse controls are implemented; nested rename/delete management remains pending.
- [x] When adding an entry, let users select an existing folder or create a new folder for the entry before saving.
- Provide a folder manager that lets users add, delete, and rename folders, and move entries between folders.
- Allow users to share selected vault entries via text, with explicit confirmation, sensitivity warnings, and secure handling.
- Keep a history of previous passwords per entry so users can refer back to them.

## Authentication and Recovery

- Provide a secure way for users to recover a forgotten master password. The recovery method remains to be designed.
- Allow users to configure a PIN as an alternative unlock method for a user-defined period after master-password authentication, with the PIN validity duration adjustable in Settings.
- Treat the PIN as a temporary, device-local unlock mechanism with secure storage, expiration, retry protections, and revocation when the master password or security settings change.
- Phase 2: support passwordless login with passkeys and hardware security keys.
- Phase 2 passwordless authentication must include secure enrollment, key management, recovery, revocation, and cross-platform compatibility.

## Security

- [~] Always use strong, modern encryption for vault data, backups, synchronization, and sensitive storage or transport. Vault encryption is implemented; backup and synchronization encryption remain pending.
- [x] New vault encryption uses AES-256-GCM with OS-provided randomness and explicit Argon2id key-derivation parameters.
- [x] Encryption changes must preserve access to existing vaults through a documented migration path.
- Never weaken encryption for convenience.
- Check generated and manually entered passwords for weakness, common patterns, reuse indicators, and likely crackability.
- Use local or privacy-preserving password checks so secrets are not exposed externally.
- Perform password checks before accepting or storing passwords where practical.

## Password Generator

- [~] Provide an easy-to-use standalone password generator without requiring a vault entry. Standalone generation, customization, and copy are implemented; generated-password history remains pending.
- [x] Allow users to adjust generated password length.
- [x] Allow users to choose character sets, including special characters, numbers, lowercase letters, and uppercase letters.
- [x] Let users configure how passwords are generated in the Edit Entry dialog: auto-generate directly or open the full password generator.
- [x] Allow users to configure password-generator defaults in Settings, including default length and enabled character classes.
- Support extensible generation options.
- Provide a user-controlled history of generated passwords.
- Store password history securely and allow users to clear it.

## Save, Autofill, and Context Awareness

- Support Save functionality for capturing credentials.
- Support Autofill across compatible applications and browser workflows.
- Nice to have: context-aware credential auto-input based on the application or authentication target.
- Context-aware auto-input must use secure matching, explicit user control, and platform permissions or accessibility APIs.

## Access Control

- Allow users to designate trusted users.
- Grant trusted users explicitly selected elevated access to vault entries rather than unrestricted access by default.
- Use strong authentication and least-privilege permissions for trusted-user access.

## User Experience

- [x] Provide a native categorized application menu with File, Edit / Settings, and Help menus exposing the primary actions.
- [x] Separate the Settings dialog into readable sections with a clickable navigation menu on the left.
- [x] Prompt users for explicit confirmation before deleting a vault entry by default.
- [x] Require the delete confirmation dialog to offer a validated encrypted vault backup before deletion, while allowing an explicit delete-without-backup choice.
- [x] Provide a settings option to enable or disable the delete-confirmation prompt.
- Keep delete behavior consistent across supported platforms.
- [x] Let users choose whether entry-list actions display descriptive text buttons or compact icons, with the preference available in the Settings dialog.
- [x] Let users choose in Settings whether primary actions use descriptive text buttons or compact icons.
- [x] Let users hide primary action buttons from the toolbar with the Show primary action icons setting.
- [x] Use a lock icon for the Lock primary action and a folder icon for the New Folder primary action.
- [x] Keep primary action icons consistent: Settings, Backup, and Lock use the same background and hover styling as the other primary icons.
- [x] Remove the folder dropdown from entry rows and replace it with a dedicated Move button or icon.
- [x] Allow the Move Entry dialog to create a new folder and move the selected entry into it.
- [x] Replace the separate “Or create a new folder” fields in Move Entry and Edit Entry with a Create Folder icon beside the folder dropdown.
- [x] Keep Clipboard control honors the selected display theme with matching wrapper, label, select, focus, border, and surface styling.
- [x] Remove the Keep Clipboard timeout dropdown from the main screen and place it in the Settings dialog.
- [x] Copy username/password toast notifications show a working countdown and automatically clear the clipboard when the timer expires.
- [x] Toast notifications are stackable so rapid actions such as Copy User followed by Copy Password display simultaneously with independent countdowns.
- [x] Allow users to choose the toast-notification position in Settings: any display corner, top-center, center, or bottom-center.
- Deferred Linux issue: investigate why the GTK/WebKit desktop build may render but not accept clicks in the master-password field or buttons. Current startup workarounds disable WebKit DMA-BUF/compositing, use software GL, and disable Dioxus debug always-on-top; a native Linux renderer or deeper GTK/WebKit investigation may be needed.

## Phase 2 Security Monitoring

- Provide a password-health dashboard.
- Detect and clearly flag compromised, reused, and weak credentials.
- Use privacy-preserving analysis and provide actionable remediation guidance.
- Monitor breach and dark-web sources for compromised credentials associated with multiple user-approved email addresses.
- Keep monitoring opt-in and user-controlled.

## Phase 2 Privacy Integrations

- Consider VPN integration or launch/control options for providers such as Hotspot Shield, NordVPN, and other compatible services.
- Use a provider-neutral architecture.
- Require explicit opt-in and provide clear privacy disclosures.
