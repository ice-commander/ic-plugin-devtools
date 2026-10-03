# Developer Toolbox

An Ice Commander plugin: hashes, encodings and JSON formatting in one dialog.

It claims no files and mounts nothing. At init it registers its locales, a view
named `devtools` and a toolbox button on the left of the header bar; the button
opens the view as a dialog.

## The dialog

The plugin has no UI toolkit code. `describe` returns the dialog as a JSON view
document, the host builds it with its own widgets, and input changes and the
Copy click come back to `on_event`, which answers with the new values.

From top to bottom: a picker of eleven tools, a multiline **Input**, the input
size in bytes with a **Copy** button, a read-only **Result**, a **Compare with**
field shown only while a checksum tool is selected, and a status line.
Input and Compare with are re-evaluated 120 ms after the last change. Copy puts
the Result on the clipboard and shows "Copied"; with an empty Result it does
nothing.

| id | result |
|---|---|
| `hash.md5` `hash.sha1` `hash.sha256` `hash.sha512` | digest, lower-case hex |
| `hash.crc32` | CRC32, eight hex digits |
| `encoding.base64_encode` | standard alphabet, padded |
| `encoding.base64_decode` | accepts standard or URL-safe, padded or not, whitespace ignored |
| `encoding.hex_encode` | lower-case hex |
| `encoding.hex_decode` | ignores whitespace, `:` and `-` |
| `format.json_pretty` | indented by two spaces |
| `format.json_minify` | no whitespace between tokens |

Hashes and encoders work on the UTF-8 bytes of the input. A failed decode or
parse clears the Result and puts the reason in the status line in the error
colour; JSON errors are serde_json's messages with line and column.

Compare with: case is ignored and surrounding whitespace trimmed. A match shows
"matches", a mismatch "does not match" in the error colour, an empty field
shows nothing.

Fifteen catalogues in `src/devtools-dlg/locales/` are registered at init.
Labels in the document carry an English fallback; tool names fall back to the
tool id.

## Layout

- `src/devtools-dlg/` — the plugin crate (`ic-devtools-dlg`): the view document,
  event replies and C entry points.
- `src/devtools-dlg/core/` — `devtools-core`: the tools, with no plugin
  interface or toolkit dependency. Most tests are here.

## Build, test, deploy

```sh
./build.sh          # cargo build --release, copies the libraries from bin/target/release to bin/
./test.sh           # cargo test --workspace
./deploy-local.sh   # copies bin/ libraries into the plugin folder
```

`deploy-local.sh` targets `~/Library/Application Support/ice-commander/plugins`
on macOS, `${XDG_DATA_HOME:-~/.local/share}/ice-commander/plugins` on Linux and
`%APPDATA%/ice-commander/plugins` on Windows; `IC_PLUGIN_DIR` overrides it. It
removes libraries it deployed earlier (listed in `bin/.deployed`) that are no
longer built. Then switch the plugin on in **Settings → Plugins** and restart.

The version comes from `package.json`; `npm run gen-version` regenerates
`version.rs`. `build.sh` does not run it.

`ic-plugin-api` is a git dependency on
`https://github.com/ice-commander/plugin-api.git`, branch `main`.

## Known limitations

- No tool search. `core/src/search.rs` (ranking a tool by id, title and
  keywords) has no caller outside its tests; the key
  `devtools.search_placeholder` is unused.
- No file input: tools work only on typed or pasted text. `ToolSpec::takes_file`
  and the key `devtools.use_file` are not read outside the core's tests.
- A decode whose output is not UTF-8 fails with the untranslated status
  "not text"; the key `devtools.not_text` is unused. Base64, hex and JSON error
  messages are English only.
- `ToolSpec::shape` is not used: every result goes to the multiline Result.
- URL-safe Base64 encoding (`Alphabet::UrlSafe`) and `json::validate` are not
  reachable from the dialog.

## Licence

MIT or Apache-2.0, at your option. Contributions are taken under the
[DCO](DCO); sign off with `git commit -s`. Icons: see
[THIRD-PARTY-LICENSES.md](THIRD-PARTY-LICENSES.md).
