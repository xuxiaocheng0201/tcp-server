# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2024-1-9

### Changed

* 修改Configuration名称，避免与其他crate中的同名类型混淆。

### Fixed

* 忽略正常关闭时的UnexpectedEof日志。
* 确认功能可用。

## [0.0.6] - 2024-1-9

### Added

* 添加测试

### Fixed

* 使用timeout，优化性能。

## [0.0.5] - 2024-1-5

### Added

* 添加连接测试

### Removed

* 移除默认版本判断实现，需子crate显式指定。

## [0.0.4] - 2024-1-4

### Added

* Handler参数增加version字段。
* 更新依赖版本。

## [0.0.3] - 2024-1-1

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
