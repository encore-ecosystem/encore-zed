# Zed Encore

A [Encore](https://github.com/encore-ecosystem/encore) extension for [Zed](https://zed.dev).

## Development

To develop this extension, see the [Developing Extensions](https://zed.dev/docs/extensions/developing-extensions) section of the Zed docs.

When testing unpublished grammar changes from a sibling `tree-sitter-encore`
checkout, generate and test that checkout with its pinned CLI, then build the
local Wasm grammar (the build command requires a CLI with WASI support):

```sh
(cd ../tree-sitter-encore && node_modules/.bin/tree-sitter generate && node_modules/.bin/tree-sitter test)
tree-sitter build --wasm --output grammars/encore.wasm ../tree-sitter-encore
```

Reload the dev extension after changing its manifest version, or restart Zed, so
the old grammar is not retained in memory. Before publishing the extension, commit
and publish the grammar and update `grammars.encore.commit` to that exact revision;
the local Wasm override is ignored by Git and is not a release dependency.

## Language Server

The extension starts the Encore language server automatically for `.enq` files.
The matching stable server is downloaded from Encore releases when no local
binary is configured or available on `PATH`.

To use a locally built server instead, install it first:

```sh
cd /path/to/encore-index/lsp
encore install --path . --name encore-lsp --force --profile release
```

By default this copies the executable to:

```text
~/.encore/bin/encore-lsp
```

The extension resolves the server in this order:

1. `lsp.encore-lsp.binary.path` in Zed settings
2. `ENCORE_LSP_PATH` from the worktree shell environment
3. `encore-lsp` from the worktree `PATH`
4. the latest stable Encore release for the current platform

Zed settings can also supply arguments and environment variables:

```json
{
  "lsp": {
    "encore-lsp": {
      "binary": {
        "path": "/absolute/path/to/encore-lsp",
        "arguments": [],
        "env": { "ENCORE_CORE_DIR": "/optional/core/path" }
      }
    }
  }
}
```
