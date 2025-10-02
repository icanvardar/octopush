# octopush

Profile management for Git repositories. Add, use, and reset per-repo identities with SSH or GitHub authentication.

## Install

```
cargo install octopush
```

## Quick Start

- Add a profile interactively:
  `octopush add-profile`
- Apply a profile to the current repo:
  `octopush use-profile --profile-name work`
- See the current repo profile:
  `octopush get-profile`

## Commands

- `octopush add-profile [--profile-name <n>] [--name <n>] [--email <e>] [--auth-type <none|ssh|gh>] [--hostname <h>] [--ssh-key-path <p>]`
- `octopush delete-profile --profile-name <name>`
- `octopush list-profiles`
- `octopush use-profile --profile-name <name>`
- `octopush get-profile`
- `octopush reset-profile`
- `octopush --help`

Examples:

```
# SSH
octopush add-profile --profile-name work --name "John Doe" --email john@doe.com --auth-type ssh --ssh-key-path ~/.ssh/id_ed25519

# GitHub CLI auth
octopush add-profile --profile-name oss --name "John Doe" --email john@doe.com --auth-type gh --hostname github.com
```

## Contributing

- Issues and PRs are welcome.
- Run tests with `cargo test`.
- Keep changes focused and well scoped.
