// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use clone::Clone;
use kinds::Send;
use ops::Drop;
use option::{Option,Some,None};
use sync::arc::UnsafeArc;
use unstable::mutex::Mutex;

pub struct LittleLock {
    priv l: Mutex,
}

pub struct LittleGuard<'a> {
    priv l: &'a mut Mutex,
}

impl Drop for LittleLock {
    fn drop(&mut self) {
        unsafe { self.l.destroy(); }
    }
}

#[unsafe_destructor]
impl<'a> Drop for LittleGuard<'a> {
    fn drop(&mut self) {
        unsafe { self.l.unlock(); }
    }
}

impl LittleLock {
    pub fn new() -> LittleLock {
        unsafe { LittleLock { l: Mutex::new() } }
    }

    pub unsafe fn lock<'a>(&'a mut self) -> LittleGuard<'a> {
        self.l.lock();
        LittleGuard { l: &mut self.l }
    }

    pub unsafe fn try_lock<'a>(&'a mut self) -> Option<LittleGuard<'a>> {
        if self.l.trylock() {
            Some(LittleGuard { l: &mut self.l })
        } else {
            None
        }
    }

    pub unsafe fn signal(&mut self) {
        self.l.signal();
    }
}

impl<'a> LittleGuard<'a> {
    pub unsafe fn wait(&mut self) {
        self.l.wait();
    }
}

struct ExData<T> {
    lock: LittleLock,
    failed: bool,
    data: T,
}

/**
 * An arc over mutable data that is protected by a lock. For library use only.
 *
 * # Safety note
 *
 * This uses a pthread mutex, not one that's aware of the userspace scheduler.
 * The user of an Exclusive must be careful not to invoke any functions that may
 * reschedule the task while holding the lock, or deadlock may result. If you
 * need to block or deschedule while accessing shared state, use extra::sync::RWArc.
 */
pub struct Exclusive<T> {
    priv x: UnsafeArc<ExData<T>>
}

impl<T:Send> Clone for Exclusive<T> {
    // Duplicate an Exclusive Arc, as std::arc::clone.
    fn clone(&self) -> Exclusive<T> {
        Exclusive { x: self.x.clone() }
    }
}

impl<T:Send> Exclusive<T> {
    pub fn new(user_data: T) -> Exclusive<T> {
        let data = ExData {
            lock: LittleLock::new(),
            failed: false,
            data: user_data
        };
        Exclusive {
            x: UnsafeArc::new(data)
        }
    }

    // Exactly like std::arc::MutexArc,access(), but with the LittleLock
    // instead of a proper mutex. Same reason for being unsafe.
    //
    // Currently, scheduling operations (i.e., descheduling, receiving on a pipe,
    // accessing the provided condition variable) are prohibited while inside
    // the Exclusive. Supporting that is a work in progress.
    #[inline]
    pub unsafe fn with<U>(&self, f: |x: &mut T| -> U) -> U {
        let rec = self.x.get();
        let _l = (*rec).lock.lock();
        if (*rec).failed {
            fail!("Poisoned Exclusive::new - another task failed inside!");
        }
        (*rec).failed = true;
        let result = f(&mut (*rec).data);
        (*rec).failed = false;
        result
    }

    #[inline]
    pub unsafe fn with_imm<U>(&self, f: |x: &T| -> U) -> U {
        self.with(|x| f(x))
    }

    #[inline]
    pub unsafe fn hold_and_signal(&self, f: |x: &mut T|) {
        let rec = self.x.get();
        let _l = (*rec).lock.lock();
        if (*rec).failed {
            fail!("Poisoned Exclusive::new - another task failed inside!");
        }
        (*rec).failed = true;
        f(&mut (*rec).data);
        (*rec).failed = false;
        (*rec).lock.signal();
    }

    #[inline]
    pub unsafe fn hold_and_wait(&self, f: |x: &T| -> bool) {
        let rec = self.x.get();
        let mut l = (*rec).lock.lock();
        if (*rec).failed {
            fail!("Poisoned Exclusive::new - another task failed inside!");
        }
        (*rec).failed = true;
        let result = f(&(*rec).data);
        (*rec).failed = false;
        if result {
            l.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use option::*;
    use prelude::*;
    use super::Exclusive;
    use task;

    #[test]
    fn exclusive_new_arc() {
        unsafe {
            let mut futures = ~[];

            let num_tasks = 10;
            let count = 10;

            let total = Exclusive::new(~0);

            for _ in range(0u, num_tasks) {
                let total = total.clone();
                let (port, chan) = Chan::new();
                futures.push(port);

                do task::spawn {
                    for _ in range(0u, count) {
                        total.with(|count| **count += 1);
                    }
                    chan.send(());
                }
            };

            for f in futures.mut_iter() { f.recv() }

            total.with(|total| assert!(**total == num_tasks * count));
        }
    }

    #[test] #[should_fail]
    fn exclusive_new_poison() {
        unsafe {
            // Tests that if one task fails inside of an Exclusive::new, subsequent
            // accesses will also fail.
            let x = Exclusive::new(1);
            let x2 = x.clone();
            do task::try || {
                x2.with(|one| assert_eq!(*one, 2))
            };
            x.with(|one| assert_eq!(*one, 1));
        }
    }
}
