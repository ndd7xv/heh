use std::{
    error::Error,
    ops::{Deref, DerefMut},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};

use memmap2::{MmapMut, MmapOptions};

const SYNC_BUFF_LEN: usize = 0x10000;

/// Messages that the background thread processes to modify the buffer outside
/// of the main rendering thread.
enum EditMessage {
    Remove,
    Add(u8), // The byte to add to the front of the buffer that got cut off synchronously
    ModifyWindow(usize), // New offset to sync the window to
}

/// Struct to encapsulate a memory mapped buffer. Memmap is unsafe due to the fact
/// that it is backed by a file that could be removed. To make it safer, the file
/// can be locked. This struct also implements deref to much more easily control
/// the content the rest of the application can see without massively restructuring.
pub(crate) struct AsyncBuffer {
    /// The mmap backed by the file that is being edited
    content_buf: MmapMut,
    /// The length of the content. Used for when elements are deleted
    len: usize,
    /// A mpsc channel that allows sending messages to a thread that finishes
    /// updating the buffer if it is very large. Makes it much more responsive
    tx: crossbeam::channel::Sender<EditMessage>,
    /// An atomic that denotes whether the background buffer is actively engaged in work
    has_work: Arc<AtomicBool>,
    /// An offset shared between the processing thread and the main thread. This is to safely
    /// work on the ultimately same buffer by splitting it into 2 independent slices
    window_end: Arc<AtomicUsize>,
}

impl Deref for AsyncBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.content_buf[..self.len]
    }
}

impl DerefMut for AsyncBuffer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.content_buf[..self.len]
    }
}

impl AsyncBuffer {
    /// Create 2 copy-on-write memmaps of the same file. Since they are shared,
    /// they edit the same underlying buffer. Store one of the buffers for use
    /// for background processing by [`AsyncBuffer::process_messages`]
    pub fn new(file: &std::fs::File) -> Result<Self, Box<dyn Error>> {
        let mut content_buf = unsafe { MmapOptions::new().map_copy(file)? };
        let internal_buf = content_buf.as_mut_ptr();

        let has_work = Arc::new(AtomicBool::new(false));

        // This is ok, because it is the len of a memmap buffer, it is limited
        // by the size of the addressing space anyways.
        #[allow(clippy::cast_possible_truncation)]
        let window_end =
            Arc::new(AtomicUsize::new(SYNC_BUFF_LEN.min(file.metadata()?.len() as usize)));

        let (tx, rx) = crossbeam::channel::unbounded();

        AsyncBuffer::process_messages(
            #[allow(clippy::cast_possible_truncation)]
            (internal_buf, file.metadata()?.len() as usize),
            rx,
            has_work.clone(),
            window_end.clone(),
        );

        #[allow(clippy::cast_possible_truncation)]
        Ok(Self { content_buf, len: file.metadata()?.len() as usize, tx, has_work, window_end })
    }

    /// Receives messages of type [`EditMessage`], and processes the buffer in the
    /// background. Although this uses unsafe in both the ptr copy and the 2 mutable
    /// buffers, it is still safe. This uses a channel to receive the messages in order.
    /// Once received, it does the copy to insert / remove where the main thread stopped.
    /// This *vastly* improves snappiness and feel and does not overlap in read / write
    /// with the main thread, thus preventing any UB in writing to the same section of
    /// the buffer
    fn process_messages(
        internal_buf: (*mut u8, usize),
        rx: crossbeam::channel::Receiver<EditMessage>,
        has_work: Arc<AtomicBool>,
        window_offset: Arc<AtomicUsize>,
    ) {
        let internal_buf =
            unsafe { std::slice::from_raw_parts_mut(internal_buf.0, internal_buf.1) };
        let mut internal_start = window_offset.load(Ordering::SeqCst);

        std::thread::spawn(move || loop {
            for rcv in &rx {
                has_work.store(true, Ordering::SeqCst);

                let start = window_offset.load(Ordering::SeqCst);
                let internal_buf = &mut internal_buf[start..];

                match rcv {
                    EditMessage::Remove => unsafe {
                        debug_assert!(internal_start >= start);

                        std::ptr::copy(
                            internal_buf.as_ptr().add(internal_start - start),
                            internal_buf.as_mut_ptr(),
                            internal_buf.len() - (internal_start - start),
                        );

                        internal_start = start;
                    },
                    EditMessage::Add(byte) => unsafe {
                        debug_assert_eq!(internal_start, window_offset.load(Ordering::SeqCst));

                        std::ptr::copy(
                            internal_buf.as_ptr(),
                            internal_buf.as_mut_ptr().add(1),
                            internal_buf.len() - 1,
                        );

                        internal_buf[0] = byte;
                    },
                    EditMessage::ModifyWindow(new_window) => {
                        window_offset.store(new_window, Ordering::SeqCst);
                        internal_start = new_window;
                    }
                }

                has_work.store(rx.is_full(), Ordering::SeqCst);
            }
        });
    }

    /// Returns the length accounting for deletes
    pub fn len(&self) -> usize {
        self.len
    }

    /// Removes the value, and then copies the rest of the buffer 1 previous
    /// up to the offset of the internal window. After this, it has the background
    /// thread process the rest.
    pub fn remove(&mut self, offset: usize) -> u8 {
        let val = self.content_buf[offset];

        unsafe {
            std::ptr::copy(
                self.content_buf.as_ptr().add(offset + 1),
                self.content_buf.as_mut_ptr().add(offset),
                self.window_end.fetch_sub(1, Ordering::SeqCst) - offset,
            );
        }

        self.tx.send(EditMessage::Remove).unwrap();
        self.len -= 1;

        val
    }

    /// At the moment, only used for undoing deletions. With that in mind,
    /// no need to worry about increasing the size of the buffer. Copies
    /// up to the window so a single byte will be cut off at the end. Sends
    /// this byte so the background thread can re-insert it once it is safe.
    pub fn insert(&mut self, offset: usize, byte: u8) {
        let window_end = self.window_end.load(Ordering::SeqCst);
        self.tx.send(EditMessage::Add(self.content_buf[window_end - 1])).unwrap();
        self.len += 1;

        unsafe {
            std::ptr::copy(
                self.content_buf.as_ptr().add(offset),
                self.content_buf.as_mut_ptr().add(offset + 1),
                window_end.saturating_sub(offset).saturating_sub(1),
            );
        }

        self.content_buf[offset] = byte;
    }

    /// Compute whether the window needs to be extended, blocks if so until there is no
    /// more work to prevent data from being inserter / removed in the wrong places.
    pub fn compute_new_window(&mut self, new_offset: usize) {
        let window_end = self.window_end.load(Ordering::SeqCst);
        // If the distance of the current offset to the end of the window is less than a
        // third of the SYNC_BUFF_LEN then increase the window.
        // OR
        // If the new offset is sufficiently far away from the end of the window, shrink the
        // window. We want to do this because if we never shrink the window, then editing at
        // the end of the file creates a large buffer that will have to sync on deletions,
        // thus blocking the user and defeating the point of all this.
        //
        // This is set behind this if to prevent rerunning every single frame and cause a
        // potential performance hit.
        if window_end.saturating_sub(new_offset) < SYNC_BUFF_LEN / 3
            || window_end.saturating_sub(new_offset) > SYNC_BUFF_LEN * 4 / 3
        {
            self.block();
            self.tx
                .send(EditMessage::ModifyWindow((new_offset + SYNC_BUFF_LEN).min(self.len)))
                .unwrap();
        }
    }

    /// Wait until the background thread has finished processing messages
    pub fn block(&self) {
        while self.has_work.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
    }
}
