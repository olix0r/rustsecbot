# `rustsecbot` action

This action uses [`cargo-deny`][cd] to find [`RUSTSEC`][rs] advisories that
impact a Rust project.

## Inputs

### `labels`

A comma-separated list of labels for impacted issues.

Default: _rust,security_

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
        with:
          labels: area/myapp,rust,security
```

<!-- refs -->
[cd]: https://github.com/EmbarkStudios/cargo-deny
[rs]: https://rustsec.org
