# Release inputs manifest example

`manifest.example.json` shows the `armorer-release-inputs` document that
`verify-release` and `publication` accept in place of their individual input flags.
**Every digest, path and tool here is a placeholder. Do not use it for a release.**

```sh
armorer verify-release --manifest manifest.json --expect-manifest-sha256 '<independently approved SHA-256>'
armorer publication --manifest manifest.json --expect-manifest-sha256 '<independently approved SHA-256>' plan
```

* The digest typed on the command line is the approval. Armorer hashes the exact
  manifest bytes and compares them before parsing the file or opening any path in it.
  Reviewing the manifest means reviewing the digests inside it; do not take the digest
  from the same place as the manifest.
* A relative path resolves against the directory that contains the manifest, never the
  working directory. An absolute path is used as written.
* `publication` is optional and ignored by `verify-release`; every `publication`
  subcommand requires it.
* The manifest does not make verification weaker or stronger. It moves the same values
  from flags into one reviewed file. Use it instead of, never together with, the
  individual input flags.

The schema is `armorer schema release-inputs-manifest`.
