//!
//! 一个对线程数量进行限制的解决方法
//!

use std::sync::{Arc, Mutex, Condvar};
use std::{thread, panic::{self, UnwindSafe}};

///
/// 线程限制结构体
///
/// 用于声明 **最大线程数量（max_threads）**
///
/// **Example:**
/// ```
/// mod thread_limit;
/// use thread_limit::ThreadLimit;
/// ```
///
pub struct ThreadLimit {
    max_threads: usize,
    condvar: Arc<(Mutex<usize>, Condvar)>,
}

impl ThreadLimit {

    ///
    /// 创建一个新的 `线程限制` 实例
    ///
    /// - 返回 `ThreadLimit` 结构体
    ///
    /// **Example:**
    /// ```
    /// mod thread_limit;
    /// use thread_limit::ThreadLimit;
    ///
    /// let thread = ThreadLimit::new(4);
    /// ```
    ///
    pub fn new(max_threads: usize) -> Self {
        Self {
            max_threads,
            condvar: Arc::new((Mutex::new(0), Condvar::new())),
        }
    }

    ///
    /// 在所给定的线程数量之内执行任务
    ///
    /// **Example:**
    /// ```
    /// mod thread_limit;
    /// use thread_limit::ThreadLimit;
    ///
    /// let thread = ThreadLimit::new(4);
    ///
    /// thread.execute(move || f(&mut x));
    /// ```
    ///
    /// `f` - 要执行的任务闭包，必须满足 FnOnce() + Send + 'static + UnwindSafe 特征
    ///
    /// 请处理好函数 `f` 的错误，以免影响线程的进行；
    ///
    /// 若函数 `f` 执行中出现无法恢复的错误，也不会影响线程的回收，保证服务可用。
    ///
    pub fn execute<F: FnOnce() + Send + 'static + UnwindSafe>(&self, f: F) {
        let (lock, cvar) = &*self.condvar;
        let mut count = lock.lock().expect("Failed to acquire mutex lock");

        while *count >= self.max_threads {
            count = cvar.wait(count).expect("Failed to wait on condition variable");
        };

        *count += 1;
        drop(count);

        let condvar_clone = Arc::clone(&self.condvar);

        thread::spawn(move || {

            if let Err(_) = panic::catch_unwind(|| f()) {};

            let (lock, cvar) = &*condvar_clone;
            let mut count = lock.lock().expect("Failed to acquire mutex lock");
            *count -= 1;
            cvar.notify_one();

        });

    }
}
