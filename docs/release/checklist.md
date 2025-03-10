# Release Checklist

- Using the latest version of branch `develop`.
- Appropriate fields of `RuntimeVersion` instance are bumped for the parachain runtime.
- Necessary migrations are properly added to the parachain runtime.
- New testnet chainspec is generated.
- Runtime incompatibilities are analyzed with `subwasm diff`.
