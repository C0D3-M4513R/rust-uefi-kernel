use core::cell::UnsafeCell;
use core::ops::{Deref, DerefMut};
use core::sync::atomic::{AtomicBool, AtomicPtr, AtomicU8, Ordering};
pub struct Lock<T>{
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

impl<T> Lock<T>{
    pub const fn new(data:T)->Self{
        return Self{
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
        }
    }

    ///Tries to lock the data behind this lock.
    pub fn try_lock(&self)->Option<LockGuard<T>>{
        if self.locked.compare_exchange(
            false,
            true,
            core::sync::atomic::Ordering::SeqCst,
            core::sync::atomic::Ordering::Acquire
        ).is_ok(){
            return Some(LockGuard{lock: self});
        }
        return None;
    }

    ///Locks the data behind this lock.
    ///This function will block until the lock is acquired.
    ///Not running the drop function of the returned guard will result in this lock being stuck as permanently locked!
    pub fn lock(&self)->LockGuard<T>{
        loop{
            if let Some(guard) = self.try_lock(){
                return guard;
            }
        }
    }
    fn unlock(&self){
        self.locked.store(false,core::sync::atomic::Ordering::Release);
    }
}


pub struct LockGuard<'a,T>{
    lock: &'a Lock<T>,
}

impl<'a, T> Drop for LockGuard<'a, T> {
    fn drop(&mut self) {
        lock.unlock();
    }
}
impl<'a,T> Deref for LockGuard<'a,T>{
    type Target = T;
    fn deref(&self)->&Self::Target{
        return unsafe{&*self.lock.data.get()};
    }
}
impl<'a,T> DerefMut for LockGuard<'a,T>{
    fn deref_mut(&self)->&mut Self::Target{
        return unsafe{&mut *self.lock.data.get()};
    }
}

pub struct ReadRWLockGuard<'a,T>{
    lock: &'a RWLock<T>,
}

impl<'a, T> Drop for ReadRWLockGuard<'a, T> {
    fn drop(&mut self) {
        lock.unlock();
    }
}

impl<'a, T> Deref for ReadRWLockGuard<'a,T>{
    type Target = T;

    fn deref(&self)->&Self::Target{
        return unsafe{&*self.lock.data.get()};
    }
}

pub struct WriteRWLockGuard<'a,T>{
    lock: &'a RWLock<T>,
}

impl<'a, T> Drop for WriteRWLockGuard<'a, T> {
    fn drop(&mut self) {
        lock.unlock();
    }
}

impl<'a,T> Deref for WriteRWLockGuard<'a,T>{
    type Target = T;
    fn deref(&self)->&Self::Target{
        return unsafe{&*self.lock.data.get()};
    }
}
impl<'a,T> DerefMut for WriteRWLockGuard<'a,T>{
    fn deref_mut(&self)->&mut Self::Target{
        return unsafe{&mut *self.lock.data.get()};
    }
}


pub struct RWLock<T>{
    locked: AtomicU8,
    data: UnsafeCell<T>,
}

impl<T> RWLock<T>{
    pub const fn new(data:T)->Self{
        return Self{
            locked: AtomicU8::new(0),
            data: UnsafeCell::new(data),
        }
    }

    ///Tries to lock the data behind this lock.
    ///This function will return None if the lock is already locked in a mutable manner.
    pub fn try_read_lock(&self)->Option<ReadRWLockGuard<T>>{
        let locked = self.locked.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| {
            if x == 0xFE || x == 0xFF {
                return None;
            }
            return Some(x+1);
        });
        if locked.is_ok(){
            return Some(ReadLockGuard{lock: self});
        }
        return None;
    }

    ///Locks the data behind this lock.
    ///This function return None if the lock is already locked in a mutable manner.
    ///This function will block until the lock is acquired.
    ///Not running the drop function of the returned guard will result in this lock being stuck as permanently locked!
    pub fn read_lock(&self)->ReadRWLockGuard<T>{
        loop{
            if let Some(guard) = self.try_read_lock(){
                return guard;
            }
        }
    }

    ///Tries to lock the data behind this lock in a mutable manner.
    ///If Read locks are active, this function will return None.
    pub fn try_write_lock(&self)->Option<WriteRWLockGuard<T>>{
        let locked = self.locked.compare_exchange(0,0xFF, Ordering::SeqCst,Ordering::SeqCst);
        if locked.is_ok(){
            return Some(WriteRWLockGuard{lock: self});
        }
        return None;
    }

    ///Locks the data behind this lock in a mutable manner.
    ///This function will block until the lock is acquired.
    ///Not running the drop function of the returned guard will result in this lock being stuck as permanently locked!
    pub fn write_lock(&self)->WriteRWLockGuard<T>{
        loop{
            if let Some(guard) = self.try_write_lock(){
                return guard;
            }
        }
    }
    
    fn unlock_read(&self){
        self.locked.fetch_sub(1, Ordering::SeqCst);
    }
    fn unlock_write(&self){
        self.locked.store(0, Ordering::SeqCst);
    }
}