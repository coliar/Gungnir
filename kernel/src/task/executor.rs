#[allow(unused_imports)]
use crate::{c_api::enable_irq, println};

use super::{Task, TaskId};
use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use core::{task::{Context, Poll, Waker}, future::Future};
use crossbeam_queue::ArrayQueue;
use spin::Mutex;

use crate::{debug, log};


struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }
    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}

pub(crate) struct Executor {
    tasks: Mutex<BTreeMap<TaskId, Task>>,
    tmp_task: Mutex<BTreeMap<TaskId, Task>>,
    task_queue: Arc<ArrayQueue<TaskId>>,
    waker_cache: Mutex<BTreeMap<TaskId, Waker>>,
}

impl Executor {
    pub(crate) fn new() -> Self {
        Executor {
            tasks: Mutex::new(BTreeMap::new()),
            tmp_task: Mutex::new(BTreeMap::new()),
            task_queue: Arc::new(ArrayQueue::new(100)),
            waker_cache: Mutex::new(BTreeMap::new()),
        }
    }

    pub(crate) fn spawn<F>(&self, task: F)
    where 
        F: Future<Output = ()> + 'static,
    {
        let task = Task::new(task);
        let task_id = task.id;
        if self.tmp_task.lock().insert(task_id, task).is_some() {
            panic!("task with same ID already in tasks");
        }
    }

    fn run_ready_tasks(&self) {
        let Self {tasks, task_queue, waker_cache, tmp_task} = self;

        while let Some((task_id, task)) = tmp_task.lock().pop_first() {
            task_queue.push(task_id).expect("task queue full");
            if tasks.lock().insert(task_id, task).is_some() {
                panic!("task with same ID already in tasks");
            }
        }
        
        while let Some(task_id) = task_queue.pop() {
            let mut tasks_guard = tasks.lock();
            let task = match tasks_guard.get_mut(&task_id) {
                Some(task) => task,
                None => continue,
            };

            let mut waker_cache_guard = waker_cache.lock();
            let waker = waker_cache_guard
                                    .entry(task_id)
                                    .or_insert_with(|| {
                                        TaskWaker::new(task_id, task_queue.clone())
                                    });
            let mut context = Context::from_waker(waker);

            match task.poll(&mut context) {
                Poll::Ready(_result) => {
                    debug!("[-Executor-]: {:?} Completed", task_id);
                    tasks_guard.remove(&task_id);
                    waker_cache_guard.remove(&task_id);
                }
                Poll::Pending => {}
            }
        }
    }

    fn do_idle(&self) {
        unsafe {
            enable_irq();
        }
    }

    pub(crate) fn run(&self) -> ! {
        loop {
            self.run_ready_tasks();
            self.do_idle();
        }
    }
}