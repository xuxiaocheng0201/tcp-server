# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

* 添加默认版本判断实现。

### Changed

* 更新依赖版本。
* 简化配置addr部分。

## [0.0.2] - 2023-12-28

### Added

* 使用宏创建Handler。

### Changed

* 拆分逻辑到network.rs中。
* recv函数内记录超时，简化返回值处理

### Fixed

* 支持自定义Identifier。

## [0.0.1] - 2023-12-28

### Added

* 实现功能路由。
* 支持ctrl-c优雅关闭。
* 支持序列化配置。
