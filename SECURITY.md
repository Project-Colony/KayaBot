# Security Policy

## Supported versions

Only the **latest release** receives security fixes. KayaBot ships as a single
binary, through Colony and the release page, so the fix for a vulnerability is
the next release, not a patch to an older one.

## Reporting a vulnerability

Please report vulnerabilities **privately** via
[GitHub Security Advisories](https://github.com/Project-Colony/KayaBot/security/advisories/new)
("Report a vulnerability"). Do not open a public issue for exploitable bugs.

Include the version (`kayabot --version`), the platform, and the steps or the
file name that reproduce it. The report stays private until a fixed release is
out.

## Scope

Reports of particular interest:

- **Your files.** KayaBot renames and moves files on disk. A file name or a
  metadata answer that makes it overwrite, delete or lose a file, or write
  outside the file's own folder, is a vulnerability.
- **API keys.** KayaBot keeps them in `api_keys.toml`, readable by your user
  only on Linux and macOS, and sends each key only to its own service. A key
  that reaches another host, an error message, a log or a world-readable file
  is a vulnerability.
- **Untrusted input.** File names and the JSON returned by TMDB, TheTVDB,
  TVmaze and OMDb are parsed without trust. A crash, a hang or a path built
  from that input that escapes the file's folder is in scope.
- **Release signatures.** Every release asset is signed with the organisation's
  ed25519 key (see [Code signing policy](README.md#code-signing-policy)).
  Anything that gets an unsigned or altered asset accepted is critical.
