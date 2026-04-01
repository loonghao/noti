# Changelog

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
