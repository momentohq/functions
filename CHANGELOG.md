# Changelog

## [0.21.0](https://github.com/momentohq/functions/compare/v0.20.1...v0.21.0) (2026-04-17)


### Features

* add v2 support for momento http crate ([#76](https://github.com/momentohq/functions/issues/76)) ([ad24a36](https://github.com/momentohq/functions/commit/ad24a36e8a1e3473c658d80294935085fba2d04f))

## [0.20.1](https://github.com/momentohq/functions/compare/v0.20.0...v0.20.1) (2026-04-17)


### Miscellaneous

* carry over v1 list optimizations to v2 ([#77](https://github.com/momentohq/functions/issues/77)) ([3f32d73](https://github.com/momentohq/functions/commit/3f32d73f9114a0b5127ca6de48811c6786ecf812))

## [0.20.0](https://github.com/momentohq/functions/compare/v0.19.0...v0.20.0) (2026-04-16)


### Features

* add v2 support for topics ([#75](https://github.com/momentohq/functions/issues/75)) ([a94d867](https://github.com/momentohq/functions/commit/a94d8679569c56f3ac3476e4a109ca998bc64c51))

## [0.19.0](https://github.com/momentohq/functions/compare/v0.18.0...v0.19.0) (2026-04-15)


### Features

* add v1 list fetch ([#72](https://github.com/momentohq/functions/issues/72)) ([8f41825](https://github.com/momentohq/functions/commit/8f41825be2b48be33fb534b9498625a8cce4d4ab))

## [0.18.0](https://github.com/momentohq/functions/compare/v0.17.0...v0.18.0) (2026-04-08)


### Features

* add v2 support for aws-ddb ([#68](https://github.com/momentohq/functions/issues/68)) ([a1e0ede](https://github.com/momentohq/functions/commit/a1e0edebbe75ab9eff7d245fdb872490ece57122))

## [0.17.0](https://github.com/momentohq/functions/compare/v0.16.1...v0.17.0) (2026-04-08)


### Features

* add support for cache-list in v2, complete v2 scalar support. fix our doc tests and add a pr check to ensure we catch more of these failures. ([#67](https://github.com/momentohq/functions/issues/67)) ([ba7c833](https://github.com/momentohq/functions/commit/ba7c833ae536f58754c6fdf0be29385b8e11e23d))
* add v2 support for token ([#69](https://github.com/momentohq/functions/issues/69)) ([b7a0349](https://github.com/momentohq/functions/commit/b7a034974b26de50b184b8e3c672ecee6af87c64))

## [0.16.1](https://github.com/momentohq/functions/compare/v0.16.0...v0.16.1) (2026-04-06)


### Bug Fixes

* surgically duplicate wit dependencies ([#65](https://github.com/momentohq/functions/issues/65)) ([a07ec79](https://github.com/momentohq/functions/commit/a07ec79b452262cdee7cdd65475df5ac04cf630a))

## [0.16.0](https://github.com/momentohq/functions/compare/v0.15.1...v0.16.0) (2026-04-06)


### Features

* remove duplicated wit directories. add new v2 crates: scalar cache, aws auth + s3, spawn ([#53](https://github.com/momentohq/functions/issues/53)) ([a885e43](https://github.com/momentohq/functions/commit/a885e43bbe011fc5c83aa630f761824da37d31c6))

## [0.15.1](https://github.com/momentohq/functions/compare/v0.15.0...v0.15.1) (2026-03-23)


### Miscellaneous

* update s3 examples to use better auth ([#62](https://github.com/momentohq/functions/issues/62)) ([53bbd8c](https://github.com/momentohq/functions/commit/53bbd8c6937ad5b530af0f8ddc7553836395e3dd))

## [0.15.0](https://github.com/momentohq/functions/compare/v0.14.1...v0.15.0) (2026-03-19)


### Features

* add v1 list push front and back ([#54](https://github.com/momentohq/functions/issues/54)) ([9503b3b](https://github.com/momentohq/functions/commit/9503b3bde1a56ceedfe56d1b0840ed14565dddeb))
* use release-please to auto publish new releases for functions ([#52](https://github.com/momentohq/functions/issues/52)) ([983067f](https://github.com/momentohq/functions/commit/983067ffd029fa2acfe0cc39bf4e3b16d647d87a))


### Bug Fixes

* Specify versions in submodules for release please ([#55](https://github.com/momentohq/functions/issues/55)) ([31b2f81](https://github.com/momentohq/functions/commit/31b2f8116f3bc560942e29528b80812de97507df))
* Use simple release please because of poor rust workspace support ([#56](https://github.com/momentohq/functions/issues/56)) ([8287758](https://github.com/momentohq/functions/commit/8287758e3c09c28055a86bd5c2174a0115408ff4))
