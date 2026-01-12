# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## 0.1.0 (2026-01-12)

### Added

- Initial release of pctx_executor

### Commit Statistics

<csr-read-only-do-not-edit/>

 - 30 commits contributed to the release over the course of 45 calendar days.
 - 0 commits were understood as [conventional](https://www.conventionalcommits.org).
 - 0 issues like '(#ID)' were seen in commit messages

### Commit Details

<csr-read-only-do-not-edit/>

<details><summary>view details</summary>

 * **Uncategorized**
    - Init changelogs ([`60b8c14`](https://github.com/portofcontext/pctx/commit/60b8c14b41da72b74a843c1e4a20297ddc17f364))
    - Update crates setup for crates.io publishing ([`92502a4`](https://github.com/portofcontext/pctx/commit/92502a46c7b006023fb767796600cc0267fbf5e0))
    - Merge pull request #43 from portofcontext/more-instrumentation ([`b26329d`](https://github.com/portofcontext/pctx/commit/b26329dc8d135865073090b2330d6f9c54404f69))
    - Merge branch 'main' into more-instrumentation ([`f43a433`](https://github.com/portofcontext/pctx/commit/f43a43397d9030a8b7bf24a9cf6c86d6f248f474))
    - Check len before register ([`e7c82f3`](https://github.com/portofcontext/pctx/commit/e7c82f3cfd2e6dbf746267c09ec223662abb0e56))
    - Merge pull request #45 from portofcontext/debug-windows-issue-from-dep-updates ([`0bc2c73`](https://github.com/portofcontext/pctx/commit/0bc2c73e1ed4ad5c087e7c4a5bd6fa9a3f136be4))
    - Switch feature to Win32_Networking_WinSock ([`c0e520f`](https://github.com/portofcontext/pctx/commit/c0e520f7c6e44a8de2b75ea0cc6d29b5e9271f9f))
    - Try adding Win32_Networking feature to pctx_executor.windows-sys ([`d518e22`](https://github.com/portofcontext/pctx/commit/d518e222feea879df513b02ff48b2194ec654109))
    - Merge pull request #44 from portofcontext/handle-mcp-errs-and-stdio ([`9732aad`](https://github.com/portofcontext/pctx/commit/9732aade68e6ac34a46ddea158f1bcb30457549d))
    - Merge branch 'main' into handle-mcp-errs-and-stdio ([`363a3f0`](https://github.com/portofcontext/pctx/commit/363a3f082c38e2b1c509e02321711c6d04291d7c))
    - Merge branch 'main' into more-instrumentation ([`bdbe6b5`](https://github.com/portofcontext/pctx/commit/bdbe6b5301868f8de51530848bfea373e6a86bbf))
    - Merge pull request #42 from portofcontext/bugfix/ts-compiler-err-msg-handling ([`7c96a40`](https://github.com/portofcontext/pctx/commit/7c96a408b76d12bc18c0e79ce6ce2c40dcc2ab2f))
    - Pr comments ([`37c2f44`](https://github.com/portofcontext/pctx/commit/37c2f44386bd777642351181d12d8d8a088d53ab))
    - Improve handling of ts compiler errors ([`80858a9`](https://github.com/portofcontext/pctx/commit/80858a95b95d6fb7c625c1d884093d656c7e56c4))
    - Add stdio MCP support ([`21d2d92`](https://github.com/portofcontext/pctx/commit/21d2d92886b6c36b79a24ba3d1e9596ae8d9324a))
    - Merge pull request #38 from portofcontext/dependency-bumps-dec-12 ([`5ef7aef`](https://github.com/portofcontext/pctx/commit/5ef7aefb11d22f09330af9cf23bd1341448f7c3a))
    - Various dependency updates for compatability ([`0a2a3e8`](https://github.com/portofcontext/pctx/commit/0a2a3e8f18d4f366ef8e4051b8f79d99ce80a86c))
    - Merge pull request #29 from portofcontext/feature/py-lib ([`0a09de6`](https://github.com/portofcontext/pctx/commit/0a09de66e3afd2b5072a198468bdbcbae117c738))
    - Delete unused crate and add architecture diagram ([`1d49697`](https://github.com/portofcontext/pctx/commit/1d4969747ccdcab5d8341d716680fa07b6d18426))
    - Merge main ([`141137e`](https://github.com/portofcontext/pctx/commit/141137e9c2e66e5ce60b09a201a71b82feaa49f4))
    - Merge branch 'callbacks-eap' of https://github.com/portofcontext/pctx into callbacks-eap ([`8fb7057`](https://github.com/portofcontext/pctx/commit/8fb7057dd70497d9451ee683cd3d7c8ee246208c))
    - CallMCPTool explicit ([`b6a792c`](https://github.com/portofcontext/pctx/commit/b6a792cd7328b7f05ab723171fff548bcbfb06db))
    - Executor callback tests ([`b80ad65`](https://github.com/portofcontext/pctx/commit/b80ad65f01218d2ec59252e7bac141ae0d101b10))
    - Add callback_registry & simplify ([`c2ac47f`](https://github.com/portofcontext/pctx/commit/c2ac47f64c63b423e724c8d97a0fda4cb0b0ac74))
    - Move session out of executor ([`7722bc4`](https://github.com/portofcontext/pctx/commit/7722bc4be6b8ef8e62606cc331e06eff298c9ff8))
    - Restructuring ([`194c3d3`](https://github.com/portofcontext/pctx/commit/194c3d314497d35e89e74a5881488bfb963750ff))
    - Merged pctx-py ([`fdb30c5`](https://github.com/portofcontext/pctx/commit/fdb30c58223d31ba67ff10e29c5f5f9238be50f0))
    - Progress on websocket handling ([`1858be2`](https://github.com/portofcontext/pctx/commit/1858be2ad0c2afcb5abee006f4987121da3d9000))
    - Websocket refactor ([`e0e1ffb`](https://github.com/portofcontext/pctx/commit/e0e1ffb658f8cb4fdfa1add0c5b7594c013ccd6d))
    - Setup python bindings and some renaming ([`e5b8755`](https://github.com/portofcontext/pctx/commit/e5b8755f3c16499e61f5ce1be09f9c8a941504ab))
</details>

