//! Authenticated watch requests join PentaKuru's existing desktop UI agent.
use serde::Serialize;
use std::collections::VecDeque;

#[derive(Clone, Debug, Serialize)]
pub struct Task {
    pub id: String,
    pub goal: String,
    pub phase: String,
    pub message: String,
    #[serde(skip)]
    pub cancel_requested: bool,
}

#[derive(Clone, Default)]
pub struct Control {
    pub key: String,
    pub ready: bool,
    pub busy: bool,
    tasks: VecDeque<Task>,
    next: u64,
}

pub fn pairing_key() -> Result<String, String> {
    let mut bytes = [0u8; 16];
    getrandom::fill(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

impl Control {
    pub fn authorize(&self, key: &str) -> bool {
        !self.key.is_empty()
            && key.len() == self.key.len()
            && key
                .bytes()
                .zip(self.key.bytes())
                .fold(0u8, |difference, (a, b)| difference | (a ^ b))
                == 0
    }
    pub fn enqueue(&mut self, goal: &str) -> Result<Task, String> {
        let goal = goal.trim();
        if goal.is_empty() || goal.chars().count() > 4000 {
            return Err("Lệnh Windows phải có nội dung và tối đa 4000 ký tự".into());
        }
        if !self.ready {
            return Err(
                "Penta chưa sẵn sàng điều khiển Windows. Cấu hình AI trên PC rồi thử lại.".into(),
            );
        }
        if self.busy
            || self
                .tasks
                .iter()
                .any(|task| matches!(task.phase.as_str(), "pending" | "running"))
        {
            return Err("Penta đang thực hiện tác vụ khác. Chờ hoàn tất hoặc bấm Dừng.".into());
        }
        self.next += 1;
        let task = Task {
            id: format!(
                "watch-{}-{}-{}",
                std::process::id(),
                self.next,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            ),
            goal: goal.into(),
            phase: "pending".into(),
            message: "Đang chờ PentaKuru nhận lệnh".into(),
            cancel_requested: false,
        };
        self.tasks.push_back(task.clone());
        while self.tasks.len() > 16 {
            self.tasks.pop_front();
        }
        Ok(task)
    }
    pub fn get(&self, id: &str) -> Option<Task> {
        self.tasks.iter().find(|task| task.id == id).cloned()
    }
    pub fn take_pending(&mut self) -> Option<Task> {
        if self.busy || !self.ready {
            return None;
        }
        let task = self.tasks.iter_mut().find(|task| task.phase == "pending")?;
        task.phase = "running".into();
        task.message = "PentaKuru đang bắt đầu tác vụ Windows".into();
        Some(task.clone())
    }
    pub fn update(&mut self, id: &str, phase: &str, message: &str) {
        if let Some(task) = self.tasks.iter_mut().find(|task| task.id == id)
            && matches!(task.phase.as_str(), "pending" | "running")
        {
            task.phase = phase.into();
            task.message = message.into();
        }
    }
    pub fn cancel(&mut self, id: &str) -> Option<Task> {
        let task = self.tasks.iter_mut().find(|task| task.id == id)?;
        task.cancel_requested = true;
        if task.phase == "pending" {
            task.phase = "cancelled".into();
            task.message = "Đã dừng trước khi thực thi".into();
        }
        Some(task.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pairing_keys_are_private_and_tasks_are_single_flight_and_cancellable() {
        let key = pairing_key().unwrap();
        assert_eq!(key.len(), 32);
        assert_ne!(key, pairing_key().unwrap());
        let mut queue = Control {
            key,
            ready: true,
            ..Default::default()
        };
        assert!(!queue.authorize(""));
        assert!(!queue.authorize("wrong"));
        assert!(queue.authorize(&queue.key));
        let task = queue.enqueue("Mở Notepad").unwrap();
        assert!(queue.enqueue("Lệnh thứ hai").is_err());
        assert_eq!(queue.take_pending().unwrap().id, task.id);
        assert!(queue.take_pending().is_none());
        assert!(queue.cancel(&task.id).unwrap().cancel_requested);
        queue.update(&task.id, "cancelled", "Đã dừng");
        queue.update(&task.id, "completed", "Phản hồi đến muộn");
        assert_eq!(queue.get(&task.id).unwrap().phase, "cancelled");
        let next = queue.enqueue("Lệnh mới").unwrap();
        queue.cancel(&next.id);
        assert!(queue.take_pending().is_none());
    }
}
