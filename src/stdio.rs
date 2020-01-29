use std::{
    io::{self, stdin, stdout},
    thread, time::Duration,
};

use crossbeam_channel::{bounded, Receiver, Sender};

use crate::Message;

/// Creates an LSP connection via stdio.
pub(crate) fn stdio_transport() -> (Sender<Message>, Receiver<Message>, IoThreads) {
    let (writer_sender, writer_receiver) = bounded::<Message>(0);
    let writer = thread::spawn(move || {
        let stdout = stdout();
        let mut stdout = stdout.lock();
        writer_receiver.into_iter().try_for_each(|it| it.write(&mut stdout))?;
        Ok(())
    });
    let (reader_sender, reader_receiver) = bounded::<Message>(0);
    let reader = thread::spawn(move || {
        let stdin = stdin();
        let mut stdin = stdin.lock();
        loop {
            let msg = match Message::read(&mut stdin) {
                Ok(msg) => msg,
                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(10));
                    continue;
                },
                Err(e) => return Err(e),
            };

            if let Some(msg) = msg {
                let is_exit = match &msg {
                    Message::Notification(n) => n.is_exit(),
                    _ => false,
                };

                reader_sender.send(msg).unwrap();

                if is_exit {
                    break;
                }
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
