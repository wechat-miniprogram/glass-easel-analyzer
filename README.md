# glass-easel-analyzer

A Language Server for glass-easel and glass-easel-miniprogram-adapter

It handles MiniProgram code structure, i.e. WXML/WXSS/JSON files.

*Still in early development.*


## Development Guide

### Preparation

`node.js` `pnpm` and `rust` toolchains are needed.

* Open this project in Visual Studio Code.
* Ensure all extensions in [.vscode/extensions.json](.vscode/extensions.json) been installed.

### Debug

Press `F5` (or go to `Run and Debug` side panel and `Run Extension`). An extension debug window will popup.

### Run Tests

In command palette, input `Tasks: Run Task` command and then select the `tasks: watch-tests` task. Then it will keep running in the `terminal` tab.

Then go to `Test` side panel and run tests. Results will be listed in the `test results` tab.

### Run Tests in Terminal

`npm test` is available.

Run with `TEST_OVERWRITE_SNAPSHOT=1` to overwrite test snapshots.
