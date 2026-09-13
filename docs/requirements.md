# JPass Requirements

This document contains planned product, security, platform, and usability requirements for JPass.

New requirements are recorded here and in the session project memory so they remain available both in source control and across assistant sessions.

## Product Scope

- Support Windows, macOS, Linux, Android, iOS, and web-compatible builds where feasible.
- All changes must preserve shared behavior across supported platforms. Platform-specific implementations must follow shared contracts.
- Provide installers or native distribution packages for every supported platform in addition to standalone build artifacts.

## Vault and Data

- Store more than username/password pairs, including SSH keys, API tokens, recovery codes, certificates, and other sensitive fields.
- Allow optional, user-controlled vault synchronization across supported platforms.
- Support local vault export and backup.
- Let users choose encrypted/password-protected or plain-text export output.
- Support formats such as XML, JSON, CSV, TSV, and raw data.
- Support credential export and import across supported platforms and formats.
- Validate imported data and provide safe conflict handling.
- Allow users to share selected vault entries via text, with explicit confirmation, sensitivity warnings, and secure handling.

## Authentication and Recovery

- Provide a secure way for users to recover a forgotten master password. The recovery method remains to be designed.
- Phase 2: support passwordless login with passkeys and hardware security keys.
- Phase 2 passwordless authentication must include secure enrollment, key management, recovery, revocation, and cross-platform compatibility.

## Security

- Always use strong, modern encryption for vault data, backups, synchronization, and sensitive storage or transport.
- Never weaken encryption for convenience.
- Check generated and manually entered passwords for weakness, common patterns, reuse indicators, and likely crackability.
- Use local or privacy-preserving password checks so secrets are not exposed externally.
- Perform password checks before accepting or storing passwords where practical.

## Password Generator

- Provide an easy-to-use standalone password generator without requiring a vault entry.
- Allow users to adjust generated password length.
- Allow users to choose character sets, including special characters, numbers, lowercase letters, and uppercase letters.
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

- Prompt users for explicit confirmation before deleting a vault entry by default.
- Provide a settings option to enable or disable the delete-confirmation prompt.
- Keep delete behavior consistent across supported platforms.

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
