# `rustsecbot` action

This action uses [`cargo-deny`][cd] to find [`RUSTSEC`][rs] advisories that impact a Rust project.

## Inputs

None.

## Outputs

### `opened`

A comma-separated list of new advisory issues created in the form `ISSUE:ADVISORY`.

## Secrets

### `GITHUB_TOKEN`

This action reads & writes issues using the GitHub API.

## Example usage

```yaml
permissions:
  issues: write

jobs:
  rustsec:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v1
      - uses: olix0r/rustsecbot@v1
```

<!-- refs -->
[cd]: https://github.com/EmbarkStudios/cargo-deny
[rs]: https://rustsec.org
