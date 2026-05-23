## 15. 高级安全机制

### 15.1 JIT 即时解密（插件场景）

针对 SlotGo 等预约类插件，敏感数据（如护照号）不应在排队期间常驻内存：

```rust
pub enum PluginTaskState {
    /// 排队期间：Plugin 仅持有 Task ID，不持有任何明文
    Queued {
        task_id: String,
    },
    /// 即将提交：触发 JIT 解密，数据进入 Wasm Store（受 TTL 保护）
    Ready {
        task_id: String,
        decrypted_data: Zeroizing<Vec<u8>>,
        expires_at: Instant,
    },
    Submitted,
}

impl PluginTaskState {
    fn transition_to_ready(&mut self, vault: &Vault) -> Result<(), Error> {
        match self {
            PluginTaskState::Queued { task_id } => {
                let data = vault.get_and_decrypt(task_id)?;
                *self = PluginTaskState::Ready {
                    task_id: task_id.clone(),
                    decrypted_data: Zeroizing::new(data),
                    expires_at: Instant::now() + Duration::from_secs(30),
                };
                Ok(())
            }
            PluginTaskState::Ready { .. } => Ok(()), // 已经是 Ready，幂等
            PluginTaskState::Submitted => Err(Error::AlreadySubmitted),
        }
    }
}
```

### 15.2 熔断机制（Circuit Breaker）

```rust
/// 按插件隔离的熔断器，区分永久性失败和临时性失败
pub struct CircuitBreaker {
    /// plugin_id -> (permanent_failures, temporary_failures, last_failure)
    counters: Mutex<HashMap<String, (u64, u64, Option<Instant>)>>,
    permanent_threshold: u64,
    temporary_threshold: u64,
    cooldown_secs: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum FailureType {
    /// 永久性失败：SHA-256 校验失败、manifest 解析失败、Wasm 编译失败
    /// 触发条件：立即熔断，不再重试
    Permanent,
    /// 临时性失败：网络超时、DNS 解析失败、CDN 不可用
    /// 触发条件：连续 N 次后熔断，冷却后可恢复
    Temporary,
}

impl CircuitBreaker {
    pub fn new() -> Self {
        Self {
            counters: Mutex::new(HashMap::new()),
            permanent_threshold: 1,   // 永久性失败 1 次即熔断
            temporary_threshold: 5,   // 临时性失败 5 次才熔断
            cooldown_secs: 300,       // 冷却 5 分钟
        }
    }

    pub fn record_failure(&self, plugin_id: &str, kind: FailureType) {
        let mut counters = self.counters.lock().unwrap();
        let entry = counters.entry(plugin_id.to_string()).or_insert((0, 0, None));
        entry.2 = Some(Instant::now());
        match kind {
            FailureType::Permanent => entry.0 += 1,
            FailureType::Temporary => entry.1 += 1,
        }
    }

    pub fn is_open(&self, plugin_id: &str) -> bool {
        let counters = self.counters.lock().unwrap();
        let Some(&(perm, temp, last)) = counters.get(plugin_id) else {
            return false;
        };
        // 永久性失败立即熔断
        if perm >= self.permanent_threshold {
            return true;
        }
        // 临时性失败超过阈值且在冷却期内
        if temp >= self.temporary_threshold {
            if let Some(t) = last {
                return t.elapsed() < Duration::from_secs(self.cooldown_secs);
            }
        }
        false
    }

    pub fn reset(&self, plugin_id: &str) {
        let mut counters = self.counters.lock().unwrap();
        counters.remove(plugin_id);
    }
}
```
