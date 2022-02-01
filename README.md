# `rustsecbot` action

This action uses [`cargo-deny`][cd] to find [`RUSTSEC`][rs] advisories that
impact a Rust project.

## Inputs

### `config`

A YAML-formatted configuration file.

Default: `.github/rustsecbot.yml`.

#### Example config file

```yaml
# Labels that apply to all rustsecbot issues.
labels:
  - lang/rust
  - security

# Per-crate configuration. rustsecbot uses the top crate in the deepest
# dependency path to the impact crate as the key.
crates:
  foo:
    labels: ["crate/foo"]
  bar:
    labels: ["crates/bar"]
```

### `token`

A GitHub PAT with the `issues:write` scope.

Default: `github.token`.

## Outputs

### `opened`

A comma-separated list of new advisory issues created in the form
`ISSUE:ADVISORY`.

## Example usage

```yaml
permissions:
  issues: write

jobs:
  rustsec:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v2
      - uses: olix0r/rustsecbot@v1
```

<!-- refs -->
[cd]: https://github.com/EmbarkStudios/cargo-deny
[rs]: https://rustsec.org
