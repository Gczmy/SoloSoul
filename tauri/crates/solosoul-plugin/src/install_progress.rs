//! 安装进度按实际阶段推进；下载占 10%–90%，完成落盘后才报告 100%。
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PluginInstallPhase {
    Preparing,
    Downloading,
    Verifying,
    Installing,
    Finalizing,
    Completed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PluginInstallProgress {
    pub percent: u8,
    pub phase: PluginInstallPhase,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

impl PluginInstallProgress {
    pub fn completed() -> Self {
        Self {
            percent: 100,
            phase: PluginInstallPhase::Completed,
            downloaded_bytes: 0,
            total_bytes: None,
        }
    }
}

pub(crate) struct InstallProgressReporter<'a> {
    callback: &'a (dyn Fn(PluginInstallProgress) + Send + Sync),
    last: PluginInstallProgress,
}

impl<'a> InstallProgressReporter<'a> {
    pub(crate) fn new(callback: &'a (dyn Fn(PluginInstallProgress) + Send + Sync)) -> Self {
        let last = PluginInstallProgress {
            percent: 0,
            phase: PluginInstallPhase::Preparing,
            downloaded_bytes: 0,
            total_bytes: None,
        };
        callback(last.clone());
        Self { callback, last }
    }

    pub(crate) fn phase(&mut self, phase: PluginInstallPhase) {
        let percent = match phase {
            PluginInstallPhase::Preparing => 0,
            PluginInstallPhase::Downloading => 10,
            PluginInstallPhase::Verifying => 90,
            PluginInstallPhase::Installing => 95,
            PluginInstallPhase::Finalizing => 98,
            PluginInstallPhase::Completed => 100,
        };
        self.last.percent = self.last.percent.max(percent);
        self.last.phase = phase;
        (self.callback)(self.last.clone());
    }

    pub(crate) fn download(&mut self, downloaded: u64, total: Option<u64>) {
        let total = total.filter(|value| *value > 0);
        let portion = total
            .map(|total| (downloaded.min(total) as u128 * 80 / total as u128) as u8)
            .unwrap_or(0);
        let next = PluginInstallProgress {
            // 远程失败后回退本地包时不倒退；失败的传输也不能提前占用校验/写入进度。
            percent: self.last.percent.max(10 + portion),
            phase: PluginInstallPhase::Downloading,
            downloaded_bytes: downloaded,
            total_bytes: total,
        };
        // 百分比变化立即回传；未知长度时按字节节流，避免每个网络块触发 IPC。
        if next.percent != self.last.percent
            || next.phase != self.last.phase
            || next.total_bytes != self.last.total_bytes
            || downloaded.abs_diff(self.last.downloaded_bytes) >= 64 * 1024
        {
            self.last = next;
            (self.callback)(self.last.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn progress_uses_bytes_and_reserves_completion_for_installation() {
        let events = Mutex::new(Vec::new());
        let emit = |progress| events.lock().unwrap().push(progress);
        let mut progress = InstallProgressReporter::new(&emit);
        progress.download(0, Some(1000));
        progress.download(500, Some(1000));
        progress.download(1000, Some(1000));
        progress.phase(PluginInstallPhase::Verifying);
        progress.phase(PluginInstallPhase::Installing);
        assert!(events.lock().unwrap().iter().all(|p| p.percent < 100));
        progress.phase(PluginInstallPhase::Completed);
        let percentages: Vec<_> = events.lock().unwrap().iter().map(|p| p.percent).collect();
        assert_eq!(percentages, [0, 10, 50, 90, 90, 95, 100]);
    }

    #[test]
    fn unknown_length_does_not_fake_progress_and_fallback_never_regresses() {
        let events = Mutex::new(Vec::new());
        let emit = |progress| events.lock().unwrap().push(progress);
        let mut progress = InstallProgressReporter::new(&emit);
        progress.download(100_000, None);
        progress.download(200_000, Some(0));
        assert_eq!(events.lock().unwrap().last().unwrap().percent, 10);
        progress.download(500, Some(1000));
        progress.download(0, None);
        progress.download(2000, Some(1000));
        let percentages: Vec<_> = events.lock().unwrap().iter().map(|p| p.percent).collect();
        assert!(percentages.windows(2).all(|p| p[0] <= p[1]));
        assert_eq!(percentages.last(), Some(&90));
    }
}
