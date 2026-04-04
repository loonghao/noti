# Changelog

## [0.1.9](https://github.com/loonghao/noti/compare/v0.1.8...v0.1.9) (2026-04-04)


### Features

* **providers:** enhance all providers with new features from issues ([#42](https://github.com/loonghao/noti/issues/42)) ([5dde218](https://github.com/loonghao/noti/commit/5dde218d2c492e751955044f1800f627133bfcb6))

## [0.1.8](https://github.com/loonghao/noti/compare/v0.1.7...v0.1.8) (2026-04-03)


### Miscellaneous Chores

* merge origin/main into auto-improve (resolve conflicts) [iteration-done] ([1697b1c](https://github.com/loonghao/noti/commit/1697b1c772e5919afc70d96e4db98da2446c0e99))

## [0.1.7](https://github.com/loonghao/noti/compare/v0.1.6...v0.1.7) (2026-04-03)


### Bug Fixes

* **ci:** fix clawhub publish command and docker arm64 build ([8214430](https://github.com/loonghao/noti/commit/82144302552f32530c3875415c90e538c0526f04))

## [0.1.6](https://github.com/loonghao/noti/compare/v0.1.5...v0.1.6) (2026-04-03)


### Bug Fixes

* **ci:** correct clawhub publish command and improve Docker build robustness ([6fb881c](https://github.com/loonghao/noti/commit/6fb881c62d0777c8f23bdfe9fb91b023bb7e56fc))

## [0.1.5](https://github.com/loonghao/noti/compare/v0.1.4...v0.1.5) (2026-04-01)


### Features

* **cli:** agent-first CLI refactoring based on Google best practices ([9dc6981](https://github.com/loonghao/noti/commit/9dc69815d6e3b4a0fcf1c26b543e5da329b7d53a))
* squash merge auto-improve + fix clawhub publish version ([ffbb116](https://github.com/loonghao/noti/commit/ffbb11650365b7c03cb80de9923e031566f40b07))
* squash merge auto-improve incremental changes ([a88ac49](https://github.com/loonghao/noti/commit/a88ac49c3924cc82cf0c4dcc9e46842f042b5467))


### Bug Fixes

* **ci:** use correct clawhub publish command instead of clawhub skill publish ([2be1ae5](https://github.com/loonghao/noti/commit/2be1ae598700558b98a406cec30ad01b8fa261df))

## [0.1.4](https://github.com/loonghao/noti/compare/v0.1.3...v0.1.4) (2026-04-01)


### Features

* squash merge auto-improve branch into release/auto-improve ([1bf3bde](https://github.com/loonghao/noti/commit/1bf3bde80b15348e4df344a8bd060f9335e436b7))


### Documentation

* enhance README with rich badges, hero section and bilingual sync ([bae6b68](https://github.com/loonghao/noti/commit/bae6b687309762000cdc2127f1966bbcec3280ec))


### Miscellaneous Chores

* **cleanup:** lint: fix clippy await_holding_lock warning in e2e_test ([13e9001](https://github.com/loonghao/noti/commit/13e90010b6718442ce98924c55de1dbad2cf5223))

## [0.1.3](https://github.com/loonghao/noti/compare/v0.1.2...v0.1.3) (2026-03-30)


### Features

* add attachment support for multiple providers ([27d6cce](https://github.com/loonghao/noti/commit/27d6cce8de9c728698b97147b705c9f86984d6f2))
* add file attachment support across 100+ providers ([5739b82](https://github.com/loonghao/noti/commit/5739b82837a8d398dba0133ff80989b1adddb2db))
* **providers:** add attachment support for googlechat, flock, gitter, and seven ([b843abb](https://github.com/loonghao/noti/commit/b843abb66dd21c7f941c02c72e8c8b02354b6912))
* **providers:** add MMS attachment support for httpsms and update tests ([30baacd](https://github.com/loonghao/noti/commit/30baacdb92ab5c1cab7b6ab42099155f922d7dd7))
* **providers:** implement real attachment handling for signl4 and synology ([4e9aadd](https://github.com/loonghao/noti/commit/4e9aadd75ac9b256b693726e728a2101c8a101f4))
* set supports_attachments=true for 44 providers with attachment handling code ([80d14db](https://github.com/loonghao/noti/commit/80d14db6c683526f20e691c8c708c52a238fa140))


### Bug Fixes

* add missing tempfile dev-dependency to noti-core ([5d256f5](https://github.com/loonghao/noti/commit/5d256f516897451f1d2acacffde979058ba73618))
* add missing url dev-dependency and fix EmailProvider::default() ([3e74913](https://github.com/loonghao/noti/commit/3e749134f131265c999fafe1b2cce9b1630ece1f))
* replace assert_eq!(x, true) with assert!(x) ([139afb0](https://github.com/loonghao/noti/commit/139afb0339a027754bd5e4a8a70534fad117252c))
* resolve all clippy needless_borrows and len_zero warnings ([080bef4](https://github.com/loonghao/noti/commit/080bef4772546ac755e26b24394255ac544077a0))
* resolve clippy warnings in email, nextcloud, twitter providers ([5101ae6](https://github.com/loonghao/noti/commit/5101ae6b2af031877de3754a2228e7a8c6f6e7e8))
* set supports_attachments to false for 49 providers using data URI hacks ([0adc208](https://github.com/loonghao/noti/commit/0adc208edb716e611313dcf4f8e89eaf0f4a46cd))
* use derive(Default) for AttachmentKind and Priority enums ([7f8e634](https://github.com/loonghao/noti/commit/7f8e634a34e6eb6ca59c15836a403452d6d97196))
* **voipms:** set supports_attachments to true to match actual MMS image handling code ([f57abc6](https://github.com/loonghao/noti/commit/f57abc6ca140b059a8eb26fd791dc6cd182dfdf5))


### Code Refactoring

* clean up data URI hacks from webhook-only providers ([ccd27e3](https://github.com/loonghao/noti/commit/ccd27e30b47d5d05c46150a01fe61b4383d248db))


### Documentation

* add VitePress documentation site with GitHub Pages CI ([1171e4d](https://github.com/loonghao/noti/commit/1171e4d9df0e1782cc4aebc3aaf1aa58e0952f6e))

## [0.1.2](https://github.com/loonghao/noti/compare/v0.1.1...v0.1.2) (2026-03-26)


### Features

* add 120+ notification providers covering chat, push, SMS, email, webhook, incident and IoT channels ([c511556](https://github.com/loonghao/noti/commit/c511556790bc586c24462311db54373d3d481f2a))
* add CI/CD workflows, install scripts, OpenClaw skills, Apprise URL aliases, and comprehensive tests ([d5fde78](https://github.com/loonghao/noti/commit/d5fde7877a66b2388b260b5e79bd742f3ed70add))

## [0.1.1](https://github.com/loonghao/noti/compare/v0.1.0...v0.1.1) (2026-03-24)


### Features

* initial project setup with 7 notification providers ([94092fa](https://github.com/loonghao/noti/commit/94092fa7d4beca5465d4a1f7850a203ee1bc5f78))


### Miscellaneous Chores

* update repo references from wecom-bot-cli to noti ([94de3aa](https://github.com/loonghao/noti/commit/94de3aad62b981d86f7c3d1f050a2b4f613f43da))
