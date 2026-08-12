use super::*;
use std::sync::{LazyLock, Mutex, MutexGuard};

static TASKS: LazyLock<Mutex<Vec<Task>>> = LazyLock::new(|| {
    Mutex::new(vec![Task {
        id: 1,
        title: "Build the smallest useful compiler".into(),
        done: false,
    }])
});

fn tasks() -> Result<MutexGuard<'static, Vec<Task>>, AppError> {
    TASKS.lock().map_err(|_| AppError {
        message: "Task storage is unavailable.".into(),
    })
}

pub async fn list_tasks() -> Result<Vec<Task>, AppError> {
    Ok(tasks()?.clone())
}

pub async fn create_task(title: String) -> Result<Vec<Task>, AppError> {
    let title = title.trim();
    if title.is_empty() {
        return Err(AppError {
            message: "A task needs a title.".into(),
        });
    }

    let mut tasks = tasks()?;
    let id = tasks.iter().map(|task| task.id).max().unwrap_or(0) + 1;
    tasks.push(Task {
        id,
        title: title.into(),
        done: false,
    });
    Ok(tasks.clone())
}

pub async fn set_task_done(id: i64, done: bool) -> Result<Vec<Task>, AppError> {
    let mut tasks = tasks()?;
    let Some(task) = tasks.iter_mut().find(|task| task.id == id) else {
        return Err(AppError {
            message: "That task no longer exists.".into(),
        });
    };

    task.done = done;
    Ok(tasks.clone())
}
