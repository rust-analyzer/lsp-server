use std::{
    io::{self, stdin, stdout, ErrorKind},
    thread,
};

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::Message;

/// Creates an LSP connection via stdio.
pub(crate) fn stdio_transport() -> (Sender<Message>, Receiver<Message>, IoThreads) {
    let (writer_sender, writer_receiver) = bounded::<Message>(0);
    let writer = thread::spawn(move || {
        let stdout = stdout();

        let mut stdout = stdout.lock();
        let res = writer_receiver.into_iter().try_for_each(|it| it.write(&mut stdout));

        res.map_err(|err| {
            if let ErrorKind::WouldBlock = err.kind() {
                log::error!(
                    "stdout returned WouldBlock, is_non_blocking(stdout) = {:?}",
                    is_non_blocking(&stdout)
                )
            }
            err
        })
    });
    let (reader_sender, reader_receiver) = bounded::<Message>(0);
    let reader = thread::spawn(move || {
        let stdin = stdin();

        let mut stdin = stdin.lock();
        while let Some(msg) = Message::read(&mut stdin).map_err(|err| {
            if let ErrorKind::WouldBlock = err.kind() {
                log::error!(
                    "stdin returned WouldBlock, is_non_blocking(stdin) = {:?}",
                    is_non_blocking(&stdin)
                )
            }
            err
        })? {
            let is_exit = match &msg {
                Message::Notification(n) => n.is_exit(),
                _ => false,
            };

            reader_sender.send(msg).unwrap();

            if is_exit {
                break;
            }
        }
        Ok(())
    });
    let threads = IoThreads { reader, writer };
    (writer_sender, reader_receiver, threads)
}

pub struct IoThreads {
    reader: thread::JoinHandle<io::Result<()>>,
    writer: thread::JoinHandle<io::Result<()>>,
}

impl IoThreads {
    pub fn join(self) -> io::Result<()> {
        match self.reader.join() {
            Ok(r) => r?,
            Err(_err) => panic!("reader panicked"),
        }
        match self.writer.join() {
            Ok(r) => r,
            Err(_err) => panic!("writer panicked"),
        }
    }
}

#[cfg(unix)]
fn is_non_blocking(fd: &impl std::os::unix::io::AsRawFd) -> bool {
    let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFL, 0) };
    (flags & libc::O_NONBLOCK) == libc::O_NONBLOCK
}

#[cfg(not(unix))]
fn is_non_blocking<T>(_fd: &T) -> bool {
    false
}
