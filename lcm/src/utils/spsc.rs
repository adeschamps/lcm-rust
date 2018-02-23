use std::cell::Cell;
use std::sync::Arc;
use std::sync::atomic::{spin_loop_hint, AtomicUsize, Ordering};
use std::{mem, ptr};

/// Creates a new asynchronous channel, returning the sender/receiver halves.
///
/// No send or receive will block, but sending to a full channel will cause the
/// oldest message to be dropped. Having a sender that vastly outpaces the
/// consumer will result in poor performance on the receiver's half.
pub fn channel<T>(size: usize) -> (Sender<T>, Receiver<T>) {
    let backing = Arc::new(RingBuffer::new(size));
    (Sender::new(backing.clone()), Receiver::new(backing.clone()))
}

/// The receiving half of the channel.
pub struct Receiver<T> {
    /// The backing ringbuffer for the channel.
    inner: Arc<RingBuffer<T>>,
}
impl<T> Receiver<T> {
    /// Creates a new receiver with the backing ringbuffer.
    fn new(backing: Arc<RingBuffer<T>>) -> Receiver<T> {
        Receiver { inner: backing }
    }

    /// Returns the next item in the channel.
    ///
    /// This will be considered starving if it attempts to receive `capacity / 2`
    /// times and is interrupted each time. After that point, it will acquire an
    /// exclusive lock to the queue that will be held exactly long enough to
    /// read a pointer.
    ///
    /// This will only be an issue if the backing buffer is full and the Sender
    /// is vastly outpacing the Receiver.
    pub fn recv(&self) -> Option<T> {
        (*self.inner).pop()
    }

    pub fn capacity(&self) -> usize {
        (*self.inner).capacity
    }
}
unsafe impl<T: Send> Send for Receiver<T> {}
//impl<T> !Sync for Receiver<T> { }

/// The sending half of the channel.
pub struct Sender<T> {
    /// The backing ringbuffer for the channel.
    inner: Arc<RingBuffer<T>>,
}
impl<T> Sender<T> {
    /// Creates a new sender with the backing ringbuffer.
    fn new(backing: Arc<RingBuffer<T>>) -> Sender<T> {
        Sender { inner: backing }
    }

    /// Pushes an item into the channel.
    ///
    /// If the queue is full, this will remove the oldest item and replace it
    /// with the new one. This will not block, but it may slow down very slightly
    /// if the Receiver is being starved.
    ///
    /// The fact that this may replace the oldest item means that it may call
    /// drop on the object.
    pub fn send(&self, item: T) {
        (*self.inner).push(item);
    }

    pub fn capacity(&self) -> usize {
        (*self.inner).capacity
    }

    /// Returns true if the receiving end of the channel is closed.
    pub fn is_closed(&self) -> bool {
        Arc::strong_count(&self.inner) < 2
    }
}
unsafe impl<T: Send> Send for Sender<T> {}
//impl<T> !Sync for Sender<T> { }

#[cfg(target_pointer_width = "64")]
/// Calculate necessary cache line padding (assumes 64 byte line).
macro_rules! pad_amount { ($N:expr) => { 8 - $N } }

#[cfg(target_pointer_width = "32")]
/// Calculate necessary cache line padding (assumes 64 byte line).
macro_rules! pad_amount { ($N:expr) => { 16 - $N } }

/// Ring buffer which backs the SPSC queue.
// This should be packed, but it's easier to test the manual
// packing (see the test section at the bottom) than it is to
// deal with all the `unsafe` blocks (as of Rust 1.24).
#[repr(C)]
struct RingBuffer<T> {
    //-----------------
    // Const stuff
    //-----------------
    /// Pointer to the allocated memory.
    data: *mut T,
    /// Number of elements this buffer is able to store.
    capacity: usize,

    _padding0: [usize; pad_amount!(2)],

    //-----------------
    // Consumer Stuff
    //-----------------
    /// The current head location.
    head: AtomicUsize,
    /// The consumer's believed current tail location.
    shadow_tail: Cell<usize>,

    _padding1: [usize; pad_amount!(2)],

    //-----------------
    // Producer Stuff
    //-----------------
    /// Current tail location.
    tail: AtomicUsize,
    /// The producer's believed current head location.
    shadow_head: Cell<usize>,
    /// Lock used to give the consumer a chance to get a value.
    giveup_lock: AtomicUsize,

    _padding2: [usize; pad_amount!(3)],
}
impl<T> RingBuffer<T> {
    /// Creates a new ring buffer of the specified size.
    fn new(size: usize) -> RingBuffer<T> {
        assert!(size > 0, "size must be greater than zero");
        assert!(size as isize > 0, "size must be able to fit into an isize");

        let data = {
            let mut data: Vec<T> = Vec::with_capacity(size);
            let ptr = data.as_mut_ptr();
            mem::forget(data);

            ptr
        };

        RingBuffer {
            capacity: size,
            data,
            _padding0: [0; pad_amount!(2)],
            head: AtomicUsize::new(0),
            shadow_tail: Cell::new(0),
            _padding1: [0; pad_amount!(2)],
            tail: AtomicUsize::new(0),
            shadow_head: Cell::new(0),
            giveup_lock: AtomicUsize::new(0),
            _padding2: [0; pad_amount!(3)],
        }
    }

    /// Returns the next item in the queue.
    fn pop(&self) -> Option<T> {
        // There is a small potential for starvation and incorrect value here,
        // but I believe that they can be ignored. First, the reason for
        // ignoring the starvation issue:
        //
        // In order for this to starve, the queue has to be full and the
        // producer has to be significantly faster than the consumer. And, if
        // it is, this will give up after some point and acquire a lock.
        //
        // For the incorrect value:
        // To get an incorrect value, the producer will have to produce
        // *exactly* usize::MAX elements from the time the consumer loads the
        // head to the time the consumer does the CAS. For 32bit systems, this
        // is super unlikely. For 64bit systems, this is basically impossible.
        for _ in 0..(1 + self.capacity / 2) {
            // Get the current head.
            let head = self.head.load(Ordering::Acquire);

            // Check to see if we think we're full
            if head >= self.shadow_tail.get() {
                // Double check to see if we're really full
                self.shadow_tail.set(self.tail.load(Ordering::Acquire));

                debug_assert!(
                    head <= self.shadow_tail.get(),
                    "head is further than shadow tail"
                );

                if head == self.shadow_tail.get() {
                    // We are really, for real, full
                    return None;
                }
            }

            // We have at least one!
            let val = self.load(head);

            // Make sure that the data we loaded was actually valid and, if it was,
            // increment the head
            if self.head
                .compare_and_swap(head, head.wrapping_add(1), Ordering::Release)
                == head
            {
                return Some(val);
            }
        }

        // At this point, we give up and acquire the lock
        debug_assert_eq!(
            self.giveup_lock.load(Ordering::Relaxed),
            0,
            "recursive giveup"
        );
        self.giveup_lock.store(1, Ordering::Acquire);
        let val = self.pop();
        self.giveup_lock.store(0, Ordering::Release);
        assert!(val.is_some(), "gave up on an empty queue"); // Curious to see this ever happen

        val
    }

    /// Pushes an item onto the queue.
    ///
    /// If the queue is full, this will remove the oldest item and replace it
    /// with the new one. This will not block unless the consumer is being
    /// starved by the constant replacing of the first item in the queue, in
    /// which case this will block long enough for the consumer to retrieve a
    /// single item.
    fn push(&self, item: T) {
        // Load the current tail
        let tail = self.tail.load(Ordering::Relaxed);

        // Check to see if we're full
        if self.shadow_head.get().wrapping_add(self.capacity) <= tail {
            // Double check to see if we're really full
            self.shadow_head.set(self.head.load(Ordering::Acquire));
            if self.shadow_head.get().wrapping_add(self.capacity) <= tail {
                // We are for real full. Spin until the giveup lock is
                // released, which should be very fast. The giveup lock is
                // acquired, an item is popped, and then released - there is no
                // opportunity for the lock to be left locked.
                while self.giveup_lock.load(Ordering::Acquire) != 0 {
                    // On x86 this is the PAUSE instruction. I am not
                    // 100% sure this should be here.
                    spin_loop_hint();
                }

                // Try to move the head up one
                let head = self.shadow_head.get();
                let old_head =
                    self.head
                        .compare_and_swap(head, head.wrapping_add(1), Ordering::Release);

                if head != old_head {
                    // The consumer managed to pop at least one value
                    debug_assert!(old_head > head, "head decreased");
                    self.shadow_head.set(old_head);
                } else {
                    // We manually moved the head, so we know the limit is at least one more
                    self.shadow_head.set(head.wrapping_add(1));

                    // We also need to drop the old value before we overwrite it
                    let conv_offset = (head % self.capacity) as isize;
                    debug_assert!(conv_offset >= 0, "converted offset does not fit in usize");
                    unsafe {
                        ptr::drop_in_place(self.data.offset(conv_offset));
                    }
                }
            }
        }

        // We have room for at least one more
        self.store(tail, item);
        self.tail.store(tail.wrapping_add(1), Ordering::Release);
    }

    /// Stores an item into the buffer.
    #[inline]
    fn store(&self, offset: usize, item: T) {
        let conv_offset = (offset % self.capacity) as isize;
        debug_assert!(conv_offset >= 0, "converted offset does not fit in usize");
        unsafe {
            *self.data.offset(conv_offset) = item;
        }
    }

    /// Loads an item from the buffer
    #[inline]
    fn load(&self, offset: usize) -> T {
        let conv_offset = (offset % self.capacity) as isize;
        debug_assert!(conv_offset >= 0, "converted offset does not fit in usize");
        unsafe { ptr::read(self.data.offset(conv_offset)) }
    }
}
impl<T> Drop for RingBuffer<T> {
    fn drop(&mut self) {
        // There will only ever be a single thread with access to this object
        // during the drop. That means head and tail will not change during
        // this function.
        let mut head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Relaxed);

        debug_assert!(head <= tail, "head is larger than tail");

        while head != tail {
            let conv_offset = (head % self.capacity) as isize;
            debug_assert!(conv_offset >= 0, "converted offset does not fit in usize");

            unsafe { ptr::drop_in_place(self.data.offset(conv_offset)) };
            head = head.wrapping_add(1);
        }

        // Free the memory
        unsafe {
            let _: Vec<T> = Vec::from_raw_parts(self.data, 0, self.capacity);
        }
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn basic_in_out() {
        const LIMIT: usize = 3;
        let (p, c) = super::channel(LIMIT);

        for x in 0..LIMIT {
            p.send(x);
        }

        for x in 0..LIMIT {
            assert_eq!(c.recv(), Some(x));
        }

        assert_eq!(c.recv(), None);
    }

    #[test]
    fn overwriting() {
        const LIMIT: usize = 3;
        const OVERWRITE: usize = 2;
        let (p, c) = super::channel(LIMIT);

        for x in 0..LIMIT + OVERWRITE {
            p.send(x);
        }

        for x in (0..LIMIT + OVERWRITE).skip(OVERWRITE) {
            assert_eq!(c.recv(), Some(x));
        }

        assert_eq!(c.recv(), None);
    }

    #[test]
    fn hammer_time() {
        use std::thread;
        use std::sync::mpsc;
        const LIMIT: usize = 500;

        let (p, c) = super::channel(LIMIT);
        let (done_p, done_c) = mpsc::channel();

        thread::spawn(move || {
            for x in 1.. {
                p.send(x);

                if done_c.try_recv().is_ok() {
                    break;
                }
            }
        });

        let mut prev = 0;
        for _ in 0..5 * LIMIT {
            if let Some(v) = c.recv() {
                assert!(v > prev);
                prev = v;
            }
        }

        done_p.send(()).unwrap();
    }

    #[test]
    fn slow_producer() {
        use std::{thread, time};
        use std::sync::mpsc;
        const LIMIT: usize = 500;

        let (p, c) = super::channel(LIMIT);
        let (done_p, done_c) = mpsc::channel();

        thread::spawn(move || {
            for x in 1.. {
                p.send(x);

                if done_c.try_recv().is_ok() {
                    break;
                }

                thread::sleep(time::Duration::from_millis(100));
            }
        });

        let mut prev = 0;
        for _ in 0..5 * LIMIT {
            if let Some(v) = c.recv() {
                assert!(v > prev);
                prev = v;
            }
        }

        done_p.send(()).unwrap();
    }

    #[test]
    fn slow_consumer() {
        use std::{thread, time};
        use std::sync::mpsc;
        const LIMIT: usize = 50;

        let (p, c) = super::channel(LIMIT);
        let (done_p, done_c) = mpsc::channel();

        thread::spawn(move || {
            for x in 1.. {
                p.send(x);

                if done_c.try_recv().is_ok() {
                    break;
                }
            }
        });

        let mut prev = 0;
        for _ in 0..2 * LIMIT {
            if let Some(v) = c.recv() {
                assert!(v > prev);
                prev = v;
            }

            thread::sleep(time::Duration::from_millis(100));
        }

        done_p.send(()).unwrap();
    }

    #[test]
    fn padding() {
        // Before Rust 1.24, using `#[repr(C, packed)]` did not require blocks
        // of unsafe code. Now that it does, trying to add those blocks would
        // be both a pain and ugly, particularly since I am very confident that
        // I manually padded things correctly. This test confirms that there is
        // no padding in the struct so the absence of `packed` changes nothing.
        use super::*;
        use std::mem::size_of;

        let total_size = size_of::<*mut u32>() + size_of::<usize>() +          // data, capacity
                         size_of::<[usize; pad_amount!(2)]>() +                // _padding0
                         size_of::<AtomicUsize>() + size_of::<Cell<usize>>() + // head, shadow_tail
                         size_of::<[usize; pad_amount!(2)]>() +                // _padding1
                         size_of::<AtomicUsize>() + size_of::<Cell<usize>>() + // tail, shadow_head
                         size_of::<AtomicUsize>() +                            // giveup_lock
                         size_of::<[usize; pad_amount!(3)]>(); // _padding2
        assert_eq!(size_of::<RingBuffer<u32>>(), total_size);
    }
}
