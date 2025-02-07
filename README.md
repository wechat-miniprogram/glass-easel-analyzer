# glass-easel-analyzer

A Language Server for glass-easel and glass-easel-miniprogram-adapter

It handles MiniProgram code structure, i.e. WXML/WXSS files.

*Still in early development.*


## Development Guide

### Preparation

`node.js` and `rust` toolchains are needed.

* Open this project in Visual Studio Code.
* Ensure all extensions in [.vscode/extensions.json](.vscode/extensions.json) been installed.

Furthermore, please read through [extension publishing guide for VSCode](https://code.visualstudio.com/api/working-with-extensions/publishing-extension) and install `vsce` globally.

### Initialize Development

Run `vscode-extension/build/init.sh` (or `.bat` for Windows) to cleanup and (re)initialize development.

### Debug

Press `F5` (or go to `Run and Debug` side panel and `Run Extension`). An extension debug window will popup.

### Run Tests

In command palette, input `Tasks: Run Task` command and then select the `tasks: watch-tests` task. Then it will keep running in the `terminal` tab.

Then go to `Test` side panel and run tests. Results will be listed in the `test results` tab.

### Run Tests in Terminal

`npm test` is available in `vscode-extension` directory.

Run with `TEST_OVERWRITE_SNAPSHOT=1` to overwrite test snapshots.

### Packaging

Execute `vscode-extension/build/[PLATFORM].sh` (or `.bat` for Windows) to build packages for the specified platform.

### Publish

After packaging, `vscode-extension/build/publish.sh [VERSION]` can be used to publish the packages of the specified version.

However, the regular publish should be done through [this GitHub Action](https://github.com/wechat-miniprogram/glass-easel-analyzer/actions/workflows/vscode-extension.yml).
