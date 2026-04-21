# 后端已完成阶段

> 本文档记录已经交付的后端阶段与已落地的基础能力。
>
> 未完成或未来阶段参见 [BACKEND_PENDING_PHASES.md](BACKEND_PENDING_PHASES.md)。
>
> 共享类型、命令、事件和状态机规则以 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md) 为唯一事实来源。

## 当前总览

- **Phase 1–7 已完成**
- **Phase 8 基线已完成**
- 当前待补齐内容为 **Phase 8 剩余增强项**，以及后续 **Phase 9–11**

## 已完成阶段

### Phase 1：下载任务领域模型

- DownloadService 与下载任务领域模型
- 单曲任务化
- 基础 commands / events

### Phase 2：整专下载与进度联动

- 整专下载
- 专辑封面落盘
- 下载进度事件推送
- 前端总进度展示
- 专辑页批量下载入口
- 重复创建保护

### Phase 3：任务控制与错误建模

- 任务取消
- 任务重试
- 历史清理
- 结构化错误码与详情
- 独立下载面板 UI

### Phase 4：系统通知集成

- 下载完成通知
- 播放切换通知
- 通知权限检查
- 测试通知

### Phase 5：批量选择管理 UI

- 全选
- 清空
- 反选按钮

### Phase 6：流水线下载优化

- download / write 两阶段流水线
- 整专下载吞吐提升

### Phase 7：统一偏好系统

- `AppPreferences` 统一偏好模型
- `preferences.toml` 持久化
- 导入 / 导出偏好
- 通知偏好收敛到统一偏好系统

### Phase 8 基线：本地已下载盘点与下载标记首版

- active `outputDir` 扫描
- `SongEntry` / `SongDetail.download` enrich
- 盘点快照 / 重扫 / 取消命令
- 盘点状态 / 进度事件
- `outputDir` 变化后自动重扫

## 已落地基础能力补充

- 统一日志中心
- session / persistent 双层日志
- 运行时错误安全事件
- 设置页日志 viewer
